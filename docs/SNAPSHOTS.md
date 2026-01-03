# Snapshot Protocol Specification

## Overview

Snapshots provide point-in-time state that can be used for fast replay and log compaction. All snapshots are content-addressed and hash-verified.

## Snapshot Format

### Metadata

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub snapshot_id: SnapshotId,
    pub run_id: RunId,
    pub logical_time: LogicalTime,
    pub created_at: Timestamp,
    pub content_hash: Hash,
    pub prior_snapshot_id: Option<SnapshotId>,
    pub log_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub metadata: SnapshotMetadata,
    pub coordinator_state: CoordinatorState,
    pub worker_states: BTreeMap<WorkerId, WorkerState>,
    pub dag_state: DagState,
    pub blobs: Vec<BlobId>,
}
```

### Content

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorState {
    pub completed_nodes: BTreeSet<NodeId>,
    pub failed_nodes: BTreeSet<NodeId>,
    pub current_logical_time: LogicalTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagState {
    pub nodes: BTreeMap<NodeId, NodeExecutionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionState {
    pub node_id: NodeId,
    pub status: NodeStatus,
    pub result_hash: Option<Hash>,
    pub error: Option<ErrorData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Skipped,
}
```

## Snapshot Creation

### Triggers

1. **Periodic** - Every N events
2. **Manual** - User request
3. **Before major operation** - Before joining cluster
4. **After completion** - Final state snapshot

### Builder

```rust
pub struct SnapshotBuilder {
    run_id: RunId,
    logical_time: LogicalTime,
    coordinator_state: CoordinatorState,
    worker_states: BTreeMap<WorkerId, WorkerState>,
    dag: DagState,
    blobs: Vec<BlobId>,
    prior_snapshot_id: Option<SnapshotId>,
    log_index: u64,
}

impl SnapshotBuilder {
    pub fn new(run_id: RunId, logical_time: LogicalTime) -> Self {
        Self {
            run_id,
            logical_time,
            coordinator_state: CoordinatorState::default(),
            worker_states: BTreeMap::new(),
            dag: DagState::default(),
            blobs: Vec::new(),
            prior_snapshot_id: None,
            log_index: 0,
        }
    }

    pub fn with_coordinator_state(mut self, state: CoordinatorState) -> Self {
        self.coordinator_state = state;
        self
    }

    pub fn with_worker(mut self, worker: WorkerState) -> Self {
        self.worker_states.insert(worker.id, worker);
        self
    }

    pub fn with_dag_state(mut self, dag: DagState) -> Self {
        self.dag = dag;
        self
    }

    pub fn prior_snapshot(mut self, id: SnapshotId) -> Self {
        self.prior_snapshot_id = Some(id);
        self
    }

    pub fn build(self) -> Result<Snapshot, SnapshotError> {
        // Serialize snapshot
        let data = serde_json::to_vec(&self.content)?;

        // Compute content hash
        let content_hash = Hash::compute(&data);

        // Create metadata
        let metadata = SnapshotMetadata {
            snapshot_id: SnapshotId::new(),
            run_id: self.run_id,
            logical_time: self.logical_time,
            created_at: Timestamp::now(),
            content_hash,
            prior_snapshot_id: self.prior_snapshot_id,
            log_index: self.log_index,
        };

        Ok(Snapshot {
            metadata,
            coordinator_state: self.coordinator_state,
            worker_states: self.worker_states,
            dag_state: self.dag,
            blobs: self.blobs,
        })
    }
}
```

## Snapshot Storage

### Content Addressing

```rust
pub struct SnapshotStore {
    blob_store: ContentStore,
    index: redb::Database,
}

impl SnapshotStore {
    pub fn save(&self, snapshot: &Snapshot) -> Result<SnapshotId, StorageError> {
        // Serialize snapshot
        let data = postcard::to_allocvec(snapshot)?;

        // Store in blob store
        let blob_id = self.blob_store.put(&data)?;

        // Index by snapshot ID
        let mut txn = self.index.begin_write()?;
        {
            let mut table = txn.open_table(SNAPSHOT_TABLE)?;
            table.insert(snapshot.metadata.snapshot_id, &blob_id)?;
        }
        txn.commit()?;

        Ok(snapshot.metadata.snapshot_id)
    }

    pub fn load(&self, id: SnapshotId) -> Result<Snapshot, StorageError> {
        // Lookup blob ID
        let txn = self.index.begin_read()?;
        let table = txn.open_table(SNAPSHOT_TABLE)?;
        let blob_id = table.get(id)?.ok_or(StorageError::NotFound)?;

        // Load from blob store
        let data = self.blob_store.get(blob_id)?;
        let snapshot = postcard::from_bytes(&data)?;

        Ok(snapshot)
    }

    pub fn list_for_run(&self, run_id: RunId) -> Result<Vec<Snapshot>, StorageError> {
        let mut snapshots = Vec::new();

        // Scan index
        let txn = self.index.begin_read()?;
        let table = txn.open_table(SNAPSHOT_TABLE)?;

        for result in table.iter() {
            let (_, blob_id) = result?;
            let data = self.blob_store.get(blob_id)?;
            let snapshot: Snapshot = postcard::from_bytes(&data)?;

            if snapshot.metadata.run_id == run_id {
                snapshots.push(snapshot);
            }
        }

        Ok(snapshots)
    }
}
```

