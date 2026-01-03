//! CATHEDRAL.FABRIC TUI
//!
//! Terminal UI for viewing traces and audit logs.

#![warn(missing_docs)]
#![warn(clippy::all)]

use cathedral_tui::ui::TuiApp;
use clap::Parser;
use color_eyre::Result;

#[derive(Parser)]
#[command(name = "cathedral-tui")]
#[command(about = "CATHEDRAL.FABRIC TUI", long_about = None)]
struct Args {
    /// Path to bundle or log file
    #[arg(short, long)]
    input: String,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let app = TuiApp::new(&args.input)?;
    app.run()?;

    Ok(())
}
