//! Policy language parser for capability policies.

use cathedral_core::{CoreResult, CoreError, Capability};
use serde::{Deserialize, Serialize};

/// Policy language parser
pub struct PolicyParser;

/// Abstract syntax tree for a policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyAst {
    /// Policy statements
    pub statements: Vec<PolicyStmt>,
}

/// Policy statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyStmt {
    /// Allow rule
    Allow(PolicyRule),
    /// Deny rule
    Deny(PolicyRule),
    /// Variable definition
    Let(String, PolicyExpr),
}

/// Policy rule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule name
    pub name: Option<String>,
    /// Rule expression
    pub expr: PolicyExpr,
    /// Capabilities granted by this rule
    pub capabilities: Vec<Capability>,
}

/// Policy expression
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyExpr {
    /// Boolean literal
    Bool(bool),
    /// String literal
    String(String),
    /// Capability check
    CapabilityCheck { capability: String },
    /// Logical AND
    And(Box<PolicyExpr>, Box<PolicyExpr>),
    /// Logical OR
    Or(Box<PolicyExpr>, Box<PolicyExpr>),
    /// Logical NOT
    Not(Box<PolicyExpr>),
    /// Variable reference
    Var(String),
    /// Comparison
    Compare {
        op: CompareOp,
        left: Box<PolicyExpr>,
        right: Box<PolicyExpr>,
    },
    /// Function call
    Call {
        func: String,
        args: Vec<PolicyExpr>,
    },
}

/// Comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
}

impl PolicyParser {
    /// Create a new parser
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Parse policy from string
    ///
    /// # Errors
    ///
    /// Returns error if parsing fails
    pub fn parse(&self, input: &str) -> CoreResult<PolicyAst> {
        let mut statements = Vec::new();
        let mut lines = input.lines();
        let mut line_num = 0;

        while let Some(line) = lines.next() {
            line_num += 1;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse statement
            let stmt = self.parse_statement(line)
                .map_err(|e| CoreError::Validation {
                    field: "policy".to_string(),
                    reason: format!("Line {}: {}", line_num, e),
                })?;
            statements.push(stmt);
        }

        Ok(PolicyAst { statements })
    }

    /// Parse a single statement
    fn parse_statement(&self, input: &str) -> Result<PolicyStmt, String> {
        let input = input.trim();

        // Check for let binding
        if input.starts_with("let ") {
            return self.parse_let(input);
        }

        // Check for allow/deny
        if input.starts_with("allow ") {
            self.parse_rule(input, true)
        } else if input.starts_with("deny ") {
            self.parse_rule(input, false)
        } else {
            Err(format!("Unknown statement: {}", input))
        }
    }

    /// Parse let binding
    fn parse_let(&self, input: &str) -> Result<PolicyStmt, String> {
        let rest = input[4..].trim();
        let eq_idx = rest
            .find('=')
            .ok_or_else(|| "Expected '=' in let binding".to_string())?;

        let name = rest[..eq_idx].trim().to_string();
        let expr_str = rest[eq_idx + 1..].trim();

        // For simplicity, just parse boolean literals
        let expr = self.parse_expr(expr_str)?;

        Ok(PolicyStmt::Let(name, expr))
    }

    /// Parse allow/deny rule
    fn parse_rule(&self, input: &str, is_allow: bool) -> Result<PolicyStmt, String> {
        let rest = input[if is_allow { 6 } else { 5 }..].trim();

        // Parse rule as: "name: expr => [caps]" or just "expr"
        let (name, rest) = if let Some(colon_idx) = rest.find(':') {
            let name = Some(rest[..colon_idx].trim().to_string());
            (name, rest[colon_idx + 1..].trim())
        } else {
            (None, rest)
        };

        // Split by =>
        let parts: Vec<&str> = rest.split("=>").collect();
        let (expr_str, caps_str) = if parts.len() == 2 {
            (parts[0].trim(), Some(parts[1].trim()))
        } else {
            (rest, None)
        };

        let expr = self.parse_expr(expr_str)?;

        // Parse capabilities
        let capabilities = if let Some(caps_str) = caps_str {
            self.parse_capabilities(caps_str)?
        } else {
            Vec::new()
        };

        let rule = PolicyRule {
            name,
            expr,
            capabilities,
        };

        Ok(if is_allow {
            PolicyStmt::Allow(rule)
        } else {
            PolicyStmt::Deny(rule)
        })
    }

