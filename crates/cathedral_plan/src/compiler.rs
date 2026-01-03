//! Compiler from DSL AST to executable DAG.

use cathedral_core::{NodeId, Capability, CoreResult};
use indexmap::IndexSet;
use super::dag::{Dag, Node, Edge, NodeKind, ResourceRequirements};

/// Output from compiling a workflow
#[derive(Debug, Clone)]
pub struct CompilerOutput {
    /// The compiled DAG
    pub dag: Dag,
    /// Compilation warnings
    pub warnings: Vec<CompilerWarning>,
}

/// Compilation warning
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerWarning {
    /// Unused variable
    UnusedVariable { name: String },
    /// Deprecated feature
    Deprecated { feature: String },
    /// Resource limit might be exceeded
    ResourceLimit { resource: String },
}

/// Compiler for transforming AST to DAG
pub struct Compiler {
    /// Next node ID counter
    next_id: u64,
}

impl Compiler {
    /// Create a new compiler
    #[must_use]
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    /// Compile an AST to a DAG
    ///
    /// # Errors
    ///
    /// Returns error if compilation fails
    pub fn compile(&mut self, ast: &Ast) -> CoreResult<CompilerOutput> {
        let mut dag = Dag::new();
        let mut warnings = Vec::new();

        // Compile each statement in the AST
        for stmt in &ast.statements {
            self.compile_statement(stmt, &mut dag, &mut warnings)?;
        }

        // Validate the resulting DAG
        dag.validate()?;

        Ok(CompilerOutput { dag, warnings })
    }

    /// Compile a single statement
    fn compile_statement(
        &mut self,
        stmt: &Statement,
        dag: &mut Dag,
        warnings: &mut Vec<CompilerWarning>,
    ) -> CoreResult<NodeId> {
        match stmt {
            Statement::ToolCall { name, args, .. } => {
                let node = Node {
                    id: self.next_node_id(),
                    kind: NodeKind::Tool {
                        name: name.clone(),
                        version: "1.0.0".to_string(),
                    },
                    dependencies: IndexSet::new(),
                    capabilities: self.infer_capabilities(name, args),
                    resources: ResourceRequirements::new(),
                };
                let id = node.id;
                dag.add_node(node)?;
                Ok(id)
            }
            Statement::Input { name, .. } => {
                let node = Node {
                    id: self.next_node_id(),
                    kind: NodeKind::Input {
                        schema: format!("schema_{}", name),
                    },
                    dependencies: IndexSet::new(),
                    capabilities: Vec::new(),
                    resources: ResourceRequirements::new(),
                };
                let id = node.id;
                dag.add_node(node)?;
                Ok(id)
            }
            Statement::Output { name, .. } => {
                let node = Node {
                    id: self.next_node_id(),
                    kind: NodeKind::Output {
                        schema: format!("schema_{}", name),
                    },
                    dependencies: IndexSet::new(),
                    capabilities: Vec::new(),
                    resources: ResourceRequirements::new(),
                };
                let id = node.id;
                dag.add_node(node)?;
                Ok(id)
            }
            Statement::Sequence { statements } => {
                let mut prev_id = None;
                for stmt in statements {
                    let id = self.compile_statement(stmt, dag, warnings)?;
                    if let Some(prev) = prev_id {
                        dag.add_edge(Edge::new(prev, id))?;
                    }
                    prev_id = Some(id);
                }
                Ok(prev_id.unwrap_or_else(|| self.next_node_id()))
            }
            Statement::Parallel { branches } => {
                let branch_ids: Vec<NodeId> = branches
                    .iter()
                    .map(|stmt| self.compile_statement(stmt, dag, warnings))
                    .collect::<CoreResult<Vec<_>>>()?;

                // Create a parallel aggregation node
                let agg_id = self.next_node_id();
                let agg_node = Node {
                    id: agg_id,
                    kind: NodeKind::Reduce {
                        function: "merge".to_string(),
                        initial: Vec::new(),
                    },
                    dependencies: branch_ids.iter().copied().collect(),
                    capabilities: Vec::new(),
                    resources: ResourceRequirements::new(),
                };
                dag.add_node(agg_node)?;

                // Add edges from each branch to the aggregation node
                for bid in &branch_ids {
                    dag.add_edge(Edge::new(*bid, agg_id))?;
                }

                Ok(agg_id)
            }
        }
    }

