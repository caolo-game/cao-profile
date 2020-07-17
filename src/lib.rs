//! Records high-level profiling information to `profile.csv`.
//! Recording is done via a thread-local buffer and dedicated file writing thread, in an attempt to
//! mitigate overhead.
//!
//! Enabling the `disable` feature will disable data collection. Allowing you to roll releases
//! without modifying code.
//!
//! ```
//! use cao_profile::profile;
//!
//! fn foo() {
//!     profile!("foo fn call");
//! }
//!
//! foo();
//! ```
//!
#[cfg(not(feature = "disable"))]
mod profiler;

pub use profiler::*;

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
#[cfg(feature = "disable")]
mod profiler {
    pub struct Profiler;

    impl Profiler {
        pub fn new(_file: &'static str, _line: u32, _name: &'static str) -> Self {
            Self
        }
    }
}
