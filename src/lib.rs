#![warn(clippy::pedantic)]
#![allow(
    clippy::unused_async,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc
)]

pub mod config;
pub mod handlers;
pub mod startup;
pub mod telemetry;
