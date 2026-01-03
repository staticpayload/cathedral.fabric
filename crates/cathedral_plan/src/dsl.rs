//! DSL parser for workflow definitions.

use cathedral_core::{CoreResult, CoreError};
use super::compiler::Ast;

/// Parse a workflow definition into an AST
///
/// # Errors
///
/// Returns error if parsing fails
pub fn parse(_input: &str) -> CoreResult<Ast> {
    // TODO: Implement actual parsing
    // For now, return an empty AST
    Ok(Ast::new())
}

/// Parse error type
pub type ParseError = CoreError;

/// Re-export AST for convenience
pub use super::compiler::{Ast, Statement, Expr};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse("");
        assert!(result.is_ok());
        assert!(result.unwrap().statements.is_empty());
    }

    #[test]
    fn test_parse_returns_ast() {
        let result = parse("input x: string");
        assert!(result.is_ok());
    }
}
