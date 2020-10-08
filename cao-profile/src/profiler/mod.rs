#[cfg(feature = "http")]
mod http_emitter;

#[allow(unused)]
use crate::Record;

use std::time::Instant;

/// Output execution of it's scope.
pub struct Profiler {
    #[allow(unused)]
    start: Instant,
    #[allow(unused)]
    name: &'static str,
    #[allow(unused)]
    file: &'static str,
    #[allow(unused)]
    line: u32,
}

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

impl Drop for Profiler {
    fn drop(&mut self) {
        #![allow(unused)]

        let end = Instant::now();
        let dur = end - self.start;

        #[cfg(feature = "http")]
        http_emitter::LOCAL_EMITTER.with(|comm| {
            comm.borrow_mut().push(Record {
                name: self.name,
                file: self.file,
                line: self.line,
                duration: dur,
            })
        });
    }
}