    /// Parse expression
    fn parse_expr(&self, input: &str) -> Result<PolicyExpr, String> {
        let input = input.trim();

        // Boolean literals
        match input {
            "true" => return Ok(PolicyExpr::Bool(true)),
            "false" => return Ok(PolicyExpr::Bool(false)),
            _ => {}
        }

        // String literals
        if input.starts_with('"') && input.ends_with('"') {
            return Ok(PolicyExpr::String(input[1..input.len() - 1].to_string()));
        }

        // NOT
        if input.starts_with('!') {
            let inner = self.parse_expr(&input[1..])?;
            return Ok(PolicyExpr::Not(Box::new(inner)));
        }

        // AND
        if let Some(and_idx) = input.find(" && ") {
            let left = self.parse_expr(&input[..and_idx])?;
            let right = self.parse_expr(&input[and_idx + 4..])?;
            return Ok(PolicyExpr::And(Box::new(left), Box::new(right)));
        }

        // OR
        if let Some(or_idx) = input.find(" || ") {
            let left = self.parse_expr(&input[..or_idx])?;
            let right = self.parse_expr(&input[or_idx + 4..])?;
            return Ok(PolicyExpr::Or(Box::new(left), Box::new(right)));
        }

        // Comparison operators (simplified)
        if let Some(idx) = input.find("==") {
            let left = self.parse_expr(&input[..idx])?;
            let right = self.parse_expr(&input[idx + 2..])?;
            return Ok(PolicyExpr::Compare {
                op: CompareOp::Eq,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if let Some(idx) = input.find("!=") {
            let left = self.parse_expr(&input[..idx])?;
            let right = self.parse_expr(&input[idx + 2..])?;
            return Ok(PolicyExpr::Compare {
                op: CompareOp::Ne,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        // Function call - simplified without nested if
        if let Some(open_idx) = input.find('(') {
            if input.len() > open_idx + 1 {
                let func = input[..open_idx].to_string();
                let args_str = &input[open_idx + 1..input.len() - 1];
                let args: Vec<PolicyExpr> = if args_str.is_empty() {
                    Vec::new()
                } else {
                    args_str
                        .split(',')
                        .map(|s| self.parse_expr(s.trim()))
                        .collect::<Result<_, _>>()?
                };
                return Ok(PolicyExpr::Call { func, args });
            }
        }

        // Variable or capability check
        Ok(PolicyExpr::Var(input.to_string()))
    }

    /// Parse capabilities list
    fn parse_capabilities(&self, input: &str) -> Result<Vec<Capability>, String> {
        // Remove brackets
        let inner = input
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| "Expected [capabilities]".to_string())?;

        // For simplicity, return empty vec
        // In a real implementation, this would parse the capability strings
        let _ = inner;
        Ok(Vec::new())
    }
}

impl Default for PolicyParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_parser_new() {
        let parser = PolicyParser::new();
        let ast = parser.parse("").unwrap();
        assert_eq!(ast.statements.len(), 0);
    }

    #[test]
    fn test_policy_parse_allow() {
        let parser = PolicyParser::new();
        let ast = parser.parse("allow true").unwrap();
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_policy_parse_deny() {
        let parser = PolicyParser::new();
        let ast = parser.parse("deny false").unwrap();
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_policy_parse_and() {
        let parser = PolicyParser::new();
        let ast = parser.parse("allow true && false").unwrap();
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_policy_parse_or() {
        let parser = PolicyParser::new();
        let ast = parser.parse("allow true || false").unwrap();
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_policy_parse_not() {
        let parser = PolicyParser::new();
        let ast = parser.parse("allow !false").unwrap();
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_policy_parse_comments() {
        let parser = PolicyParser::new();
        let ast = parser.parse("# comment\nallow true").unwrap();
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_policy_parse_let() {
        let parser = PolicyParser::new();
        let ast = parser.parse("let x = true\nallow x").unwrap();
        assert_eq!(ast.statements.len(), 2);
    }

    #[test]
    fn test_policy_parse_with_name() {
        let parser = PolicyParser::new();
        let ast = parser.parse("allow my_rule: true").unwrap();
        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0] {
            PolicyStmt::Allow(rule) => {
                assert_eq!(rule.name, Some("my_rule".to_string()));
            }
            _ => panic!("Expected Allow rule"),
        }
    }

    #[test]
    fn test_compare_op() {
        assert_eq!(CompareOp::Eq, CompareOp::Eq);
        assert_ne!(CompareOp::Eq, CompareOp::Ne);
    }
}
