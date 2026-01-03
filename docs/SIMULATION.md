# Deterministic Simulation

## Overview

The simulation framework provides deterministic, reproducible testing of cluster behavior under various failure conditions.

## Principles

1. **Deterministic by default** - Same seed → same results
2. **Recorded seeds** - Failing simulations become regression tests
3. **Fast execution** - No real waiting or timeouts
4. **Comprehensive coverage** - Test all failure modes

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                      Simulation Harness                        │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Seed Source                                             │  │
│  │  - Fixed seed for reproducibility                        │  │
│  │  - Random seed for exploration                           │  │
│  └──────────────┬───────────────────────────────────────────┘  │
│                 │                                               │
│  ┌──────────────┴───────────────────────────────────────────┐  │
│  │  Simulated Cluster                                       │  │
│  │  - SimNodes (coordinator, workers)                       │  │
│  │  - Simulated network                                     │  │
│  │  - Simulated storage                                     │  │
│  └──────────────┬───────────────────────────────────────────┘  │
│                 │                                               │
│  ┌──────────────┴───────────────────────────────────────────┐  │
│  │  Failure Injector                                       │  │
│  │  - Network conditions                                   │  │
│  │  - Node crashes                                         │  │
│  │  - Clock skew                                           │  │
│  └──────────────┬───────────────────────────────────────────┘  │
│                 │                                               │
│  ┌──────────────┴───────────────────────────────────────────┐  │
│  │  Result Recorder                                        │  │
│  │  - Seed + outcome                                       │  │
│  │  - Trace log                                            │  │
│  │  - Assertions                                           │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

## Seed Management

```rust
pub struct SimSeed {
    pub value: u64,
    pub source: SeedSource,
}

pub enum SeedSource {
    Fixed { name: String },
    Random,
    Recorded { run_id: String },
}

impl SimSeed {
    pub fn fixed(value: u64) -> Self {
        Self {
            value,
            source: SeedSource::Fixed {
                name: format!("seed_{:x}", value),
            },
        }
    }

    pub fn random() -> Self {
        Self {
            value: rand::random(),
            source: SeedSource::Random,
        }
    }

    pub fn into_rng(self) -> StdRng {
        StdRng::seed_from_u64(self.value)
    }
}
```

## Network Simulation

```rust
pub struct NetworkSim {
    rng: StdRng,
    condition: NetworkCondition,
}

#[derive(Clone, Copy)]
pub struct NetworkCondition {
    pub latency: Duration,
    pub jitter: Duration,
    pub packet_loss: f64,
    pub bandwidth: Option<u64>,  // bytes/sec
}

impl NetworkSim {
    pub fn deliver(&mut self, packet: Packet) -> Result<(), NetworkError> {
        // Check packet loss
        if self.rng.gen_bool(self.condition.packet_loss) {
            return Err(NetworkError::PacketLost);
        }

        // Calculate latency
        let base = self.condition.latency;
        let jitter = if self.condition.jitter.is_zero() {
            Duration::ZERO
        } else {
            let ms = self.rng.gen_range(0..self.condition.jitter.as_millis() as u64);
            Duration::from_millis(ms)
        };

        // Simulate delivery delay (in sim time, not real time)
        self.schedule_delivery(packet, base + jitter)
    }
}
```

## Failure Injection

```rust
pub struct FailureInjector {
    rng: StdRng,
    schedule: Vec<ScheduledFailure>,
}

pub struct ScheduledFailure {
    pub sim_time: SimTime,
    pub failure: FailureKind,
}

pub enum FailureKind {
    Partition {
        nodes: Vec<NodeId>,
        duration: SimDuration,
    },
    Crash {
        node: NodeId,
        restart_after: Option<SimDuration>,
    },
    Delay {
        node: NodeId,
        duration: SimDuration,
    },
    CorruptSnapshot {
        node: NodeId,
    },
    ClockSkew {
        node: NodeId,
        skew: SimDuration,
    },
}

impl FailureInjector {
    pub fn generate_from_seed(
        &mut self,
        seed: u64,
        nodes: &[NodeId],
        sim_duration: SimDuration,
    ) {
        self.rng = StdRng::seed_from_u64(seed);
        self.schedule = Vec::new();

        // Generate random failures throughout simulation
        let mut time = SimTime::ZERO;
        while time < sim_duration {
            if let Some(failure) = self.random_failure(nodes) {
                time += self.random_delay();
                self.schedule.push(ScheduledFailure {
                    sim_time: time,
                    failure,
                });
            }
        }
    }
}
```

## Simulated Node

```rust
pub struct SimNode {
    pub id: NodeId,
    pub role: NodeRole,
    pub state: SimNodeState,
    pub clock: SimClock,
    pub network: NetworkHandle,
    pub storage: SimStorage,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Coordinator,
    Worker,
}

pub enum SimNodeState {
    Running,
    Crashed,
    Delayed(SimTime),
    Partitioned,
}

impl SimNode {
    pub fn process_message(&mut self, msg: Message) -> SimResult<Vec<Message>> {
        match &self.state {
            SimNodeState::Running => {
                self.handle_message(msg)
            }
            SimNodeState::Crashed => {
                Ok(Vec::new())  // Drop all messages
            }
            SimNodeState::Delayed(until) => {
                if self.clock.now() < *until {
                    Ok(Vec::new())  // Buffer messages
                } else {
                    self.state = SimNodeState::Running;
                    self.handle_message(msg)
                }
            }
            SimNodeState::Partitioned => {
                Ok(Vec::new())  // Drop all messages
            }
        }
    }
}
```

