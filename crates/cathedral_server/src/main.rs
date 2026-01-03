//! CATHEDRAL.FABRIC Server
//!
//! HTTP API server for remote execution and cluster management.

#![warn(missing_docs)]
#![warn(clippy::all)]

use anyhow::Result;
use cathedral_server::api::ApiServer;
use clap::Parser;

#[derive(Parser)]
#[command(name = "cathedral-server")]
#[command(about = "CATHEDRAL.FABRIC server", long_about = None)]
struct Args {
    /// Bind address
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    bind: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter("cathedral=debug,tower_http=debug")
        .init();

    let server = ApiServer::new(&args.bind)?;
    server.serve().await?;

    Ok(())
}
