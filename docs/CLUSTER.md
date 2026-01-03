# Cluster Execution Specification

## Overview

CATHEDRAL can execute workflows across a cluster of machines while maintaining deterministic ordering through a replicated log.

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         Coordinator                            │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Raft Consensus Module                                   │  │
│  │  - Leader election                                       │  │
│  │  - Log replication                                       │  │
│  │  - Snapshot transfer                                     │  │
│  └───────────────┬──────────────────────────────────────────┘  │
│                  │                                               │
│  ┌───────────────┴──────────────────────────────────────────┐  │
│  │  Scheduler (only on leader)                              │  │
│  └───────────────┬──────────────────────────────────────────┘  │
│                  │                                               │
│  ┌───────────────┴──────────────────────────────────────────┐  │
│  │  Event Log (replicated)                                  │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────┬───────────────────────────────────────┘
                         │ RPC
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   Worker 1  │  │   Worker 2  │  │   Worker 3  │
│  ┌────────┐  │  │  ┌────────┐  │  │  ┌────────┐  │
│  │Executor│  │  │  │Executor│  │  │  │Executor│  │
│  │Sandbox │  │  │  │Sandbox │  │  │  │Sandbox │  │
│  └────────┘  │  │  └────────┘  │  │  └────────┘  │
└─────────────┘  └─────────────┘  └─────────────┘
```

## Replicated Log

### Log Entry Format

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    pub command: Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Assign task to worker
    AssignTask {
        task_id: TaskId,
        node_id: NodeId,
        worker_id: WorkerId,
    },

    /// Task completed
    TaskCompleted {
        task_id: TaskId,
        worker_id: WorkerId,
        result: TaskResult,
    },

    /// Worker heartbeat
    Heartbeat {
        worker_id: WorkerId,
        status: WorkerStatus,
    },

    /// Membership change
    AddWorker { worker_id: WorkerId },
    RemoveWorker { worker_id: WorkerId },

    /// Snapshot boundary
    Snapshot { snapshot_id: SnapshotId },
}
```

### Consensus Configuration

```rust
pub struct ConsensusConfig {
    pub election_timeout: Duration,
    pub heartbeat_interval: Duration,
    pub max_payload_entries: u64,
    pub snapshot_threshold: u64,
    pub log_compaction_threshold: u64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            election_timeout: Duration::from_millis(150),
            heartbeat_interval: Duration::from_millis(50),
            max_payload_entries: 300,
            snapshot_threshold: 10_000,
            log_compaction_threshold: 50_000,
        }
    }
}
```

## Coordinator

### Leader Election

```rust
pub struct Coordinator {
    node_id: NodeId,
    raft: Raft<Node>,
    state: CoordinatorState,
    scheduler: Option<Scheduler>,  // Only on leader
}

#[derive(Debug, Clone)]
pub enum CoordinatorState {
    Leader,
    Candidate,
    Follower { leader_id: Option<NodeId> },
    Learner,
}

impl Coordinator {
    pub async fn start(&mut self) -> Result<(), ClusterError> {
        // Start Raft
        self.raft.start().await?;

        // Watch for leadership changes
        let mut leader_rx = self.raft.leader_changes();
        tokio::spawn(async move {
            while let Some(leader_id) = leader_rx.recv().await {
                if leader_id == Some(self.node_id) {
                    self.become_leader().await;
                } else {
                    self.become_follower(leader_id).await;
                }
            }
        });

        Ok(())
    }

    async fn become_leader(&mut self) {
        self.state = CoordinatorState::Leader;
        self.scheduler = Some(Scheduler::new(self.dag.clone(), self.config.clone()));

        // Start scheduling loop
        tokio::spawn(async move {
            self.scheduling_loop().await;
        });
    }

    async fn scheduling_loop(&mut self) {
        loop {
            if let Some(decision) = self.scheduler.next_decision() {
                // Propose via Raft
                let entry = LogEntry {
                    index: 0,  // Set by Raft
                    term: 0,   // Set by Raft
                    command: Command::AssignTask {
                        task_id: decision.task_id,
                        node_id: decision.node_id,
                        worker_id: decision.worker_id,
                    },
                };

                self.raft.propose(entry).await.unwrap();
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    pub async fn submit_result(
        &mut self,
        task_id: TaskId,
        result: TaskResult,
    ) -> Result<(), ClusterError> {
        let entry = LogEntry {
            index: 0,
            term: 0,
            command: Command::TaskCompleted {
                task_id,
                worker_id: self.node_id,
                result,
            },
        };

        self.raft.propose(entry).await?;
        Ok(())
    }
}
```

## Worker

### Worker Implementation

```rust
pub struct Worker {
    id: WorkerId,
    coordinator_endpoint: String,
    executor: Executor,
    state: WorkerState,
}

impl Worker {
    pub async fn start(&mut self) -> Result<(), WorkerError> {
        // Connect to coordinator
        let mut client = CoordinatorClient::connect(&self.coordinator_endpoint).await?;

        // Register with cluster
        client.register(self.id).await?;

        // Start task executor loop
        loop {
            // Poll for assignment
            if let Some(assignment) = client.poll_assignment(self.id).await? {
                self.execute_assignment(assignment).await?;
            }

            // Send heartbeat
            client.heartbeat(self.id, self.state.status).await?;
        }
    }

    async fn execute_assignment(
        &mut self,
        assignment: TaskAssignment,
    ) -> Result<(), WorkerError> {
        // Execute in sandbox
        let result = self.executor.execute(assignment.task).await?;

        // Submit result to coordinator
        let mut client = self.coordinator_client().await?;
        client.submit_result(assignment.task_id, result).await?;

        Ok(())
    }
}
```