## Simulation Harness

```rust
pub struct SimHarness {
    nodes: BTreeMap<NodeId, SimNode>,
    network: NetworkSim,
    injector: FailureInjector,
    clock: SimClock,
    messages: VecDeque<PendingMessage>,
    results: SimResults,
}

pub struct SimConfig {
    pub num_coordinators: usize,
    pub num_workers: usize,
    pub duration: SimDuration,
    pub seed: SimSeed,
}

impl SimHarness {
    pub fn new(config: SimConfig) -> Self {
        let rng = config.seed.into_rng();
        let mut harness = Self {
            nodes: BTreeMap::new(),
            network: NetworkSim::new(rng),
            injector: FailureInjector::new(),
            clock: SimClock::new(),
            messages: VecDeque::new(),
            results: SimResults::new(),
        };

        // Create nodes
        for i in 0..config.num_coordinators {
            let node = SimNode::coordinator(i);
            harness.nodes.insert(node.id, node);
        }
        for i in 0..config.num_workers {
            let node = SimNode::worker(i);
            harness.nodes.insert(node.id, node);
        }

        // Schedule failures
        harness.injector.generate_from_seed(
            config.seed.value,
            &harness.nodes.keys().copied().collect::<Vec<_>>(),
            config.duration,
        );

        harness
    }

    pub async fn run(&mut self) -> SimResult<()> {
        while self.clock.now() < self.config.duration {
            // Process scheduled failures
            self.process_failures()?;

            // Deliver pending messages
            self.deliver_messages()?;

            // Advance clock
            self.clock.tick();

            // Check invariants
            self.check_invariants()?;
        }

        Ok(())
    }

    fn check_invariants(&self) -> SimResult<()> {
        // Single leader invariant
        let leaders = self.nodes.values()
            .filter(|n| n.is_leader())
            .count();
        if leaders > 1 {
            return Err(SimError::MultipleLeaders { count: leaders });
        }

        // Hash chain invariant
        for node in self.nodes.values() {
            node.verify_hash_chain()?;
        }

        Ok(())
    }
}
```

## Recording Failures

```rust
pub struct SimRecorder {
    runs: Vec<RecordedRun>,
}

#[derive(Serialize, Deserialize)]
pub struct RecordedRun {
    pub seed: u64,
    pub config: SimConfig,
    pub result: SimResultKind,
    pub trace: Vec<TraceEvent>,
    pub failures: Vec<ScheduledFailure>,
}

pub enum SimResultKind {
    Passed,
    Failed { reason: String },
    Inconclusive,
}

impl SimRecorder {
    pub fn record(&mut self, run: RecordedRun) {
        self.runs.push(run);
    }

    pub fn save_regression_tests(&self, path: &Path) -> Result<(), IoError> {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &self.runs)?;
        Ok(())
    }

    pub fn load_regression_tests(path: &Path) -> Result<Vec<RecordedRun>, IoError> {
        let file = File::open(path)?;
        let runs: Vec<RecordedRun> = serde_json::from_reader(file)?;
        Ok(runs)
    }
}
```

## Property-Based Testing

```rust
#[proptest]
fn test_sim_deterministic(seed: u64, config: SimConfig) {
    let mut sim1 = SimHarness::with_seed(seed, config.clone());
    let mut sim2 = SimHarness::with_seed(seed, config);

    sim1.run().unwrap();
    sim2.run().unwrap();

    assert_eq!(sim1.results(), sim2.results());
}

#[proptest]
fn test_replay_from_trace(seed: u64) {
    let mut sim = SimHarness::with_seed(seed, SimConfig::default());
    sim.run().unwrap();

    let trace = sim.take_trace();

    let mut replay = SimReplay::from_trace(trace);
    replay.run().unwrap();

    assert_eq!(sim.results(), replay.results());
}
```

## Fuzzing

```rust
pub fn fuzz_sim_configurations() {
    let mut corpus = FuzzCorpus::new();

    loop {
        // Generate random config
        let config = random_sim_config();

        // Run simulation
        let result = run_sim_with_config(config.clone());

        // Check for failures
        if result.is_err() {
            corpus.add(config, result);
        }

        // Minimize failing cases
        corpus.minimize();
    }
}
```

## CLI Usage

```bash
# Run single simulation with fixed seed
cathedral sim --seed 12345 --config sim-config.json

# Run many simulations
cathedral sim --count 10000 --output results.json

# Run regression tests
cathedral sim regressions --test-cases recorded-failures.json

# Explore random configurations
cathedral sim explore --duration 1000
```

## CI Integration

```yaml
name: sim-short

on: [push, pull_request]

jobs:
  sim:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
      - name: Run 100 deterministic sims
        run: |
          cathedral sim --count 100 --seeds seeds.txt
      - name: Check results
        run: |
          cathedral sim verify-results results.json
```

## Metrics

```rust
pub struct SimMetrics {
    pub sim_duration: SimDuration,
    pub real_duration: Duration,
    pub events_processed: usize,
    pub messages_sent: usize,
    pub failures_injected: usize,
    pub invariants_checked: usize,
}
```
