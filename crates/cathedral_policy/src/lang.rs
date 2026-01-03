//! Policy language parser

use crate::compiler::CompiledPolicy;
use cathedral_core::error::CoreResult;

pub struct PolicyParser;
pub struct PolicyAst;
pub struct PolicyExpr;

impl PolicyParser {
    pub fn parse(&self, _input: &str) -> CoreResult<PolicyAst> {
        Ok(PolicyAst)
    }
}
