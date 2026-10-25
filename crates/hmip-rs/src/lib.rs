//! The crate documentation is the README, so docs.rs, crates.io and GitHub show the same text and
//! every example in it is compiled by `cargo test`.
//!
//! # Where to look
//!
//! * [`Plugin`] and [`PluginBuilder`] – connect to the HCU and run a plugin.
//! * [`Handler`] – the trait you implement to react to messages; [`Context`] – how you answer.
//! * [`api`] – the websocket messages ([`api::Message`], [`api::Payload`] and one body struct per
//!   message type).
//! * [`domain`] – devices, [features](domain::features), configuration templates and the
//!   [enumerations](domain::types) of the Connect API.
//! * [`Error`] and [`Result`] – error handling.
//!
//! ---
//!
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod api;
pub mod domain;
mod error;
mod plugin;

pub use error::{Error, Result};
pub use plugin::{
    Context, DEFAULT_HOST, DEFAULT_PORT, DEFAULT_TOKEN_PATH, Handler, Plugin, PluginBuilder,
};
