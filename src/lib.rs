//! # kilar
//!
//! A powerful CLI tool for managing port processes on your system.
//!
//! ## Features
//!
//! - Check port usage status
//! - Kill processes using specific ports
//! - List all ports in use with filtering and sorting options
//! - Interactive process selection for termination
//! - JSON output support for scripting
//!
//! ## Example
//!
//! ```no_run
//! use kilar::commands::CheckCommand;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Check if port 3000 is in use
//!     CheckCommand::execute(3000, "tcp", false, false, false).await.unwrap();
//! }
//! ```

pub mod cli;
pub mod commands;
pub mod error;
pub mod port;
pub mod process;
pub mod utils;

pub use error::{Error, Result};
