//! Unofficial Rust client for Flackle.
//!
//! This is a native port of the Bun/TypeScript `flackle-selfbot` library. It
//! speaks Flackle's REST API and realtime gateway directly.
//!
//! ```no_run
//! # async fn run() -> flackle_core::Result<()> {
//! use flackle_core::{Client, LoginOutcome};
//!
//! let client = Client::new();
//! match client.login("you@example.com", "password").await? {
//!     LoginOutcome::Success(user) => println!("hello {}", user.username),
//!     LoginOutcome::MfaRequired { .. } => {}
//! }
//! client.connect().await?;
//! # Ok(()) }
//! ```

mod client;
mod error;
mod gateway;
mod models;
mod rest;

pub use client::{Bootstrap, Client, FetchMessagesOptions, LoginOutcome, Target};
pub use error::{Error, Result};
pub use gateway::Command as GatewayCommand;
pub use models::*;
pub use rest::{Tokens, DEFAULT_BASE_URL, DEFAULT_USER_AGENT};