## Snapshot Validation

```rust
pub struct SnapshotValidator {
    blob_store: ContentStore,
}

impl SnapshotValidator {
    pub fn validate(&self, snapshot: &Snapshot) -> Result<(), ValidationError> {
        // 1. Validate content hash
        let data = postcard::to_allocvec(snapshot)?;
        let computed_hash = Hash::compute(&data);
        if computed_hash != snapshot.metadata.content_hash {
            return Err(ValidationError::HashMismatch {
                expected: snapshot.metadata.content_hash,
                computed: computed_hash,
            });
        }

        // 2. Validate prior snapshot chain
        if let Some(prior_id) = snapshot.metadata.prior_snapshot_id {
            let prior = self.blob_store.get_snapshot(prior_id)?;
            // Verify chain continuity
        }

        // 3. Validate log index
        // Ensure snapshot matches log position

        // 4. Validate all referenced blobs exist
        for blob_id in &snapshot.blobs {
            self.blob_store.exists(blob_id)?;
        }

        Ok(())
    }
}
```

## Snapshot Loading for Replay

```rust
pub struct SnapshotLoader {
    store: SnapshotStore,
}

impl SnapshotLoader {
    pub fn load_for_replay(&self, snapshot_id: SnapshotId) -> Result<ReplayState, ReplayError> {
        let snapshot = self.store.load(snapshot_id)?;

        // Validate before use
        self.validator.validate(&snapshot)?;

        // Reconstruct state
        let mut state = ReplayState::new();

        // Restore coordinator state
        state.restore_coordinator(snapshot.coordinator_state);

        // Restore worker states
        for (worker_id, worker_state) in snapshot.worker_states {
            state.restore_worker(worker_id, worker_state);
        }

        // Restore DAG state
        state.restore_dag(snapshot.dag_state);

        // Set starting logical time
        state.set_logical_time(snapshot.metadata.logical_time);

        Ok(state)
    }
}
```

## Incremental Snapshots

Snapshots can reference prior snapshots to avoid storing full state:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalSnapshot {
    pub metadata: SnapshotMetadata,
    pub base_snapshot_id: SnapshotId,
    pub delta: SnapshotDelta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDelta {
    pub changed_nodes: BTreeMap<NodeId, NodeExecutionState>,
    pub added_workers: BTreeMap<WorkerId, WorkerState>,
    pub removed_workers: BTreeSet<WorkerId>,
    pub new_blobs: Vec<BlobId>,
}

impl IncrementalSnapshot {
    pub fn apply_to(&self, base: &Snapshot) -> Result<Snapshot, SnapshotError> {
        let mut snapshot = base.clone();

        // Apply delta
        for (node_id, state) in &self.delta.changed_nodes {
            snapshot.dag_state.nodes.insert(*node_id, state.clone());
        }

        snapshot.metadata.snapshot_id = self.metadata.snapshot_id;
        snapshot.metadata.content_hash = Hash::compute(&snapshot);

        Ok(snapshot)
    }
}
```

## Compaction

### Log Compaction with Snapshots

```rust
pub struct LogCompactor {
    log: EventLog,
    snapshot_store: SnapshotStore,
    retention_threshold: u64,  // Keep N events after last snapshot
}

impl LogCompactor {
    pub fn compact(&mut self) -> Result<CompactionResult, CompactionError> {
        // Find latest snapshot
        let latest_snapshot = self.snapshot_store.latest()?;

        // Compact events before snapshot (plus retention)
        let keep_index = latest_snapshot.metadata.log_index
            .saturating_sub(self.retention_threshold);

        let removed = self.log.truncate(keep_index)?;

        Ok(CompactionResult {
            removed_count: removed,
            snapshot_id: latest_snapshot.metadata.snapshot_id,
            new_log_head: keep_index,
        })
    }
}
```

## Snapshot Distribution

In cluster mode, snapshots are distributed to all nodes:

```rust
pub struct SnapshotDistributor {
    cluster: ClusterHandle,
}

impl SnapshotDistributor {
    pub async fn distribute(&self, snapshot: &Snapshot) -> Result<(), DistributionError> {
        // Upload to blob store
        let blob_id = self.blob_store.put_snapshot(snapshot).await?;

        // Notify all members
        let members = self.cluster.members().await;

        for member in members {
            if member.id != self.local_id {
                self.notify_snapshot(member.id, blob_id).await?;
            }
        }

        Ok(())
    }
}
```

## Testing

### Property Tests

```rust
#[proptest]
fn test_snapshot_roundtrip(state: ExecutionState) {
    let snapshot = state.create_snapshot();
    let data = postcard::to_allocvec(&snapshot).unwrap();
    let restored: Snapshot = postcard::from_bytes(&data).unwrap();

    assert_eq!(snapshot, restored);
}

#[proptest]
fn test_snapshot_hash_consistency(snapshot: Snapshot) {
    let data1 = postcard::to_allocvec(&snapshot).unwrap();
    let data2 = postcard::to_allocvec(&snapshot).unwrap();

    assert_eq!(data1, data2);
    assert_eq!(Hash::compute(&data1), Hash::compute(&data2));
}
```
