# WASM Sandbox Specification

## Overview

The WASM sandbox provides deterministic, isolated execution of tools with strict resource limits.

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         WASM Sandbox                           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Wasmtime Engine                                         │  │
│  │  - Cranelift compiler (deterministic)                    │  │
│  │  - Fuel metering                                         │  │
│  │  - Memory limits                                         │  │
│  └────────────┬─────────────────────────────────────────────┘  │
│               │                                                  │
│  ┌────────────┴─────────────────────────────────────────────┐  │
│  │  Host Functions (Capability-Mediated)                    │  │
│  │  - cathedral_read (logged)                               │  │
│  │  - cathedral_write (logged)                              │  │
│  │  - cathedral_fetch (logged, capability-gated)            │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

## Deterministic ABI

All host calls are deterministic and logged:

```rust
pub struct HostCall {
    pub name: String,
    pub inputs: Vec<u8>,
    pub outputs: Vec<u8>,
    pub fuel_consumed: u64,
    pub capability_check: CapabilityCheck,
    pub timestamp: LogicalTime,
}
```

## Fuel Limits

### Fuel Meter

```rust
pub struct FuelMeter {
    remaining: u64,
    initial: u64,
}

impl FuelMeter {
    pub fn new(fuel: u64) -> Self {
        Self {
            remaining: fuel,
            initial: fuel,
        }
    }

    pub fn consume(&mut self, amount: u64) -> Result<(), FuelError> {
        if amount > self.remaining {
            return Err(FuelError::OutOfFuel {
                requested: amount,
                remaining: self.remaining,
            });
        }
        self.remaining -= amount;
        Ok(())
    }

    pub fn check(&self) -> Result<(), FuelError> {
        if self.remaining == 0 {
            return Err(FuelError::OutOfFuel {
                requested: 1,
                remaining: 0,
            });
        }
        Ok(())
    }
}
```

### Fuel Allocation

```rust
pub const DEFAULT_FUEL_LIMIT: u64 = 10_000_000_000;  // ~10B instructions

pub struct FuelConfig {
    pub base_limit: u64,
    pub per_instruction_cost: u64,
    pub host_call_multiplier: u64,
}

impl FuelConfig {
    pub fn standard() -> Self {
        Self {
            base_limit: DEFAULT_FUEL_LIMIT,
            per_instruction_cost: 1,
            host_call_multiplier: 1000,  // Host calls are expensive
        }
    }

    pub fn quick() -> Self {
        Self {
            base_limit: 1_000_000_000,  // 1B instructions
            per_instruction_cost: 1,
            host_call_multiplier: 1000,
        }
    }

    pub fn generous() -> Self {
        Self {
            base_limit: 100_000_000_000,  // 100B instructions
            per_instruction_cost: 1,
            host_call_multiplier: 1000,
        }
    }
}
```

## Memory Limits

```rust
pub struct MemoryLimit {
    max_pages: u32,  // Wasm pages (64KB each)
    current_pages: u32,
}

impl MemoryLimit {
    pub fn new(max_mb: u64) -> Self {
        let max_pages = (max_mb * 1024 * 1024 / 65536) as u32;
        Self {
            max_pages,
            current_pages: 0,
        }
    }

    pub fn allocate(&mut self, pages: u32) -> Result<(), MemoryError> {
        let new_total = self.current_pages.saturating_add(pages);
        if new_total > self.max_pages {
            return Err(MemoryError::OutOfMemory {
                requested: pages,
                available: self.max_pages - self.current_pages,
            });
        }
        self.current_pages = new_total;
        Ok(())
    }

    pub fn max_bytes(&self) -> u64 {
        self.max_pages as u64 * 65536
    }
}
```

## Host Functions

### Canonical Host ABI

All host functions follow this pattern:

```rust
pub trait HostFunction: Send + Sync {
    /// Function name as exported to WASM
    fn name(&self) -> &str;

    /// Required capability
    fn required_capability(&self) -> Option<Capability>;

    /// Execute the host call
    fn execute(
        &self,
        ctx: &mut HostContext,
        input: &[u8],
    ) -> Result<Vec<u8>, HostError>;
}

pub struct HostContext {
    pub logical_time: LogicalTime,
    pub capability_gate: CapabilityGate,
    pub logger: EventLogger,
}
```

### Standard Host Functions

#### cathedral_log

```rust
pub struct LogHostFn;

impl HostFunction for LogHostFn {
    fn name(&self) -> &str { "cathedral_log" }

    fn required_capability(&self) -> Option<Capability> {
        None  // Logging is always allowed
    }

    fn execute(&self, ctx: &mut HostContext, input: &[u8]) -> Result<Vec<u8>, HostError> {
        let message = String::from_utf8(input.to_vec())
            .map_err(|_| HostError::InvalidInput)?;

        // Log as event
        ctx.logger.log(message.clone());

        Ok(message.into_bytes())
    }
}
```

#### cathedral_read

```rust
pub struct ReadHostFn;

impl HostFunction for ReadHostFn {
    fn name(&self) -> &str { "cathedral_read" }

    fn required_capability(&self) -> Option<Capability> {
        Some(Capability::FsRead {
            prefixes: vec![PathBuf::from("./")],  // Policy filters
        })
    }

    fn execute(&self, ctx: &mut HostContext, input: &[u8]) -> Result<Vec<u8>, HostError> {
        let path = String::from_utf8(input.to_vec())
            .map_err(|_| HostError::InvalidInput)?;

        // Check capability
        ctx.capability_gate.require_fs_read(&path)?;

        // Log the read
        ctx.logger.log_capability_check("fs_read", &path, true);

        // Perform read
        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| HostError::Io(e))?;

        Ok(content.into_bytes())
    }
}
```

