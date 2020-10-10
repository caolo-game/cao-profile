//! Uses Curl to perform http requests.
//!
//! Note that we require our http client to work during a panic, which the `reqwest` library does
//! not satisfy.
//!
//!
//! Spawns two threads:
//! - An aggregator thread that collects records in a buffer
//! - And an emitter thread that will send the recorded data to a backend
//!
use crate::Record;

use crate::{error, trace};
use anyhow::Context;
use crossbeam_channel::bounded;
use curl::easy::{Easy, List};
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::io::Read;
use std::mem;
use std::sync::Mutex;
use std::time::Duration;

type Sender = crossbeam_channel::Sender<Record<'static>>;

thread_local!(
    pub static LOCAL_EMITTER: RefCell<LocalHttpEmitter> = {
        let comm = EMITTER.lock().expect("Expected to be able to lock EMITTER");
        let sender = comm.get_sender();
        let res = LocalHttpEmitter { sender };
        RefCell::new(res)
    };
);

fn send<'a>(rows: &[Record<'a>]) -> anyhow::Result<()> {
    let payload = serde_json::to_string(rows)?;
    let mut payload = payload.as_bytes();
    let mut easy = Easy::new();
    easy.url(URL.as_str())?;
    easy.post(true)?;
    let mut list = List::new();
    list.append("Content-type: application/json")?;
    easy.http_headers(list)?;
    easy.post_field_size(payload.len() as u64)?;
    let mut trans = easy.transfer();
    trans.read_function(|buf| Ok(payload.read(buf).unwrap_or(0)))?;
    trans.perform()?;
    Ok(())
}

lazy_static! {
    static ref BUFFER_SIZE: usize = std::env::var("CAO_PROFILE_BUFFER")
        .unwrap_or_else(|_| "2048".to_owned())
        .parse()
        .unwrap();
    static ref URL: String = {
        std::env::var("CAO_PROFILE_URI")
            .unwrap_or_else(|_| "http://localhost:6660/push-records".to_owned())
    };
    static ref EMITTER: Mutex<HttpEmitter> = {
        let buffer_size = *BUFFER_SIZE;
        let (payload_sender, payload_receiver) = bounded::<Vec<Record<'static>>>(buffer_size);
        let (sender, receiver) = bounded::<Record<'static>>(buffer_size);
        let builder = std::thread::Builder::new().name("cao-profile http aggregator".into());
        let mut container = Vec::with_capacity(buffer_size);
        let send_impl = move |container: Vec<Record<'static>>| {
            if let Err(err) = payload_sender.send(container) {
                error!("Failed to send payload to sender thread {:?}", err);
            }
        };
        let aggregator = builder
            .spawn(move || loop {
                match receiver
                    .recv_timeout(Duration::from_millis(12))
                    .with_context(|| "Failed to receive data")
                {
                    Ok(record) => {
                        container.push(record);
                        if container.len() >= buffer_size {
                            let container =
                                mem::replace(&mut container, Vec::with_capacity(buffer_size));
                            send_impl(container);
                        }
                    }
                    Err(err) => {
                        trace!("Failed to read record {:?}", err);
                        if !container.is_empty() {
                            let container =
                                mem::replace(&mut container, Vec::with_capacity(buffer_size));
                            send_impl(container);
                        }
                    }
                }
            })
            .unwrap();
        let builder = std::thread::Builder::new().name("cao-profile http emitter".into());
        let emitter = builder
            .spawn(move || loop {
                match payload_receiver
                    .recv()
                    .with_context(|| "Failed to receive data")
                {
                    Ok(container) => {
                        send(container.as_slice())
                            .map_err(|e| {
                                error!(
                                    "Failed to send payload to HTTP endpoint ({}): {:?}",
                                    *URL, e
                                );
                            })
                            .unwrap_or_default();
                    }
                    Err(err) => {
                        trace!("Failed to receive payload {:?}", err);
                        return;
                    }
                }
            })
            .unwrap();
        let res = HttpEmitter {
            sender,
            _emitter: emitter,
            _aggregator: aggregator,
        };
        Mutex::new(res)
    };
}

struct HttpEmitter {
    _aggregator: std::thread::JoinHandle<()>,
    _emitter: std::thread::JoinHandle<()>,
    sender: Sender,
}

pub struct LocalHttpEmitter {
    sender: Sender,
}

impl LocalHttpEmitter {
    pub fn push(&mut self, r: Record<'static>) {
        if let Err(err) = self.sender.send_timeout(r, Duration::from_micros(50)) {
            trace!("Failed to push record {:?}", err);
        }
    }
}

impl HttpEmitter {
    pub fn get_sender(&self) -> Sender {
        self.sender.clone()
    }
}
