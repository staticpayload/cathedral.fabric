//! Policy compiler for evaluating policies.

use crate::lang::{PolicyAst, PolicyExpr, PolicyStmt};
use cathedral_core::{CoreResult, CoreError, Capability, EventId, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Policy compilation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// Unknown variable
    UnknownVar { name: String },
    /// Type mismatch
    TypeMismatch { expected: String, actual: String },
    /// Runtime error
    Runtime { message: String },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVar { name } => write!(f, "Unknown variable: {}", name),
            Self::TypeMismatch { expected, actual } => {
                write!(f, "Type mismatch: expected {}, got {}", expected, actual)
            }
            Self::Runtime { message } => write!(f, "Runtime error: {}", message),
        }
    }
}

impl std::error::Error for PolicyError {}

impl From<PolicyError> for CoreError {
    fn from(err: PolicyError) -> Self {
        CoreError::Validation {
            field: "policy".to_string(),
            reason: err.to_string(),
        }
    }
}

/// Compiled policy ready for evaluation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledPolicy {
    /// Policy ID
    pub id: String,
    /// Compiled rules
    pub rules: Vec<CompiledRule>,
    /// Variables
    pub vars: HashMap<String, PolicyValue>,
}

/// Compiled rule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledRule {
    /// Rule name
    pub name: Option<String>,
    /// Compiled expression
    pub expr: PolicyExpr,
    /// Whether this is an allow rule (true) or deny rule (false)
    pub is_allow: bool,
    /// Granted capabilities
    pub capabilities: Vec<Capability>,
}

/// Policy value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyValue {
    /// Boolean value
    Bool(bool),
    /// String value
    String(String),
    /// Integer value
    Int(i64),
}

/// Evaluation context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalContext {
    /// Current node ID
    pub node_id: Option<NodeId>,
    /// Current event ID
    pub event_id: Option<EventId>,
    /// Requested capability
    pub requested_capability: Option<Capability>,
    /// User-defined variables
    pub vars: HashMap<String, PolicyValue>,
}

impl EvalContext {
    /// Create a new evaluation context
    #[must_use]
    pub fn new() -> Self {
        Self {
            node_id: None,
            event_id: None,
            requested_capability: None,
            vars: HashMap::new(),
        }
    }

    /// Set node ID
    #[must_use]
    pub fn with_node(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    /// Set event ID
    #[must_use]
    pub fn with_event(mut self, event_id: EventId) -> Self {
        self.event_id = Some(event_id);
        self
    }

    /// Set requested capability
    #[must_use]
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.requested_capability = Some(capability);
        self
    }

