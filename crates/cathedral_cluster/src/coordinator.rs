//! Cluster coordinator for distributed execution.

use crate::{consensus::Consensus, leader::LeaderElection, membership::Membership, remote::RemoteExecutor};
use cathedral_core::{CoreResult, CoreError, EventId, Hash, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Coordinator configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Node ID for this coordinator
    pub node_id: NodeId,
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Execution timeout in milliseconds
    pub execution_timeout_ms: u64,
    /// Retry limit for failed executions
    pub retry_limit: usize,
    /// Snapshot interval in milliseconds
    pub snapshot_interval_ms: u64,
}

impl CoordinatorConfig {
    /// Create a new coordinator config
    #[must_use]
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            max_concurrent: 100,
            execution_timeout_ms: 30000,
            retry_limit: 3,
            snapshot_interval_ms: 60000,
        }
    }

    /// Set max concurrent executions
    #[must_use]
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set execution timeout
    #[must_use]
    pub fn with_execution_timeout(mut self, timeout_ms: u64) -> Self {
        self.execution_timeout_ms = timeout_ms;
        self
    }

    /// Set retry limit
    #[must_use]
    pub fn with_retry_limit(mut self, limit: usize) -> Self {
        self.retry_limit = limit;
        self
    }
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self::new(NodeId::new())
    }
}

/// Coordinator errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoordinatorError {
    /// No workers available
    #[error("No workers available")]
    NoWorkers,

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Quorum lost during execution
    #[error("Quorum lost during execution")]
    QuorumLost,

    /// Timeout
    #[error("Execution timeout after {0}ms")]
    Timeout(u64),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Execution task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTask {
    /// Task ID
    pub task_id: String,
    /// Event ID to execute
    pub event_id: EventId,
    /// Assigned worker
    pub assigned_worker: Option<NodeId>,
    /// Task status
    pub status: TaskStatus,
    /// Retry count
    pub retry_count: usize,
    /// Creation time
    pub created_at: u64,
}

impl ExecutionTask {
    /// Create a new execution task
    #[must_use]
    pub fn new(event_id: EventId) -> Self {
        Self {
            task_id: uuid::Uuid::new_v4().to_string(),
            event_id,
            assigned_worker: None,
            status: TaskStatus::Pending,
            retry_count: 0,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Assign a worker
    #[must_use]
    pub fn with_worker(mut self, worker: NodeId) -> Self {
        self.assigned_worker = Some(worker);
        self.status = TaskStatus::Assigned;
        self
    }

    /// Set status
    #[must_use]
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }

    /// Increment retry count
    #[must_use]
    pub fn with_retry(mut self) -> Self {
        self.retry_count += 1;
        self
    }
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending
    Pending,
    /// Task is assigned
    Assigned,
    /// Task is running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
}

/// Execution result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Task ID
    pub task_id: String,
    /// Event ID that was executed
    pub event_id: EventId,
    /// Result hash
    pub result_hash: Hash,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ExecutionResult {
    /// Create a successful result
    #[must_use]
    pub fn success(task_id: String, event_id: EventId, result_hash: Hash, time_ms: u64) -> Self {
        Self {
            task_id,
            event_id,
            result_hash,
            success: true,
            error: None,
            execution_time_ms: time_ms,
        }
    }

    /// Create a failed result
    #[must_use]
    pub fn failure(task_id: String, event_id: EventId, error: String) -> Self {
        Self {
            task_id,
            event_id,
            result_hash: Hash::compute(&[]),
            success: false,
            error: Some(error),
            execution_time_ms: 0,
        }
    }
}

/// Cluster coordinator
pub struct Coordinator {
    /// Configuration
    config: CoordinatorConfig,
    /// Consensus instance
    consensus: Arc<Consensus>,
    /// Leader election instance
    election: Arc<LeaderElection>,
    /// Membership instance
    membership: Arc<Membership>,
    /// Remote executor
    remote: Arc<RemoteExecutor>,
    /// Active tasks
    tasks: Arc<RwLock<HashMap<String, ExecutionTask>>>,
    /// Completed tasks
    completed: Arc<RwLock<HashMap<String, ExecutionResult>>>,
    /// Current snapshot index
    snapshot_index: Arc<RwLock<u64>>,
}

impl Coordinator {
    /// Create a new coordinator
    #[must_use]
    pub fn new(
        config: CoordinatorConfig,
        consensus: Arc<Consensus>,
        election: Arc<LeaderElection>,
        membership: Arc<Membership>,
        remote: Arc<RemoteExecutor>,
    ) -> Self {
        Self {
            config,
            consensus,
            election,
            membership,
            remote,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            completed: Arc::new(RwLock::new(HashMap::new())),
            snapshot_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Submit a task for execution
    ///
    /// # Errors
    ///
    /// Returns error if submission fails
    pub async fn submit(&self, event_id: EventId) -> CoreResult<String> {
        // Only leader can accept submissions
        if !self.election.is_leader().await {
            return Err(CoreError::Validation {
                field: "leader".to_string(),
                reason: "Not a leader".to_string(),
            });
        }

        let task = ExecutionTask::new(event_id.clone());
        let task_id = task.task_id.clone();

        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id.clone(), task);

        Ok(task_id)
    }

    /// Assign a task to a worker
    ///
    /// # Errors
    ///
    /// Returns error if assignment fails
    pub async fn assign_task(&self, task_id: String, worker_id: NodeId) -> CoreResult<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.assigned_worker = Some(worker_id);
            task.status = TaskStatus::Assigned;
            Ok(())
        } else {
            Err(CoreError::NotFound {
                kind: "task".to_string(),
                id: task_id,
            })
        }
    }

