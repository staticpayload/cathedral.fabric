//! CATHEDRAL.FABRIC TUI
//!
//! Terminal UI for viewing traces and audit logs.

#![warn(missing_docs)]
#![warn(clippy::all)]

use std::process;
use cathedral_tui::{TuiApp, TuiError};
use clap::Parser;

#[derive(Parser)]
#[command(name = "cathedral-tui")]
#[command(about = "CATHEDRAL.FABRIC TUI", long_about = None)]
struct Args {
    /// Path to bundle or log file
    #[arg(short, long)]
    input: String,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = TuiApp::new(&args.input).and_then(|mut app| app.run()) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
