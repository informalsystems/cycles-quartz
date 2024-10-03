#![no_std]
#![forbid(unsafe_code)]
#![warn(
    clippy::checked_conversions,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_import_braces,
    unused_qualifications
)]

extern crate alloc;

mod error;
mod null_io;
mod provider;

pub use error::Error;
pub use provider::{make_provider, StatelessProvider};