    /// Get pending tasks
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn pending_tasks(&self) -> Vec<ExecutionTask> {
        self.tasks
            .read()
            .await
            .values()
            .filter(|t| t.status == TaskStatus::Pending)
            .cloned()
            .collect()
    }

    /// Select a worker for a task
    ///
    /// # Errors
    ///
    /// Returns error if no workers available
    pub async fn select_worker(&self) -> CoreResult<NodeId> {
        let members = self.membership.active_members().await;
        let coordinator_id = self.config.node_id;

        // Filter out the coordinator itself
        let workers: Vec<NodeId> = members
            .iter()
            .map(|m| m.node_id)
            .filter(|id| *id != coordinator_id)
            .collect();

        if workers.is_empty() {
            return Err(CoreError::Validation {
                field: "workers".to_string(),
                reason: "No workers available".to_string(),
            });
        }

        // Simple round-robin: use first available
        Ok(workers[0])
    }

    /// Execute a task on a worker
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    pub async fn execute_task(&self, task_id: String) -> CoreResult<ExecutionResult> {
        let (worker_id, event_id) = {
            let tasks = self.tasks.read().await;
            let task = tasks.get(&task_id).ok_or_else(|| CoreError::NotFound {
                kind: "task".to_string(),
                id: task_id.clone(),
            })?;

            let worker_id = task.assigned_worker.ok_or_else(|| CoreError::Validation {
                field: "assigned_worker".to_string(),
                reason: "Task not assigned".to_string(),
            })?;

            (worker_id, task.event_id.clone())
        };

        let start = std::time::Instant::now();

        // Update task status to running
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.status = TaskStatus::Running;
            }
        }

        // Execute remotely
        let request = crate::remote::RemoteRequest::new(
            self.config.node_id,
            event_id.clone(),
            Vec::new(),
        );

        match self.remote.execute_remote(worker_id, request).await {
            Ok(response) => {
                let elapsed = start.elapsed().as_millis() as u64;
                let result = ExecutionResult::success(
                    task_id.clone(),
                    event_id,
                    Hash::compute(&response.payload),
                    elapsed,
                );

                // Update task status
                {
                    let mut tasks = self.tasks.write().await;
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.status = TaskStatus::Completed;
                    }
                }

                // Store result
                let mut completed = self.completed.write().await;
                completed.insert(task_id.clone(), result.clone());

                Ok(result)
            }
            Err(e) => {
                let mut tasks = self.tasks.write().await;
                if let Some(task) = tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Failed;

                    // Retry if under limit
                    if task.retry_count < self.config.retry_limit {
                        task.status = TaskStatus::Pending;
                        task.assigned_worker = None;
                        task.retry_count += 1;
                    }
                }

                Err(e)
            }
        }
    }

    /// Get task by ID
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn get_task(&self, task_id: String) -> Option<ExecutionTask> {
        self.tasks.read().await.get(&task_id).cloned()
    }

    /// Get result by task ID
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn get_result(&self, task_id: String) -> Option<ExecutionResult> {
        self.completed.read().await.get(&task_id).cloned()
    }

    /// Create a snapshot
    ///
    /// # Errors
    ///
    /// Returns error if snapshot creation fails
    pub async fn create_snapshot(&self) -> CoreResult<u64> {
        let mut index = self.snapshot_index.write().await;
        *index += 1;

        // In a real implementation, this would serialize state
        let _ = (self.tasks.read().await, self.completed.read().await);

        Ok(*index)
    }

    /// Get current snapshot index
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn snapshot_index(&self) -> u64 {
        *self.snapshot_index.read().await
    }

    /// Get active task count
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn active_task_count(&self) -> usize {
        self.tasks
            .read()
            .await
            .values()
            .filter(|t| matches!(t.status, TaskStatus::Running | TaskStatus::Assigned))
            .count()
    }

    /// Get completed task count
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn completed_task_count(&self) -> usize {
        self.completed.read().await.len()
    }

    /// Process pending tasks
    ///
    /// # Errors
    ///
    /// Returns error if processing fails
    pub async fn process_pending(&self) -> CoreResult<Vec<ExecutionResult>> {
        let pending = self.pending_tasks().await;
        let mut results = Vec::new();

        for task in pending {
            let worker = self.select_worker().await?;
            self.assign_task(task.task_id.clone(), worker).await?;

            match self.execute_task(task.task_id.clone()).await {
                Ok(result) => results.push(result),
                Err(_) => continue,
            }
        }

        Ok(results)
    }

    /// Check if coordinator is healthy
    ///
    /// # Errors
    ///
    /// Returns error if check fails
    pub async fn is_healthy(&self) -> bool {
        let has_leader = self.election.leader().await.is_some();
        let has_quorum = self.membership.active_count().await >= 2;
        has_leader && has_quorum
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        let config = CoordinatorConfig::default();
        let consensus = Arc::new(crate::consensus::Consensus::default());
        let election = Arc::new(LeaderElection::default());
        let membership = Arc::new(Membership::default());
        let remote = Arc::new(RemoteExecutor::default());

        Self::new(config, consensus, election, membership, remote)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{consensus::ConsensusConfig, leader::ElectionConfig};

    #[tokio::test]
    async fn test_coordinator_config_new() {
        let node_id = NodeId::new();
        let config = CoordinatorConfig::new(node_id);
        assert_eq!(config.node_id, node_id);
        assert_eq!(config.max_concurrent, 100);
        assert_eq!(config.execution_timeout_ms, 30000);
    }

    #[tokio::test]
    async fn test_coordinator_config_with_max_concurrent() {
        let config = CoordinatorConfig::new(NodeId::new()).with_max_concurrent(50);
        assert_eq!(config.max_concurrent, 50);
    }

    #[tokio::test]
    async fn test_execution_task_new() {
        let event_id = EventId::new();
        let task = ExecutionTask::new(event_id.clone());
        assert_eq!(task.event_id, event_id);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.assigned_worker.is_none());
        assert_eq!(task.retry_count, 0);
    }

    #[tokio::test]
    async fn test_execution_task_with_worker() {
        let event_id = EventId::new();
        let worker_id = NodeId::new();
        let task = ExecutionTask::new(event_id).with_worker(worker_id);
        assert_eq!(task.assigned_worker, Some(worker_id));
        assert_eq!(task.status, TaskStatus::Assigned);
    }

    #[tokio::test]
    async fn test_execution_result_success() {
        let result = ExecutionResult::success(
            "task-1".to_string(),
            EventId::new(),
            Hash::compute(&[]),
            100,
        );
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.execution_time_ms, 100);
    }

    #[tokio::test]
    async fn test_execution_result_failure() {
        let result = ExecutionResult::failure(
            "task-1".to_string(),
            EventId::new(),
            "error".to_string(),
        );
        assert!(!result.success);
        assert_eq!(result.error, Some("error".to_string()));
    }

    #[tokio::test]
    async fn test_coordinator_new() {
        let node_id = NodeId::new();
        let config = CoordinatorConfig::new(node_id);
        let consensus_config = crate::consensus::ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let election_config = ElectionConfig::new(node_id);
        let election = Arc::new(LeaderElection::new(
            election_config,
            consensus.clone(),
            Arc::new(Membership::new(node_id)),
        ));
        let membership = Arc::new(Membership::new(node_id));
        let remote = Arc::new(RemoteExecutor::new(node_id));

        let coordinator = Coordinator::new(
            config.clone(),
            consensus,
            election,
            membership,
            remote,
        );

        assert_eq!(coordinator.active_task_count().await, 0);
        assert_eq!(coordinator.completed_task_count().await, 0);
        assert_eq!(coordinator.snapshot_index().await, 0);
    }

    #[tokio::test]
    async fn test_coordinator_submit() {
        let node_id = NodeId::new();
        let config = CoordinatorConfig::new(node_id);
        let consensus_config = crate::consensus::ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let election_config = ElectionConfig::new(node_id);
        let election = Arc::new(LeaderElection::new(
            election_config,
            consensus.clone(),
            Arc::new(Membership::new(node_id)),
        ));
        // Set as leader
        election.set_state(crate::leader::ElectionState::Leader).await;

        let membership = Arc::new(Membership::new(node_id));
        let remote = Arc::new(RemoteExecutor::new(node_id));

        let coordinator = Coordinator::new(
            config,
            consensus,
            election,
            membership,
            remote,
        );

        let event_id = EventId::new();
        let task_id = coordinator.submit(event_id).await;
        assert!(task_id.is_ok());
    }

    #[tokio::test]
    async fn test_coordinator_assign_task() {
        let node_id = NodeId::new();
        let config = CoordinatorConfig::new(node_id);
        let consensus_config = crate::consensus::ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let election_config = ElectionConfig::new(node_id);
        let election = Arc::new(LeaderElection::new(
            election_config,
            consensus.clone(),
            Arc::new(Membership::new(node_id)),
        ));
        // Set as leader
        election.set_state(crate::leader::ElectionState::Leader).await;

        let membership = Arc::new(Membership::new(node_id));
        let remote = Arc::new(RemoteExecutor::new(node_id));

        let coordinator = Coordinator::new(
            config,
            consensus,
            election,
            membership,
            remote,
        );

        let event_id = EventId::new();
        let task_id = coordinator.submit(event_id).await.unwrap();
        let worker_id = NodeId::new();

        let result = coordinator.assign_task(task_id.clone(), worker_id).await;
        assert!(result.is_ok());

        let task = coordinator.get_task(task_id).await;
        assert!(task.is_some());
        assert_eq!(task.unwrap().assigned_worker, Some(worker_id));
    }

    #[tokio::test]
    async fn test_coordinator_create_snapshot() {
        let node_id = NodeId::new();
        let config = CoordinatorConfig::new(node_id);
        let consensus_config = crate::consensus::ConsensusConfig::new(node_id);
        let consensus = Arc::new(Consensus::new(consensus_config));
        let election_config = ElectionConfig::new(node_id);
        let election = Arc::new(LeaderElection::new(
            election_config,
            consensus.clone(),
            Arc::new(Membership::new(node_id)),
        ));
        let membership = Arc::new(Membership::new(node_id));
        let remote = Arc::new(RemoteExecutor::new(node_id));

        let coordinator = Coordinator::new(
            config,
            consensus,
            election,
            membership,
            remote,
        );

        let index1 = coordinator.create_snapshot().await.unwrap();
        let index2 = coordinator.create_snapshot().await.unwrap();

        assert_eq!(index1, 1);
        assert_eq!(index2, 2);
    }

    #[test]
    fn test_task_status_equality() {
        assert_eq!(TaskStatus::Pending, TaskStatus::Pending);
        assert_ne!(TaskStatus::Pending, TaskStatus::Running);
        assert_ne!(TaskStatus::Completed, TaskStatus::Failed);
    }
}
