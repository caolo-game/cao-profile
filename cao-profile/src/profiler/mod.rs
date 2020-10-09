#[cfg(feature = "http")]
mod http_emitter;

#[allow(unused)]
use crate::Record;

use std::time::SystemTime;

/// Output execution of it's scope.
pub struct Profiler {
    #[allow(unused)]
    start: SystemTime,
    #[allow(unused)]
    name: &'static str,
    #[allow(unused)]
    file: &'static str,
    #[allow(unused)]
    line: u32,
}

impl Profiler {
    pub fn new(file: &'static str, line: u32, name: &'static str) -> Self {
        let start = SystemTime::now();
        Self {
            name,
            start,
            file,
            line,
        }
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        #![allow(unused)]

        let end = SystemTime::now();
        let elapsed = end.duration_since(self.start).expect("elapsed");

        #[cfg(feature = "http")]
        http_emitter::LOCAL_EMITTER.with(|comm| {
            comm.borrow_mut().push(Record {
                time: end
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("failed to get duration since epoch")
                    .as_millis(),
                name: self.name,
                file: self.file,
                line: self.line,
                duration: elapsed,
            })
        });
    }
}
