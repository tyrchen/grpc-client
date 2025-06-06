mod codec;

pub mod cli;
pub mod client;
pub mod connection;
pub mod domain;
pub mod format;
pub mod reflection;
pub mod server;

// Re-export main types for convenience
pub use cli::{Cli, Command, FormatType};
pub use client::GrpcClient;
pub use domain::*;
