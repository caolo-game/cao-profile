//! Uses Curl to perform http requests.
//!
//! Note that we require our http client to work during a panic, which the `reqwest` library does
//! not satisfy.
//!
use crate::Record;

use crate::{error, trace};
use anyhow::Context;
use curl::easy::{Easy, List};
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::io::Read;
use std::mem;
use std::sync::mpsc::{self, sync_channel};
use std::sync::Mutex;
use std::time::Duration;

type Sender = mpsc::SyncSender<Record<'static>>;

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
        let (sender, receiver) = sync_channel::<Record<'static>>(buffer_size);
        let builder = std::thread::Builder::new().name("cao-profile http emitter".into());
        let mut container = Vec::with_capacity(buffer_size);
        let send_impl = |container: Vec<Record>| {
            send(container.as_slice())
                .map_err(|e| {
                    error!(
                        "Failed to send payload to HTTP endpoint ({}): {:?}",
                        *URL, e
                    );
                })
                .unwrap_or_default();
        };
        let worker = builder
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
                        let container =
                            mem::replace(&mut container, Vec::with_capacity(buffer_size));
                        send_impl(container);
                    }
                }
            })
            .unwrap();
        let res = HttpEmitter {
            sender,
            _worker: worker,
        };
        Mutex::new(res)
    };
}

struct HttpEmitter {
    _worker: std::thread::JoinHandle<()>,
    sender: Sender,
}

pub struct LocalHttpEmitter {
    sender: Sender,
}

impl LocalHttpEmitter {
    pub fn push(&mut self, r: Record<'static>) {
        self.sender
            .send(r)
            .expect("Failed to send records for saving");
    }
}

impl HttpEmitter {
    pub fn get_sender(&self) -> Sender {
        self.sender.clone()
    }
}
