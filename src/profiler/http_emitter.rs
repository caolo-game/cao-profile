//! Uses Curl to perform http requests.
//!
//! Note that we require our http client to work during a panic, which the `reqwest` library does
//! not satisfy.
//!
use crate::Record;

use crate::error;
use anyhow::Context;
use curl::easy::{Easy, List};
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::io::Read;
use std::mem;
use std::sync::mpsc::{self, channel};
use std::sync::Mutex;

pub const BUFFER_SIZE: usize = 1 << 12;
pub const LOCAL_BUFFER_SIZE: usize = 1 << 9;

type Sender = mpsc::Sender<Vec<Record<'static>>>;

thread_local!(
    pub static LOCAL_COMM: RefCell<LocalHttpEmitter> = {
        let comm = COMM.lock().expect("Expected to be able to lock COMM");
        let sender = comm.get_sender();
        let buffer = Vec::with_capacity(LOCAL_BUFFER_SIZE);
        let res = LocalHttpEmitter { sender, buffer };
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
    static ref URL: String = {
        std::env::var("CAO_PROFILE_URI")
            .unwrap_or_else(|_| "http://localhost:6660/push-records".to_owned())
    };
    static ref COMM: Mutex<HttpEmitter> = {
        let (sender, receiver) = channel::<Vec<Record<'static>>>();
        let builder = std::thread::Builder::new().name("cao-profile http emitter".into());
        let mut container = Vec::with_capacity(BUFFER_SIZE);
        let worker = builder
            .spawn(move || loop {
                match receiver.recv().with_context(|| "Failed to receive data") {
                    Ok(records) => {
                        container.extend_from_slice(records.as_slice());
                        if container.len() >= BUFFER_SIZE {
                            let container =
                                mem::replace(&mut container, Vec::with_capacity(BUFFER_SIZE));
                            send(container.as_slice())
                                .map_err(|e| {
                                    error!(
                                        "Failed to send payload to HTTP endpoint ({}): {:?}",
                                        *URL, e
                                    );
                                })
                                .unwrap_or_default();
                        }
                    }
                    Err(err) => {
                        error!("Failed to read record {:?}", err);
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
    buffer: Vec<Record<'static>>,
}

impl LocalHttpEmitter {
    pub fn push(&mut self, r: Record<'static>) {
        self.buffer.push(r);
        if self.buffer.len() >= LOCAL_BUFFER_SIZE {
            let buffer = mem::replace(&mut self.buffer, Vec::with_capacity(LOCAL_BUFFER_SIZE));
            self.sender
                .send(buffer)
                .expect("Failed to send records for saving");
        }
    }
}

impl HttpEmitter {
    pub fn get_sender(&self) -> Sender {
        self.sender.clone()
    }
}

impl Drop for LocalHttpEmitter {
    fn drop(&mut self) {
        let v = mem::replace(&mut self.buffer, Vec::new());
        self.sender.send(v).unwrap();
    }
}