## Remote Execution

### RPC Protocol

```rust
#[async_trait]
pub trait CoordinatorService: Send + Sync {
    async fn register_worker(&self, worker: WorkerInfo) -> Result<(), RpcError>;
    async fn poll_assignment(&self, worker_id: WorkerId) -> Result<Option<TaskAssignment>, RpcError>;
    async fn submit_result(&self, result: TaskResult) -> Result<(), RpcError>;
    async fn heartbeat(&self, worker_id: WorkerId, status: WorkerStatus) -> Result<(), RpcError>;
}

#[async_trait]
pub trait WorkerService: Send + Sync {
    async fn execute_task(&self, task: Task) -> Result<TaskResult, RpcError>;
    async fn cancel_task(&self, task_id: TaskId) -> Result<(), RpcError>;
}
```

### Transport

Uses Tower/Tokio for transport:

```rust
pub struct RpcClient {
    client: reqwest::Client,
    base_url: String,
}

impl RpcClient {
    pub async fn call<T: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, RpcError> {
        let response = self.client
            .post(format!("{}/{}", self.base_url, method))
            .json(&params)
            .send()
            .await?;

        Ok(response.json().await?)
    }
}
```

## Membership

### Membership Protocol

```rust
pub struct Membership {
    members: BTreeMap<NodeId, Member>,
    pending: Vec<MembershipChange>,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub id: NodeId,
    pub endpoint: String,
    pub role: NodeRole,
    pub status: MemberStatus,
    pub last_heartbeat: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Coordinator,
    Worker,
    Learner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus {
    Active,
    Suspect,
    Down,
}

impl Membership {
    pub fn add_member(&mut self, member: Member) -> Result<(), MembershipError> {
        if self.members.contains_key(&member.id) {
            return Err(MembershipError::AlreadyExists);
        }

        // Propose membership change via Raft
        let change = MembershipChange::Add(member.clone());
        self.propose_change(change)?;

        Ok(())
    }

    pub fn remove_member(&mut self, id: NodeId) -> Result<(), MembershipError> {
        if !self.members.contains_key(&id) {
            return Err(MembershipError::NotFound);
        }

        let change = MembershipChange::Remove(id);
        self.propose_change(change)?;

        Ok(())
    }

    fn propose_change(&self, change: MembershipChange) -> Result<(), MembershipError> {
        // Submit to Raft log
        todo!()
    }
}
```

## Failure Detection

### Suspicion Mechanism

```rust
pub struct FailureDetector {
    suspicion_timeout: Duration,
    last_heartbeats: BTreeMap<NodeId, Timestamp>,
}

impl FailureDetector {
    pub fn record_heartbeat(&mut self, node_id: NodeId) {
        self.last_heartbeats.insert(node_id, Timestamp::now());
    }

    pub fn check_failures(&self) -> Vec<NodeId> {
        let now = Timestamp::now();
        self.last_heartbeats
            .iter()
            .filter(|(_, last)| now.duration_since(*last) > self.suspicion_timeout)
            .map(|(id, _)| *id)
            .collect()
    }
}
```

## Snapshot Transfer

### Snapshot Protocol

```rust
pub struct SnapshotTransfer {
    snapshot: Snapshot,
    chunk_size: usize,
}

impl SnapshotTransfer {
    pub async fn send_to(&self, dest: &str) -> Result<(), TransferError> {
        let mut client = SnapshotClient::connect(dest).await?;

        for chunk in self.chunks() {
            client.send_chunk(chunk).await?;
        }

        client.finalize().await?;
        Ok(())
    }

    pub fn chunks(&self) -> Vec<SnapshotChunk> {
        let mut chunks = Vec::new();
        let data = self.snapshot.serialize();

        for (i, chunk) in data.chunks(self.chunk_size).enumerate() {
            chunks.push(SnapshotChunk {
                index: i,
                total: (data.len() + self.chunk_size - 1) / self.chunk_size,
                data: chunk.to_vec(),
            });
        }

        chunks
    }
}
```

## Determinism Guarantees

1. **Single scheduler** - Only leader makes scheduling decisions
2. **Log replication** - All decisions replicated before execution
3. **Total order** - Raft provides total ordering of events
4. **Snapshot consistency** - Snapshots include log index
5. **Replayable** - Full cluster state reconstructable from log

## Testing

### Simulation Tests

```rust
#[tokio::test]
async fn test_cluster_deterministic() {
    let mut sim = ClusterSim::new();

    // Create cluster
    sim.add_coordinator();
    sim.add_workers(3);

    // Submit workflow
    sim.submit_workflow(test_workflow()).await;

    // Run to completion
    sim.run_until_complete().await;

    // Get event log
    let log = sim.get_event_log();

    // Verify determinism
    sim.verify_determinism(&log);
}
```

## Deployment

### Single-Node to Cluster

```bash
# Start single node
cathedral-server --role standalone

# Start coordinator
cathedral-server --role coordinator --bind 0.0.0.0:8080

# Start workers
cathedral-server --role worker --join coordinator:8080 --bind 0.0.0.0:8081
cathedral-server --role worker --join coordinator:8080 --bind 0.0.0.0:8082
```
