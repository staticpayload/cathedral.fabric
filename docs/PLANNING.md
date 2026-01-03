# Planning System Specification

## Overview

The planning system transforms workflow definitions into executable DAGs with explicit resource contracts and capability requirements.

## DSL Syntax

### Basic Workflow

```cathedral
workflow "data_pipeline" {
    version: "1.0.0"
    description: "Fetch, transform, and store data"

    // Resource limits for the workflow
    resources {
        cpu: "1000m"
        memory: "512Mi"
        timeout: "5m"
    }

    // Step definition
    step "fetch_data" {
        tool: "http_fetch"
        input: {
            url: "https://api.example.com/data"
        }
        capabilities: [NetRead { allowlist: ["api.example.com"] }]
    }

    step "transform" depends_on: ["fetch_data"] {
        tool: "transform_json"
        input: {
            data: fetch_data.output
        }
        resources {
            cpu: "500m"
            memory: "256Mi"
        }
    }

    step "store" depends_on: ["transform"] {
        tool: "write_file"
        input: {
            path: "./output/result.json"
            content: transform.output
        }
        capabilities: [FsWrite { prefixes: ["./output"] }]
    }
}
```

### Advanced Features

```cathedral
workflow "parallel_processing" {
    resources {
        cpu: "2000m"
        memory: "1Gi"
    }

    // Parallel steps (no dependency)
    step "fetch_a" {
        tool: "http_fetch"
        input: { url: "https://api.a.com/data" }
    }

    step "fetch_b" {
        tool: "http_fetch"
        input: { url: "https://api.b.com/data" }
    }

    // Join step
    step "merge" depends_on: ["fetch_a", "fetch_b"] {
        tool: "merge_json"
        input: {
            sources: [fetch_a.output, fetch_b.output]
        }
    }

    // Conditional
    step "maybe_process" depends_on: ["merge"] {
        tool: "conditional_transform"
        condition: merge.output.size > 0
        input: { data: merge.output }
    }

    // Loop
    step "batch_process" depends_on: ["maybe_process"] {
        tool: "batch_transform"
        input: { data: maybe_process.output }
        batch_size: 100
    }
}
```

### Policy Binding

```cathedral
workflow "secure_pipeline" {
    policy: "strict_security"

    step "fetch" {
        tool: "http_fetch"
        // Capabilities granted by policy
    }
}
```

## AST Structure

```rust
pub struct WorkflowAst {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub resources: ResourceContract,
    pub steps: Vec<StepDef>,
    pub policy: Option<String>,
}

pub struct StepDef {
    pub name: String,
    pub tool: String,
    pub input: InputExpr,
    pub dependencies: Vec<String>,
    pub capabilities: Vec<Capability>,
    pub resources: Option<ResourceContract>,
    pub condition: Option<Expr>,
    pub retry: Option<RetryPolicy>,
}

pub enum InputExpr {
    Literal(Value),
    Reference { step: String, field: String },
    Array { elements: Vec<InputExpr> },
    Object { fields: BTreeMap<String, InputExpr> },
}

pub enum Expr {
    BinaryOp { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    FieldAccess { base: Box<Expr>, field: String },
    FunctionCall { func: String, args: Vec<Expr> },
    Literal(Value),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
}
```

## DAG Compilation

### Node Types

```rust
pub enum NodeKind {
    /// Tool invocation node
    Tool {
        tool: String,
        input: InputExpr,
        capabilities: Vec<Capability>,
    },

    /// Conditional execution
    Conditional {
        condition: Expr,
        true_branch: NodeId,
        false_branch: Option<NodeId>,
    },

    /// Loop construct
    Loop {
        body: NodeId,
        max_iterations: u64,
    },

    /// Parallel execution
    Parallel {
        branches: Vec<NodeId>,
    },

    /// Input/output nodes
    Input { name: String },
    Output { name: String },
}
```

### DAG Structure

```rust
pub struct Dag {
    pub nodes: BTreeMap<NodeId, Node>,
    pub edges: Vec<Edge>,
    pub entry_nodes: BTreeSet<NodeId>,
    pub exit_nodes: BTreeSet<NodeId>,
}

pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub resources: ResourceContract,
}

pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}

pub enum EdgeKind {
    Data,
    Control,
    Capability,
}
```

### Compiler