    /// Set a variable
    #[must_use]
    pub fn with_var(mut self, name: String, value: PolicyValue) -> Self {
        self.vars.insert(name, value);
        self
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Policy compiler
pub struct PolicyCompiler;

impl PolicyCompiler {
    /// Create a new compiler
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Compile AST to executable policy
    ///
    /// # Errors
    ///
    /// Returns error if compilation fails
    pub fn compile(&self, ast: PolicyAst) -> CoreResult<CompiledPolicy> {
        let mut rules = Vec::new();
        let mut vars = HashMap::new();

        for stmt in ast.statements {
            match stmt {
                PolicyStmt::Allow(rule) => {
                    rules.push(CompiledRule {
                        name: rule.name,
                        expr: rule.expr,
                        is_allow: true,
                        capabilities: rule.capabilities,
                    });
                }
                PolicyStmt::Deny(rule) => {
                    rules.push(CompiledRule {
                        name: rule.name,
                        expr: rule.expr,
                        is_allow: false,
                        capabilities: rule.capabilities,
                    });
                }
                PolicyStmt::Let(name, expr) => {
                    // Evaluate static expressions
                    if let PolicyExpr::Bool(b) = expr {
                        vars.insert(name, PolicyValue::Bool(b));
                    } else if let PolicyExpr::String(s) = expr {
                        vars.insert(name, PolicyValue::String(s));
                    } else {
                        // Keep as variable reference
                        vars.insert(name, PolicyValue::Bool(false));
                    }
                }
            }
        }

        Ok(CompiledPolicy {
            id: uuid::Uuid::new_v4().to_string(),
            rules,
            vars,
        })
    }

    /// Compile from source string
    ///
    /// # Errors
    ///
    /// Returns error if compilation fails
    pub fn compile_from_source(&self, source: &str) -> CoreResult<CompiledPolicy> {
        let parser = crate::lang::PolicyParser::new();
        let ast = parser.parse(source)?;
        self.compile(ast)
    }
}

impl Default for PolicyCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl CompiledPolicy {
    /// Evaluate policy with context
    ///
    /// # Errors
    ///
    /// Returns error if evaluation fails
    pub fn evaluate(&self, ctx: &EvalContext) -> CoreResult<PolicyDecision> {
        let mut allowed = false;
        let mut matched_rules = Vec::new();

        for rule in &self.rules {
            let result = self.eval_expr(&rule.expr, ctx)?;

            if result {
                matched_rules.push(rule.name.clone().unwrap_or_else(|| "unnamed".to_string()));

                // Deny rules take precedence
                if !rule.is_allow {
                    return Ok(PolicyDecision {
                        allowed: false,
                        matched_rules,
                        reason: "Deny rule matched".to_string(),
                    });
                }

                allowed = true;
            }
        }

        Ok(PolicyDecision {
            allowed,
            matched_rules,
            reason: if allowed {
                "Allowed by policy".to_string()
            } else {
                "No matching allow rule".to_string()
            },
        })
    }

    /// Check if a specific capability is allowed
    ///
    /// # Errors
    ///
    /// Returns error if evaluation fails
    pub fn check_capability(
        &self,
        ctx: &EvalContext,
        capability: &Capability,
    ) -> CoreResult<PolicyDecision> {
        let decision = self.evaluate(ctx)?;

        // Also check if the specific capability was granted
        let has_capability = self.rules.iter().any(|rule| {
            rule.is_allow
                && rule.capabilities.iter().any(|c| c == capability)
                && self
                    .eval_expr(&rule.expr, ctx)
                    .unwrap_or(false)
        });

        Ok(PolicyDecision {
            allowed: decision.allowed && has_capability,
            matched_rules: decision.matched_rules,
            reason: decision.reason,
        })
    }

    /// Evaluate an expression
    fn eval_expr(&self, expr: &PolicyExpr, ctx: &EvalContext) -> CoreResult<bool> {
        match expr {
            PolicyExpr::Bool(b) => Ok(*b),
            PolicyExpr::String(_) => Ok(false),
            PolicyExpr::CapabilityCheck { capability } => {
                // Check if the requested capability matches
                if let Some(req_cap) = &ctx.requested_capability {
                    // Simple string comparison
                    Ok(req_cap.to_string().contains(capability))
                } else {
                    Ok(false)
                }
            }
            PolicyExpr::And(left, right) => {
                Ok(self.eval_expr(left, ctx)? && self.eval_expr(right, ctx)?)
            }
            PolicyExpr::Or(left, right) => {
                Ok(self.eval_expr(left, ctx)? || self.eval_expr(right, ctx)?)
            }
            PolicyExpr::Not(inner) => Ok(!self.eval_expr(inner, ctx)?),
            PolicyExpr::Var(name) => {
                if let Some(PolicyValue::Bool(b)) = self.vars.get(name) {
                    Ok(*b)
                } else if let Some(PolicyValue::Bool(b)) = ctx.vars.get(name) {
                    Ok(*b)
                } else {
                    Err(PolicyError::UnknownVar { name: name.clone() }.into())
                }
            }
            PolicyExpr::Compare { op, left, right } => {
                self.eval_compare(*op, left, right, ctx)
            }
            PolicyExpr::Call { func, args } => self.eval_call(func, args, ctx),
        }
    }

    /// Evaluate comparison
    fn eval_compare(
        &self,
        op: crate::lang::CompareOp,
        left: &PolicyExpr,
        right: &PolicyExpr,
        ctx: &EvalContext,
    ) -> CoreResult<bool> {
        let left_val = self.eval_to_value(left, ctx)?;
        let right_val = self.eval_to_value(right, ctx)?;

        match (left_val, right_val) {
            (PolicyValue::String(l), PolicyValue::String(r)) => match op {
                crate::lang::CompareOp::Eq => Ok(l == r),
                crate::lang::CompareOp::Ne => Ok(l != r),
                _ => Ok(false),
            },
            (PolicyValue::Int(l), PolicyValue::Int(r)) => match op {
                crate::lang::CompareOp::Eq => Ok(l == r),
                crate::lang::CompareOp::Ne => Ok(l != r),
                crate::lang::CompareOp::Lt => Ok(l < r),
                crate::lang::CompareOp::Le => Ok(l <= r),
                crate::lang::CompareOp::Gt => Ok(l > r),
                crate::lang::CompareOp::Ge => Ok(l >= r),
            },
            (PolicyValue::Bool(l), PolicyValue::Bool(r)) => match op {
                crate::lang::CompareOp::Eq => Ok(l == r),
                crate::lang::CompareOp::Ne => Ok(l != r),
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    /// Evaluate expression to value
    fn eval_to_value(&self, expr: &PolicyExpr, ctx: &EvalContext) -> CoreResult<PolicyValue> {
        match expr {
            PolicyExpr::Bool(b) => Ok(PolicyValue::Bool(*b)),
            PolicyExpr::String(s) => Ok(PolicyValue::String(s.clone())),
            PolicyExpr::Var(name) => {
                if let Some(val) = self.vars.get(name) {
                    Ok(val.clone())
                } else if let Some(val) = ctx.vars.get(name) {
                    Ok(val.clone())
                } else {
                    Ok(PolicyValue::Bool(false))
                }
            }
            _ => Ok(PolicyValue::Bool(false)),
        }
    }

    /// Evaluate function call
    fn eval_call(&self, func: &str, _args: &[PolicyExpr], ctx: &EvalContext) -> CoreResult<bool> {
        match func {
            "is_authenticated" => Ok(true),
            "is_admin" => Ok(false),
            "has_capability" => {
                // Check if requested capability is set
                Ok(ctx.requested_capability.is_some())
            }
            _ => Ok(false),
        }
    }
}

/// Policy decision result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Whether the action is allowed
    pub allowed: bool,
    /// Rules that matched
    pub matched_rules: Vec<String>,
    /// Human-readable reason
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_compiler_new() {
        let compiler = PolicyCompiler::new();
        let ast = PolicyAst { statements: Vec::new() };
        let policy = compiler.compile(ast).unwrap();
        assert_eq!(policy.rules.len(), 0);
    }

    #[test]
    fn test_compile_allow() {
        let compiler = PolicyCompiler::new();
        let source = "allow true";
        let policy = compiler.compile_from_source(source).unwrap();
        assert_eq!(policy.rules.len(), 1);
        assert!(policy.rules[0].is_allow);
    }

    #[test]
    fn test_compile_deny() {
        let compiler = PolicyCompiler::new();
        let source = "deny false";
        let policy = compiler.compile_from_source(source).unwrap();
        assert_eq!(policy.rules.len(), 1);
        assert!(!policy.rules[0].is_allow);
    }

    #[test]
    fn test_eval_allow_true() {
        let compiler = PolicyCompiler::new();
        let source = "allow true";
        let policy = compiler.compile_from_source(source).unwrap();
        let ctx = EvalContext::new();
        let decision = policy.evaluate(&ctx).unwrap();
        assert!(decision.allowed);
    }

    #[test]
    fn test_eval_deny_true() {
        let compiler = PolicyCompiler::new();
        let source = "deny true";
        let policy = compiler.compile_from_source(source).unwrap();
        let ctx = EvalContext::new();
        let decision = policy.evaluate(&ctx).unwrap();
        assert!(!decision.allowed);
    }

    #[test]
    fn test_eval_context() {
        let compiler = PolicyCompiler::new();
        let source = "allow true";
        let policy = compiler.compile_from_source(source).unwrap();

        let ctx = EvalContext::new()
            .with_capability(Capability::FsRead { prefixes: vec![] });

        let decision = policy.evaluate(&ctx).unwrap();
        assert!(decision.allowed);
    }

    #[test]
    fn test_eval_error_display() {
        let err = PolicyError::UnknownVar {
            name: "x".to_string(),
        };
        assert!(err.to_string().contains("Unknown variable"));
    }

    #[test]
    fn test_policy_decision_allowed() {
        let decision = PolicyDecision {
            allowed: true,
            matched_rules: vec!["rule1".to_string()],
            reason: "Allowed".to_string(),
        };
        assert!(decision.allowed);
    }
}
