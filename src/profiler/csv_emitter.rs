use super::Record;

use std::cell::RefCell;
use std::fs::File;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;

fn dump_row<'a>(row: Record<'a>, file: &mut impl std::io::Write) -> Result<(), std::io::Error> {
    writeln!(
        file,
        "{:?},{},{:?},{},ns",
        row.file,
        row.line,
        row.name,
        row.duration.as_nanos()
    )
}

lazy_static::lazy_static! {
    static ref COMM: Mutex<CsvEmitter> = {
        let (sender, receiver) = channel::<Vec<Record<'static>>>();
        let worker = std::thread::spawn(move || {
            while let Ok(rows) = receiver.recv() {
                let mut file = PROF_FILE.lock().unwrap();

                for row in rows {
                    dump_row(row, &mut *file).expect("Failed to save profiling information");
                }
            }
        });
        let res = CsvEmitter {
            sender,
            _worker:worker,
        };
        Mutex::new(res)
    };
    static ref PROF_FILE: Mutex<File> = {
        use std::path::Path;
        let fname = std::env::var("CAO_PROFILE_CSV")
            .or_else(|_|Ok::<_, std::convert::Infallible>("profile.csv".to_owned()))
            .unwrap();
        let fname = Path::new(fname.as_str());
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(fname)
            .expect("profiler file");
        Mutex::new(file)
    };
}

thread_local!(
    pub static LOCAL_COMM: RefCell<LocalCsvEmitter> = {
        let comm = COMM.lock().unwrap();
        let sender = comm.get_sender();
        let res = LocalCsvEmitter {
            sender,
            container: Vec::with_capacity(1 << 15),
        };
        RefCell::new(res)
    };
);

struct CsvEmitter {
    _worker: std::thread::JoinHandle<()>,
    sender: Sender<Vec<Record<'static>>>,
}

pub struct LocalCsvEmitter {
    sender: Sender<Vec<Record<'static>>>,
    container: Vec<Record<'static>>,
}

impl LocalCsvEmitter {
    pub fn push(&mut self, r: Record<'static>) {
        self.container.push(r);
        if self.container.len() >= ((1 << 15) - 1) {
            let mut v = Vec::with_capacity(1 << 15);
            std::mem::swap(&mut v, &mut self.container);
            self.sender
                .send(v)
                .expect("Failed to send records for saving");
        }
    }
}

impl CsvEmitter {
    pub fn get_sender(&self) -> Sender<Vec<Record<'static>>> {
        self.sender.clone()
    }
}

impl Drop for LocalCsvEmitter {
    fn drop(&mut self) {
        let mut file = PROF_FILE.lock().unwrap();

        let v = Vec::new();
        let v = std::mem::replace(&mut self.container, v);
        for row in v.into_iter() {
            dump_row(row, &mut *file).expect("Failed to write to file");
        }
    }
}
