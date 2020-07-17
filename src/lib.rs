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
//! | Name  | Enabled by default | Description                                                                                                                                                |
//! | ----- | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
//! | `csv` | `true`             | Logs profiling data to a CSV file. The default file path is `profile.csv`, which can be owerwritten by setting the `CAO_PROFILE_CSV` environment variable. |
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
//! // outputs something similar:
//!
//! // [src\lib.rs::7::foo fn call label],600,ns
//! // [src\lib.rs::7::foo fn call label],100,ns
//! // [src\lib.rs::7::foo fn call label],0,ns
//! ```
#[cfg(any(feature = "csv"))]
mod profiler;

pub use profiler::Profiler;
use std::time::Duration;

#[derive(Debug, Clone)]
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

// In case profiling is disable we replace the `Profiler` struct with a unit struct.
#[cfg(not(any(feature = "csv")))]
mod profiler {
    pub struct Profiler;

    impl Profiler {
        pub fn new(_file: &'static str, _line: u32, _name: &'static str) -> Self {
            Self
        }
    }
}
