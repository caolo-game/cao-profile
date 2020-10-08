//! Records high-level profiling information. ( See [Record](./struct.Record.html) ).
//!
//! Recording is done via a thread-local buffer and dedicated file writing thread, in an attempt to
//! mitigate overhead.
//!
//! Disabling all features will disable data collection and replacing `Profile` structs with an empty struct.
//! Allowing you to roll release builds without the profiler overhead and also without modifying code.
//!
//! ## Features
//!
//! | Name   | Enabled by default | Description                                                                                                                                                |
//! | ------ | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
//! | `http` | `false`            | Logs profiling data to a HTTP server. Set the `CAO_PROFILE_URI` environment variable.                                                                      |
//! | `log`  | `false`            | Logs errors using the log crate, instead of stderr.                                                                                                        |
//!
//! ## Example
//!
//! ```
//! use cao_profile::profile;
//!
//! fn foo() {
//!     profile!("foo fn call label");
//! }
//!
//! foo();
//! foo();
//! foo();
//!
//! // Outputs something similar to:
//!
//! // "src/lib.rs",7,"foo fn call label",200,ns
//! // "src/lib.rs",7,"foo fn call label",100,ns
//! // "src/lib.rs",7,"foo fn call label",0,ns
//! ```
#[cfg(any(feature = "csv", feature = "http"))]
mod profiler;

pub use profiler::Profiler;
use std::time::Duration;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Record<'a> {
    pub duration: Duration,
    pub name: &'a str,
    pub file: &'a str,
    pub line: u32,
}

#[macro_export]
macro_rules! profile {
    ($name: expr) => {
        let _profile = {
            use cao_profile::Profiler;

            Profiler::new(std::file!(), std::line!(), $name)
        };
    };
}

#[macro_export(internal_macros)]
macro_rules! trace {
    ($($args: expr),*) => {
        #[cfg(feature="log")]
        log::trace!($($args),*);
    }
}

#[macro_export(internal_macros)]
macro_rules! warn {
    ($($args: expr),*) => {
        #[cfg(feature="log")]
        log::warn!($($args),*);
        #[cfg(not(feature="log"))]
        eprintln!($($args),*);
    }
}

#[macro_export(internal_macros)]
macro_rules! error {
    ($($args: expr),*) => {
        #[cfg(feature="log")]
        log::error!($($args),*);
        #[cfg(not(feature="log"))]
        eprintln!($($args),*);
    }
}

// In case profiling is disable we replace the `Profiler` struct with a unit struct.
#[cfg(not(any(feature = "csv", feature = "http")))]
mod profiler {
    pub struct Profiler;

    impl Profiler {
        pub fn new(_file: &'static str, _line: u32, _name: &'static str) -> Self {
            Self
        }
    }
}

#[cfg(test)]
mod test {
    use crate as cao_profile;
    use crate::profile;

    #[test]
    fn smoke() {
        fn bar() {
            profile!("bar fn call label");
        }

        for _ in 0..1 << 17 {
            bar();
        }
    }
}