    /// Infer capabilities required for a tool call
    fn infer_capabilities(&self, name: &str, _args: &[Expr]) -> Vec<Capability> {
        // Simple heuristic-based capability inference
        match name {
            "read_file" | "read_files" => vec![Capability::FsRead {
                prefixes: vec![".".to_string()],
            }],
            "write_file" => vec![Capability::FsWrite {
                prefixes: vec!["./outputs".to_string()],
            }],
            "http_get" | "http_post" => vec![Capability::NetRead {
                allowlist: vec!["*".to_string()],
            }],
            "exec" => vec![Capability::Exec {
                cpu_limit: "1000".to_string(),
                mem_limit: "1GB".to_string(),
            }],
            _ => Vec::new(),
        }
    }

    /// Generate the next node ID
    fn next_node_id(&mut self) -> NodeId {
        let id = NodeId::new(); // In real implementation, use deterministic IDs
        self.next_id += 1;
        id
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// AST statement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// Tool invocation
    ToolCall {
        name: String,
        args: Vec<Expr>,
        output: Option<String>,
    },
    /// Input definition
    Input {
        name: String,
        schema: String,
    },
    /// Output definition
    Output {
        name: String,
        value: Expr,
    },
    /// Sequential composition
    Sequence {
        statements: Vec<Statement>,
    },
    /// Parallel execution
    Parallel {
        branches: Vec<Statement>,
    },
}

/// Expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// String literal
    String(String),
    /// Integer literal
    Integer(i64),
    /// Variable reference
    Variable(String),
    /// Function call
    Call {
        function: String,
        args: Vec<Expr>,
    },
}

/// AST (Abstract Syntax Tree)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ast {
    /// Statements in the program
    pub statements: Vec<Statement>,
}

impl Ast {
    /// Create a new empty AST
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a statement to the AST
    pub fn add_statement(&mut self, stmt: Statement) {
        self.statements.push(stmt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_new() {
        let compiler = Compiler::new();
        assert_eq!(compiler.next_id, 0);
    }

    #[test]
    fn test_ast_new() {
        let ast = Ast::new();
        assert!(ast.statements.is_empty());
    }

    #[test]
    fn test_ast_add_statement() {
        let mut ast = Ast::new();
        ast.add_statement(Statement::Input {
            name: "test".to_string(),
            schema: "string".to_string(),
        });
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_compile_empty() {
        let mut compiler = Compiler::new();
        let ast = Ast::new();
        let result = compiler.compile(&ast);
        assert!(result.is_ok());
        assert!(result.unwrap().dag.is_empty());
    }

    #[test]
    fn test_compile_input() {
        let mut compiler = Compiler::new();
        let mut ast = Ast::new();
        ast.add_statement(Statement::Input {
            name: "data".to_string(),
            schema: "string".to_string(),
        });

        let result = compiler.compile(&ast);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().dag.node_count(), 1);
    }

    #[test]
    fn test_infer_capabilities() {
        let compiler = Compiler::new();
        let caps = compiler.infer_capabilities("read_file", &[]);
        assert_eq!(caps.len(), 1);
        assert!(matches!(&caps[0], Capability::FsRead { .. }));
    }

    #[test]
    fn test_expr_string() {
        let expr = Expr::String("test".to_string());
        assert_eq!(expr, Expr::String("test".to_string()));
    }

    #[test]
    fn test_expr_integer() {
        let expr = Expr::Integer(42);
        assert_eq!(expr, Expr::Integer(42));
    }
}