#### cathedral_fetch

```rust
pub struct FetchHostFn;

impl HostFunction for FetchHostFn {
    fn name(&self) -> &str { "cathedral_fetch" }

    fn required_capability(&self) -> Option<Capability> {
        Some(Capability::NetRead {
            allowlist: vec!["*".to_string()],  // Policy filters
        })
    }

    fn execute(&self, ctx: &mut HostContext, input: &[u8]) -> Result<Vec<u8>, HostError> {
        let url = String::from_utf8(input.to_vec())
            .map_err(|_| HostError::InvalidInput)?;

        // Extract domain
        let domain = extract_domain(&url)?;

        // Check capability
        ctx.capability_gate.require_net_read(&domain)?;

        // Log the fetch
        ctx.logger.log_capability_check("net_read", &domain, true);

        // Perform fetch
        let response = fetch_url(&url).await
            .map_err(|e| HostError::Fetch(e))?;

        Ok(response.into_bytes())
    }
}
```

## Sandbox Configuration

```rust
pub struct SandboxConfig {
    pub fuel: u64,
    pub memory_mb: u64,
    pub host_functions: HostRegistry,
    pub timeout: Duration,
}

impl SandboxConfig {
    pub fn standard() -> Self {
        Self {
            fuel: 10_000_000_000,
            memory_mb: 64,
            host_functions: HostRegistry::standard(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn minimal() -> Self {
        Self {
            fuel: 1_000_000_000,
            memory_mb: 16,
            host_functions: HostRegistry::minimal(),
            timeout: Duration::from_secs(5),
        }
    }

    pub fn generous() -> Self {
        Self {
            fuel: 100_000_000_000,
            memory_mb: 256,
            host_functions: HostRegistry::with_all(),
            timeout: Duration::from_secs(300),
        }
    }
}
```

## Sandbox Execution

```rust
pub struct Sandbox {
    engine: wasmtime::Engine,
    module: wasmtime::Module,
    config: SandboxConfig,
}

impl Sandbox {
    pub fn new(wasm: &[u8], config: SandboxConfig) -> Result<Self, SandboxError> {
        let mut engine_config = wasmtime::Config::new();
        engine_config.wasm_simd(true);
        engine_config.consume_fuel(true);
        engine_config.static_memory_maximum_size(0);  // Force dynamic
        engine_config.dynamic_memory_guard_size(65536);

        let engine = wasmtime::Engine::new(&engine_config)?;
        let module = wasmtime::Module::new(&engine, wasm)?;

        Ok(Self { engine, module, config })
    }

    pub async fn execute(&self, input: &[u8]) -> Result<SandboxOutput, SandboxError> {
        let mut store = wasmtime::Store::new(&self.engine, HostState::new());
        store.add_fuel(self.config.fuel)
            .map_err(|_| SandboxError::FuelError)?;

        // Set memory limit via memory limiter
        store.limiter(|state| &mut state.memory_limiter);

        // Link host functions
        let mut linker = wasmtime::Linker::new(&self.engine);
        self.config.host_functions.register(&mut linker)?;

        // Instantiate
        let instance = linker.instantiate(&mut store, &self.module)?;

        // Run with timeout
        tokio::time::timeout(self.config.timeout, async {
            let func = instance.get_func(&mut store, "run")
                .ok_or(SandboxError::NoRunFunction)?;

            // Call the function
            func.call(&mut store, &[input.into()], &mut result)
        })
        .await
        .map_err(|_| SandboxError::Timeout)?
    }
}
```

## WASM Compilation

```rust
pub struct WasmCompiler {
    config: CompileConfig,
}

pub struct CompileConfig {
    pub optimize: bool,
    pub debug_info: bool,
}

impl WasmCompiler {
    pub fn compile_wat(&self, wat: &str) -> Result<Vec<u8>, CompileError> {
        let wasm = wat::parse_str(wat)
            .map_err(|e| CompileError::WatParse(e))?;
        Ok(wasm)
    }

    pub fn validate(&self, wasm: &[u8]) -> Result<(), CompileError> {
        // Validate WASM
        let mut config = wasmtime::Config::new();
        config.wasm_simd(true);
        config.consume_fuel(true);

        let engine = wasmtime::Engine::new(&config)?;
        let _module = wasmtime::Module::new(&engine, wasm)?;

        Ok(())
    }
}
```

## Example WASM Tool

```wat
(module
  (import "cathedral" "cathedral_log"
    (func $log (param i32 i32)))

  (import "cathedral" "cathedral_read"
    (func $read (param i32 i32) (result i32)))

  (memory (export "memory") 1)

  (func (export "run") (param i32 i32) (result i32)
    (local $input i32)
    (local $len i32)

    local.get 0
    local.set $input
    local.get 1
    local.set $len

    ;; Log something
    (i32.const 100)  ;; pointer
    (i32.const 5)    ;; length
    call $log

    ;; Return success
    (i32.const 0))
)
```

## Security Properties

1. **Fuel limits** - Prevent infinite loops
2. **Memory limits** - Prevent memory bombs
3. **Capability gating** - All host calls checked
4. **No ambient authority** - No default filesystem/network
5. **Deterministic** - Same inputs → same outputs
6. **Logged** - All host calls recorded