```rust
pub struct Compiler {
    tool_registry: ToolRegistry,
    policy_engine: PolicyEngine,
}

impl Compiler {
    pub fn compile(&self, ast: WorkflowAst) -> Result<CompiledWorkflow, CompileError> {
        // 1. Validate AST
        self.validate_ast(&ast)?;

        // 2. Resolve tool references
        let tool_refs = self.resolve_tools(&ast)?;

        // 3. Build DAG
        let mut dag = self.build_dag(&ast, &tool_refs)?;

        // 4. Type check
        self.type_check(&dag)?;

        // 5. Analyze resources
        let resources = self.analyze_resources(&dag)?;

        // 6. Verify capabilities
        self.verify_capabilities(&dag)?;

        // 7. Topological sort for execution order
        let order = self.topological_sort(&dag)?;

        Ok(CompiledWorkflow {
            dag,
            resources,
            execution_order: order,
            capability_sets: self.extract_capabilities(&dag)?,
        })
    }

    fn validate_ast(&self, ast: &WorkflowAst) -> Result<(), CompileError> {
        // Check no circular dependencies
        self.check_no_cycles(&ast.steps)?;

        // Check all dependencies exist
        for step in &ast.steps {
            for dep in &step.dependencies {
                if !ast.steps.iter().any(|s| &s.name == dep) {
                    return Err(CompileError::UndefinedDependency {
                        step: step.name.clone(),
                        dep: dep.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    fn resolve_tools(&self, ast: &WorkflowAst) -> Result<BTreeMap<String, ToolDef>, CompileError> {
        let mut tools = BTreeMap::new();
        for step in &ast.steps {
            if let Some(tool) = self.tool_registry.get(&step.tool) {
                tools.insert(step.name.clone(), tool.clone());
            } else {
                return Err(CompileError::ToolNotFound {
                    step: step.name.clone(),
                    tool: step.tool.clone(),
                });
            }
        }
        Ok(tools)
    }

    fn topological_sort(&self, dag: &Dag) -> Result<Vec<NodeId>, CompileError> {
        let mut sorted = Vec::new();
        let mut visited = BTreeSet::new();
        let mut visiting = BTreeSet::new();

        for node_id in &dag.entry_nodes {
            self.visit(*node_id, &dag, &mut sorted, &mut visited, &mut visiting)?;
        }

        Ok(sorted)
    }

    fn visit(
        &self,
        node_id: NodeId,
        dag: &Dag,
        sorted: &mut Vec<NodeId>,
        visited: &mut BTreeSet<NodeId>,
        visiting: &mut BTreeSet<NodeId>,
    ) -> Result<(), CompileError> {
        if visiting.contains(&node_id) {
            return Err(CompileError::CycleDetected);
        }
        if visited.contains(&node_id) {
            return Ok(());
        }

        visiting.insert(node_id);

        for edge in dag.edges.iter().filter(|e| e.from == node_id) {
            self.visit(edge.to, dag, sorted, visited, visiting)?;
        }

        visiting.remove(&node_id);
        visited.insert(node_id);
        sorted.push(node_id);
        Ok(())
    }
}
```

## Resource Contracts

```rust
pub struct ResourceContract {
    pub cpu: CpuResource,
    pub memory: MemoryResource,
    pub timeout: Duration,
    pub retry: Option<RetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuResource {
    pub millicores: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResource {
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub retry_on: Vec<ErrorKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed { duration: Duration },
    Exponential { base: Duration, max: Duration },
    Linear { increment: Duration },
}
```

## Capability Inference

```rust
impl Compiler {
    fn infer_capabilities(&self, step: &StepDef, tool: &ToolDef) -> Vec<Capability> {
        let mut caps = step.capabilities.clone();

        // Add tool's required capabilities
        caps.extend(tool.required_capabilities.clone());

        // Infer from input expressions
        self.infer_from_input(&step.input, &mut caps);

        caps
    }

    fn infer_from_input(&self, expr: &InputExpr, caps: &mut Vec<Capability>) {
        match expr {
            InputExpr::Reference { step, .. } => {
                // Dependency on another step
            }
            InputExpr::Array { elements } => {
                for el in elements {
                    self.infer_from_input(el, caps);
                }
            }
            InputExpr::Object { fields } => {
                for expr in fields.values() {
                    self.infer_from_input(expr, caps);
                }
            }
            InputExpr::Literal(_) => {}
        }
    }
}
```

## Validation

### Type Checking

```rust
pub struct TypeChecker {
    schemas: BTreeMap<String, (InputSchema, OutputSchema)>,
}

impl TypeChecker {
    pub fn check(&self, dag: &Dag) -> Result<(), TypeError> {
        for node_id in &dag.entry_nodes {
            self.check_node(*node_id, dag)?;
        }
        Ok(())
    }

    fn check_node(&self, node_id: NodeId, dag: &Dag) -> Result<(), TypeError> {
        let node = &dag.nodes[&node_id];

        match &node.kind {
            NodeKind::Tool { tool, input, .. } => {
                // Check input matches tool schema
                if let Some((input_schema, _)) = self.schemas.get(tool) {
                    self.validate_input(input, input_schema)?;
                }
            }
            _ => {}
        }

        // Check dependencies
        for edge in dag.edges.iter().filter(|e| e.from == node_id) {
            self.check_edge(edge, dag)?;
        }

        Ok(())
    }
}
```

## Examples

### Example 1: Simple Pipeline

```cathedral
workflow "etl" {
    step "extract" {
        tool: "db_query"
        input: { query: "SELECT * FROM users" }
        capabilities: [DbRead { tables: ["users"] }]
    }

    step "transform" depends_on: ["extract"] {
        tool: "transform"
        input: { data: extract.output }
    }

    step "load" depends_on: ["transform"] {
        tool: "write_file"
        input: { path: "output.json", content: transform.output }
        capabilities: [FsWrite { prefixes: ["."] }]
    }
}
```

### Example 2: Parallel Fanout

```cathedral
workflow "fanout" {
    step "fetch" {
        tool: "http_fetch"
        input: { url: "https://api.example.com/items" }
    }

    // Parallel processing of items
    step "process_1" depends_on: ["fetch"] {
        tool: "process_item"
        input: { item: fetch.output[0] }
    }

    step "process_2" depends_on: ["fetch"] {
        tool: "process_item"
        input: { item: fetch.output[1] }
    }

    step "process_3" depends_on: ["fetch"] {
        tool: "process_item"
        input: { item: fetch.output[2] }
    }

    step "aggregate" depends_on: ["process_1", "process_2", "process_3"] {
        tool: "aggregate"
        input: { results: [process_1.output, process_2.output, process_3.output] }
    }
}
```
