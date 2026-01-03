//! TUI app

use cathedral_core::error::CoreResult;

pub struct TuiApp;
pub struct TuiConfig;
pub struct TuiError;

impl TuiApp {
    pub fn new(_input: &str) -> CoreResult<Self> {
        Ok(TuiApp)
    }

    pub fn run(&self) -> CoreResult<()> {
        Ok(())
    }
}
