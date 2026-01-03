//! Tool trait

use cathedral_core::error::CoreResult;

pub struct ToolOutput;
pub struct ToolError;

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, input: &[u8]) -> CoreResult<ToolOutput>;
}
