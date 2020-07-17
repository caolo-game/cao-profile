use std::cell::RefCell;
use std::fs::File;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Output execution of it's scope.
/// Output is in CSV format: name, time, timeunit
#[cfg(not(feature = "disable"))]
pub struct Profiler {
    start: Instant,
    name: &'static str,
    file: &'static str,
    line: u32,
}

#[cfg(not(feature = "disable"))]
impl Profiler {
    pub fn new(file: &'static str, line: u32, name: &'static str) -> Self {
        let start = Instant::now();
        Self {
            name,
            start,
            file,
            line,
        }
    }
}

#[cfg(not(feature = "disable"))]
impl Drop for Profiler {
    fn drop(&mut self) {
        let end = Instant::now();
        let dur = end - self.start;

        #[cfg(not(feature = "disable"))]
        {
            LOCAL_COMM.with(|comm| {
                comm.borrow_mut().push(Record {
                    name: self.name,
                    file: self.file,
                    line: self.line,
                    duration: dur,
                })
            });
        }
    }
}

#[cfg(not(feature = "disable"))]
lazy_static::lazy_static! {
    static ref COMM: Mutex<Aggregate> = {
        let (sender, receiver) = channel::<Vec<Record<'static>>>();
        let worker = std::thread::spawn(move || {
            while let Ok(rows) = receiver.recv() {
                use std::io::Write;

                let mut file = PROF_FILE.lock().unwrap();

                for row in rows {
                    writeln!(
                        file,
                        "[{}::{}::{}],{},ns",
                        row.file,
                        row.line,
                        row.name,
                        row.duration.as_nanos()
                    ).expect("Failed to save profiling information");
                }
            }
        });
        let res = Aggregate {
            sender,
            _worker:worker,
        };
        Mutex::new(res)
    };
    static ref PROF_FILE: Mutex<File> = {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open("profile.csv")
            .expect("profiler file");
        Mutex::new(file)
    };
}

#[cfg(not(feature = "disable"))]
thread_local!(
    static LOCAL_COMM: RefCell<LocalAggregate> = {
        let comm = COMM.lock().unwrap();
        let sender = comm.get_sender();
        let res = LocalAggregate {
            sender,
            container: Vec::with_capacity(1 << 15),
        };
        RefCell::new(res)
    };
);

#[cfg(not(feature = "disable"))]
struct Aggregate {
    _worker: std::thread::JoinHandle<()>,
    sender: Sender<Vec<Record<'static>>>,
}

#[cfg(not(feature = "disable"))]
struct LocalAggregate {
    sender: Sender<Vec<Record<'static>>>,
    container: Vec<Record<'static>>,
}

#[cfg(not(feature = "disable"))]
impl LocalAggregate {
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

    fn save<'a>(v: &[Record<'a>]) {
        use std::io::Write;

        let mut file = PROF_FILE.lock().unwrap();

        for row in v.iter() {
            writeln!(
                file,
                "[{}::{}::{}],{},ns",
                row.file,
                row.line,
                row.name,
                row.duration.as_nanos()
            )
            .expect("Failed to write to file");
        }
    }
}

#[cfg(not(feature = "disable"))]
impl Aggregate {
    pub fn get_sender(&self) -> Sender<Vec<Record<'static>>> {
        self.sender.clone()
    }
}

#[cfg(not(feature = "disable"))]
impl Drop for LocalAggregate {
    fn drop(&mut self) {
        Self::save(&self.container);
    }
}

struct Record<'a> {
    duration: Duration,
    name: &'a str,
    file: &'a str,
    line: u32,
}
