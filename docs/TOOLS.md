# Tool System Specification

## Overview

The CATHEDRAL tool system provides a secure interface for executing external operations. All tools are treated as potentially hostile and must declare their requirements explicitly.

## Tool Interface

### Core Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool identifier (must be unique)
    fn name(&self) -> &str;

    /// Tool version (semver)
    fn version(&self) -> &str;

    /// Input schema
    fn input_schema(&self) -> &InputSchema;

    /// Output schema
    fn output_schema(&self) -> &OutputSchema;

    /// Required capabilities
    fn required_capabilities(&self) -> &[Capability];

    /// Declared side effects
    fn side_effects(&self) -> &[SideEffect];

    /// Determinism declaration
    fn determinism(&self) -> DeterminismLevel;

    /// Resource bounds
    fn resource_bounds(&self) -> &ResourceBounds;

    /// Execute the tool
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
}
```

## Schemas

### Input Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSchema {
    pub parameters: BTreeMap<String, Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub param_type: ParameterType,
    pub required: bool,
    pub description: String,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Object { schema: Box<InputSchema> },
    Array { item_type: Box<ParameterType> },
    OneOf { variants: Vec<ParameterType> },
}
```

### Output Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSchema {
    pub result_type: ResultType,
    pub fields: BTreeMap<String, FieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultType {
    Object,
    Array,
    String,
    Binary,
    Stream,
}
```

## Side Effects

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SideEffect {
    /// Reads from network
    NetRead,

    /// Writes to network
    NetWrite,

    /// Reads from filesystem
    FsRead,

    /// Writes to filesystem
    FsWrite,

    /// Reads from database
    DbRead,

    /// Writes to database
    DbWrite,

    /// Spawns subprocess
    ProcessSpawn,

    /// No observable side effects (pure function)
    None,
}
```

## Determinism Levels

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeterminismLevel {
    /// Always produces same output for same input
    Pure,

    /// Deterministic within a run (uses logical time)
    Deterministic,

    /// May produce different output (external data source)
    Maybe,

    /// Non-deterministic (uses randomness, real time, etc.)
    NonDeterministic,
}
```

## Resource Bounds

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBounds {
    pub timeout: Duration,
    pub max_memory: Option<u64>,
    pub max_cpu: Option<String>,
    pub max_network: Option<NetworkBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkBounds {
    pub max_requests: u32,
    pub max_bytes: u64,
}
```

## Tool Registry

```rust
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) -> Result<(), RegistryError> {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            return Err(RegistryError::AlreadyRegistered(name));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|k| k.as_str()).collect()
    }
}
```

## Tool Execution

```rust
pub struct ToolExecutor {
    registry: ToolRegistry,
    policy_engine: PolicyEngine,
    capability_gate: CapabilityGate,
}

impl ToolExecutor {
    pub async fn execute(
        &self,
        tool_name: &str,
        input: ToolInput,
        capabilities: &CapabilitySet,
    ) -> Result<ToolResult, ExecutionError> {
        // 1. Get tool
        let tool = self.registry.get(tool_name)
            .ok_or(ExecutionError::ToolNotFound(tool_name.to_string()))?;

        // 2. Validate input against schema
        tool.input_schema().validate(&input)?;

        // 3. Check capabilities
        for required in tool.required_capabilities() {
            self.capability_gate.check(required)?;
        }

        // 4. Check policy
        let context = MatchContext::tool(tool_name);
        let proof = self.policy_engine.decide(&context);
        if !proof.allowed {
            return Err(ExecutionError::PolicyDenied { proof });
        }

        // 5. Execute
        let output = tool.execute(input).await?;

        // 6. Validate output
        tool.output_schema().validate(&output)?;

        // 7. Normalize
        let normalized = self.normalize(tool_name, &output)?;

        // 8. Return
        Ok(ToolResult {
            raw: output,
            normalized,
            proof,
            side_effects: tool.side_effects().to_vec(),
        })
    }
}
```

## Normalization

Tool output is normalized to ensure deterministic hashing:

