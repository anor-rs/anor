//! The Storage API Service for [Anor](https://github.com/anor-rs)
//!
//! ## Project Stage
//!
//! **Research:** This project is at the design stage, with some sketches of work but nothing usable yet.

pub mod service;
pub mod client;

pub use service::api_service::*;
pub use client::api_client::*;