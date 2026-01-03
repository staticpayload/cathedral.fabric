# Scheduling System Specification

## Overview

The scheduler is responsible for assigning DAG nodes to workers with deterministic ordering. All scheduling decisions are logged as events.

## Principles

1. **Deterministic assignment** - Same state → same assignment
2. **Explicit decisions** - Every assignment is an event
3. **Resource awareness** - Respects CPU, memory limits
4. **Dependency respect** - Never schedules before dependencies complete
5. **Backpressure handling** - Queue limits, rejection when overwhelmed

## Scheduler Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                        Scheduler                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Ready Queue (topological order)                        │  │
│  └───────────────┬──────────────────────────────────────────┘  │
│                 │                                               │
│  ┌──────────────┴──────────────────────────────────────────┐  │
│  │  Assignment Logic                                       │  │
│  │  - Worker selection                                     │  │
│  │  - Resource checking                                    │  │
│  │  - Capability verification                              │  │
│  └───────────────┬──────────────────────────────────────────┘  │
│                  │                                               │
│  ┌───────────────┴──────────────────────────────────────────┐  │
│  │  Event Emitter                                          │  │
│  │  - TaskAssigned events                                  │  │
│  │  - Assignment logged with reasoning                     │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

## Scheduling Decision

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDecision {
    pub task_id: TaskId,
    pub node_id: NodeId,
    pub worker_id: WorkerId,
    pub assigned_at: LogicalTime,
    pub reasoning: ScheduleReasoning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleReasoning {
    OnlyWorkerAvailable { worker_id: WorkerId },
    LeastLoaded {
        worker_id: WorkerId,
        queue_depth: usize,
    },
    ResourceFit {
        worker_id: WorkerId,
        available_cpu: u64,
        required_cpu: u64,
    },
    Affinity {
        worker_id: WorkerId,
        reason: AffinityReason,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AffinityReason {
    DataLocality,
    PreviouslyExecuted,
    SameZone,
}
```

## Scheduler Implementation

```rust
pub struct Scheduler {
    workers: BTreeMap<WorkerId, WorkerState>,
    ready_queue: VecDeque<NodeId>,
    completed: BTreeSet<NodeId>,
    dag: Dag,
    config: SchedulerConfig,
}

pub struct SchedulerConfig {
    pub max_queue_per_worker: usize,
    pub prefer_locality: bool,
    pub balance_strategy: BalanceStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum BalanceStrategy {
    RoundRobin,
    LeastLoaded,
    Random,
    Affinity,
}

impl Scheduler {
    pub fn new(dag: Dag, config: SchedulerConfig) -> Self {
        let ready_queue = Self::compute_ready_nodes(&dag);
        Self {
            workers: BTreeMap::new(),
            ready_queue,
            completed: BTreeSet::new(),
            dag,
            config,
        }
    }

    pub fn add_worker(&mut self, worker: WorkerState) {
        self.workers.insert(worker.id, worker);
    }

    pub fn next_decision(&mut self) -> Option<ScheduleDecision> {
        // Find next ready node
        let node_id = self.ready_queue.front()?.clone();

        // Find available workers
        let available = self.find_available_workers(&node_id)?;

        // Select worker deterministically
        let worker_id = self.select_worker(&node_id, &available)?;

        // Create decision
        let decision = ScheduleDecision {
            task_id: TaskId::new(),
            node_id,
            worker_id,
            assigned_at: LogicalTime::new(self.completed.len() as u64),
            reasoning: self.build_reasoning(&node_id, worker_id, &available),
        };

        // Update queue
        self.ready_queue.pop_front();

        Some(decision)
    }

    pub fn mark_completed(&mut self, node_id: NodeId) {
        self.completed.insert(node_id);

        // Add newly ready nodes
        let newly_ready = self.find_newly_ready(&node_id);
        self.ready_queue.extend(newly_ready);
    }

    fn find_available_workers(&self, node_id: &NodeId) -> Option<Vec<WorkerId>> {
        let node = &self.dag.nodes[node_id];

        self.workers
            .values()
            .filter(|w| w.is_available())
            .filter(|w| w.has_resources(&node.resources))
            .filter(|w| w.has_capabilities(&node.required_capabilities()))
            .map(|w| w.id)
            .collect::<Vec<_>>()
            .into()
    }

    fn select_worker(&self, node_id: &NodeId, workers: &[WorkerId]) -> Option<WorkerId> {
        if workers.is_empty() {
            return None;
        }

        match self.config.balance_strategy {
            BalanceStrategy::RoundRobin => {
                let index = self.completed.len() % workers.len();
                Some(workers[index])
            }
            BalanceStrategy::LeastLoaded => {
                workers
                    .iter()
                    .min_by_key(|id| {
                        self.workers.get(id).map(|w| w.queue_depth()).unwrap_or(0)
                    })
                    .copied()
            }
            BalanceStrategy::Affinity => {
                // Check for data locality
                self.find_affinity_worker(node_id, workers)
            }
            BalanceStrategy::Random => {
                // Deterministic "random"
                let index = self.hash_node_and_workers(node_id, workers);
                Some(workers[index])
            }
        }
    }

    fn hash_node_and_workers(&self, node_id: &NodeId, workers: &[WorkerId]) -> usize {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        node_id.hash(&mut hasher);
        (workers.len() as u64).hash(&mut hasher);
        (hasher.finish() as usize) % workers.len()
    }

    fn compute_ready_nodes(dag: &Dag) -> VecDeque<NodeId> {
        let mut ready = VecDeque::new();

        for node_id in &dag.entry_nodes {
            ready.push_back(*node_id);
        }

        ready
    }

    fn find_newly_ready(&self, completed_node: &NodeId) -> Vec<NodeId> {
        let mut newly_ready = Vec::new();

        for edge in &self.dag.edges {
            if edge.from == *completed_node {
                let to_node = &edge.to;
                // Check if all dependencies satisfied
                if self.are_dependencies_satisfied(to_node) {
                    newly_ready.push(*to_node);
                }
            }
        }

        newly_ready
    }

    fn are_dependencies_satisfied(&self, node_id: &NodeId) -> bool {
        let deps: BTreeSet<_> = self
            .dag
            .edges
            .iter()
            .filter(|e| e.to == *node_id)
            .map(|e| e.from)
            .collect();

        deps.is_subset(&self.completed)
    }
}
```

## Backpressure

```rust
pub struct BackpressureController {
    max_queue_size: usize,
    reject_threshold: f64,  // 0.0 - 1.0
}

impl BackpressureController {
    pub fn should_accept(&self, scheduler: &Scheduler) -> bool {
        let total_queued: usize = scheduler.workers.values()
            .map(|w| w.queue_depth())
            .sum();

        let total_capacity = scheduler.workers.len() * self.max_queue_size;
        let usage = total_queued as f64 / total_capacity.max(1) as f64;

        usage < self.reject_threshold
    }

    pub fn should_throttle(&self, scheduler: &Scheduler) -> bool {
        let total_queued: usize = scheduler.workers.values()
            .map(|w| w.queue_depth())
            .sum();

        let total_capacity = scheduler.workers.len() * self.max_queue_size;
        let usage = total_queued as f64 / total_capacity.max(1) as f64;

        usage > 0.5
    }
}
```

## Worker State

```rust
#[derive(Debug, Clone)]
pub struct WorkerState {
    pub id: WorkerId,
    pub endpoint: String,
    pub capabilities: CapabilitySet,
    pub resources: WorkerResources,
    pub active_tasks: BTreeMap<TaskId, TaskState>,
    pub queue: Vec<TaskId>,
    pub status: WorkerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResources {
    pub cpu_millicores: u64,
    pub memory_bytes: u64,
    pub available_cpu: u64,
    pub available_memory: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Idle,
    Busy,
    Overloaded,
    Draining,
    Unreachable,
}

impl WorkerState {
    pub fn is_available(&self) -> bool {
        matches!(self.status, WorkerStatus::Idle | WorkerStatus::Busy)
            && self.queue.len() < 10  // Max queue depth
    }

    pub fn has_resources(&self, required: &ResourceContract) -> bool {
        self.resources.available_cpu >= required.cpu.millicores
            && self.resources.available_memory >= required.memory.bytes
    }

    pub fn has_capabilities(&self, required: &[Capability]) -> bool {
        required.iter().all(|cap| self.capabilities.has(cap))
    }

    pub fn queue_depth(&self) -> usize {
        self.active_tasks.len() + self.queue.len()
    }
}
```

## Event Logging

Every scheduling decision is logged:

```rust
pub fn emit_assignment_event(decision: &ScheduleDecision) -> Event {
    Event {
        event_id: EventId::new(),
        run_id: RunId::current(),
        node_id: decision.node_id,
        parent_event_id: None,
        logical_time: decision.assigned_at,
        kind: EventKind::TaskAssigned,
        payload: serde_json::to_vec(&decision).unwrap(),
        payload_hash: Hash::compute(&decision),
        prior_state_hash: None,
        post_state_hash: None,
        capability_check_result: None,
        policy_decision_id: None,
        tool_request_hash: None,
        tool_response_hash: None,
        error_data: None,
    }
}
```

## Cluster Scheduling

In cluster mode, scheduling uses Raft for consensus:

```rust
pub struct ClusterScheduler {
    local_id: NodeId,
    raft_handle: RaftHandle,
    local_scheduler: Scheduler,
}

impl ClusterScheduler {
    pub async fn schedule_next(&mut self) -> Result<ScheduleDecision, ScheduleError> {
        // Only leader makes scheduling decisions
        if !self.is_leader()? {
            return Err(ScheduleError::NotLeader);
        }

        // Get local decision
        let decision = self.local_scheduler
            .next_decision()
            .ok_or(ScheduleError::NoReadyTasks)?;

        // Propose via Raft
        let proposal = RaftProposal::Schedule(decision.clone());
        self.raft_handle.propose(proposal).await?;

        // Wait for commitment
        self.raft_handle.wait_for_commitment(&decision.task_id).await?;

        Ok(decision)
    }
}
```

## Performance

- Scheduling decisions: <1ms
- Worker selection: O(n) where n = worker count
- Ready queue maintenance: O(m) where m = edge count
- Backpressure check: O(1)

## Testing

### Property Tests

```rust
#[proptest]
fn test_scheduling_deterministic(
    dag: Dag,
    workers: Vec<WorkerState>,
    config: SchedulerConfig,
) {
    let mut scheduler1 = Scheduler::new(dag.clone(), config.clone());
    let mut scheduler2 = Scheduler::new(dag, config);

    for worker in workers {
        scheduler1.add_worker(worker.clone());
        scheduler2.add_worker(worker);
    }

    // Get same decisions in same order
    loop {
        let d1 = scheduler1.next_decision();
        let d2 = scheduler2.next_decision();

        match (d1, d2) {
            (Some(dec1), Some(dec2)) => {
                assert_eq!(dec1, dec2);
            }
            (None, None) => break,
            _ => panic!("Different decisions"),
        }
    }
}
```