```rust
pub struct Normalizer {
    rules: BTreeMap<String, NormalizationRule>,
}

impl Normalizer {
    pub fn normalize(&self, tool_name: &str, output: &ToolOutput) -> Result<NormalizedOutput> {
        let rule = self.rules.get(tool_name)
            .ok_or(NormalizationError::NoRule(tool_name.to_string()))?;

        match rule {
            NormalizationRule::Json => {
                // Parse, sort keys, re-encode
                let value: Value = serde_json::from_slice(&output.raw)?;
                let normalized = serde_json::to_vec(&value)?;
                Ok(NormalizedOutput {
                    data: normalized,
                    hash: Hash::compute(&normalized),
                })
            }
            NormalizationRule::Binary => {
                // Just hash
                Ok(NormalizedOutput {
                    data: output.raw.clone(),
                    hash: Hash::compute(&output.raw),
                })
            }
            NormalizationRule::Custom(normalizer) => {
                normalizer(output)
            }
        }
    }
}
```

## Tool Adapters

Tools can be loaded from different sources:

```rust
pub enum ToolSource {
    /// Built-in tool (Rust)
    BuiltIn(Box<dyn Tool>),

    /// WASM module
    Wasm(PathBuf),

    /// HTTP endpoint
    Http { url: String },

    /// Subprocess
    Subprocess { command: String, args: Vec<String> },
}
```

## Built-in Tools

### HttpFetch

```rust
pub struct HttpFetch;

#[async_trait]
impl Tool for HttpFetch {
    fn name(&self) -> &str { "http_fetch" }
    fn version(&self) -> &str { "1.0.0" }

    fn input_schema(&self) -> &InputSchema {
        static SCHEMA: InputSchema = InputSchema {
            parameters: maplit::btreemap! {
                "url" => Parameter {
                    param_type: ParameterType::String,
                    required: true,
                    description: "URL to fetch".to_string(),
                    default: None,
                },
                "headers" => Parameter {
                    param_type: ParameterType::Array {
                        item_type: Box::new(ParameterType::String),
                    },
                    required: false,
                    description: "HTTP headers".to_string(),
                    default: None,
                },
            },
        };
        &SCHEMA
    }

    fn required_capabilities(&self) -> &[Capability] {
        static CAPS: once_cell::sync::Lazy<Vec<Capability>> =
            once_cell::sync::Lazy::new(|| vec![
                Capability::NetRead {
                    allowlist: vec!["*".to_string()],  // Policy filters this
                },
            ]);
        &CAPS
    }

    fn side_effects(&self) -> &[SideEffect] {
        static EFFECTS: once_cell::sync::Lazy<Vec<SideEffect>> =
            once_cell::sync::Lazy::new(|| vec![SideEffect::NetRead]);
        &EFFECTS
    }

    fn determinism(&self) -> DeterminismLevel {
        DeterminismLevel::Maybe  // External data may change
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let url: String = input.get("url")?;
        // Execute fetch...
        Ok(ToolOutput {
            raw: serde_json::to_vec(&response)?,
        })
    }
}
```

## Security

### Validation

1. Input validation against schema
2. Output validation against schema
3. Capability checks before execution
4. Policy approval before execution

### Isolation

1. Built-in tools: Rust memory safety
2. WASM tools: Fuel limits, memory limits, capability-mediated host calls
3. Subprocess tools: Resource limits, timeouts

### Auditing

All tool executions logged:
```
ToolInvoked → CapabilityCheck → PolicyDecision → ToolCompleted
```

## Example Tool Definition

```rust
pub struct ReadFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &str { "read_file" }
    fn version(&self) -> &str { "1.0.0" }

    fn input_schema(&self) -> &InputSchema {
        static SCHEMA: InputSchema = InputSchema {
            parameters: maplit::btreemap! {
                "path" => Parameter {
                    param_type: ParameterType::String,
                    required: true,
                    description: "File path to read".to_string(),
                    default: None,
                },
            },
        };
        &SCHEMA
    }

    fn required_capabilities(&self) -> &[Capability] {
        &[
            Capability::FsRead {
                prefixes: vec![],  // Policy filters
            },
        ]
    }

    fn side_effects(&self) -> &[SideEffect] {
        &[SideEffect::FsRead]
    }

    fn determinism(&self) -> DeterminismLevel {
        DeterminismLevel::Deterministic  // Same path → same content
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let path: String = input.get("path")?;
        let content = tokio::fs::read_to_string(&path).await?;
        Ok(ToolOutput {
            raw: serde_json::to_vec(&json!({ "content": content }))?,
        })
    }
}
```
