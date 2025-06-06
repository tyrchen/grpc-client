use clap::{Parser, Subcommand, ValueEnum};

/// A modern gRPC command-line client
#[derive(Parser, Clone)]
#[command(name = "grpc-client")]
#[command(about = "A modern gRPC command-line client")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Skip TLS (use plain HTTP/2)
    #[arg(long)]
    pub plaintext: bool,

    /// Path to CA certificate file for TLS verification
    #[arg(long)]
    pub ca: Option<String>,

    /// Additional headers in 'name: value' format
    #[arg(short = 'H', long)]
    pub header: Vec<String>,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: FormatType,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand, Clone)]
pub enum Command {
    /// List services or methods
    List {
        /// Server endpoint (host:port)
        endpoint: String,
        /// Optional service name to list methods for
        service: Option<String>,
    },
    /// Describe a service, method, or message
    Describe {
        /// Server endpoint (host:port)
        endpoint: String,
        /// Symbol to describe
        symbol: String,
    },
    /// Invoke a gRPC method
    Call {
        /// Server endpoint (host:port)
        endpoint: String,
        /// Method to call (service.method or service/method)
        method: String,
        /// Request data (JSON string or @filename or @- for stdin)
        #[arg(short, long)]
        data: Option<String>,
        /// Emit default values in JSON output
        #[arg(long)]
        emit_defaults: bool,
    },
    /// Start web server for UI interface
    Server {
        /// Port to run web server on
        #[arg(short, long, default_value = "4000")]
        port: u16,
        /// Path to YAML configuration file
        #[arg(short, long, default_value = "fixtures/app.yml")]
        config: String,
        /// Path to UI assets directory
        #[arg(long, default_value = "ui/dist")]
        ui_path: String,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum FormatType {
    Json,
    Text,
}
