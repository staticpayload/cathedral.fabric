//! Worker node for cluster execution.

use crate::{membership::Membership, remote::RemoteRequest};
use cathedral_core::{CoreResult, CoreError, EventId, NodeId};
use cathedral_runtime::Executor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Worker configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Node ID for this worker
    pub node_id: NodeId,
    /// Worker address
    pub address: String,
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Execution timeout in milliseconds
    pub execution_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Capabilities
    pub capabilities: Vec<String>,
}

impl WorkerConfig {
    /// Create a new worker config
    #[must_use]
    pub fn new(node_id: NodeId, address: String) -> Self {
        Self {
            node_id,
            address,
            max_concurrent: 10,
            execution_timeout_ms: 30000,
            heartbeat_interval_ms: 5000,
            capabilities: Vec::new(),
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

    /// Add a capability
    #[must_use]
    pub fn with_capability(mut self, capability: String) -> Self {
        self.capabilities.push(capability);
        self
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self::new(NodeId::new(), "127.0.0.1:0".to_string())
    }
}

/// Worker errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkerError {
    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Worker busy
    #[error("Worker busy: {current}/{max} concurrent executions")]
    WorkerBusy { current: usize, max: usize },

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Not registered
    #[error("Worker not registered with cluster")]
    NotRegistered,

    /// Shutdown
    #[error("Worker shutting down")]
    ShuttingDown,
}

/// Worker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerState {
    /// Worker is idle
    Idle,
    /// Worker is busy
    Busy,
    /// Worker is draining
    Draining,
    /// Worker is shut down
    Shutdown,
}

/// Execution job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    /// Job ID
    pub job_id: String,
    /// Event being executed
    pub event_id: EventId,
    /// Request that triggered this job
    pub request: RemoteRequest,
    /// Job status
    pub status: JobStatus,
    /// Start time
    pub started_at: u64,
}

impl Job {
    /// Create a new job
    #[must_use]
    pub fn new(event_id: EventId, request: RemoteRequest) -> Self {
        Self {
            job_id: uuid::Uuid::new_v4().to_string(),
            event_id,
            request,
            status: JobStatus::Pending,
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Get job ID
    #[must_use]
    pub fn id(&self) -> &str {
        &self.job_id
    }
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is pending
    Pending,
    /// Job is running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
}

/// Worker node
pub struct Worker {
    /// Configuration
    config: WorkerConfig,
    /// Worker state
    state: Arc<RwLock<WorkerState>>,
    /// Membership instance
    membership: Arc<Membership>,
    /// Active jobs
    jobs: Arc<RwLock<HashMap<String, Job>>>,
    /// Completed jobs
    completed: Arc<RwLock<HashMap<String, Job>>>,
    /// Executor for running events
    executor: Arc<Executor>,
    /// Registered flag
    registered: Arc<RwLock<bool>>,
}

impl Worker {
    /// Create a new worker
    #[must_use]
    pub fn new(
        config: WorkerConfig,
        membership: Arc<Membership>,
        executor: Arc<Executor>,
    ) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(WorkerState::Idle)),
            membership,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            completed: Arc::new(RwLock::new(HashMap::new())),
            executor,
            registered: Arc::new(RwLock::new(false)),
        }
    }

    /// Get the worker's node ID
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        self.config.node_id
    }

    /// Get the worker's address
    #[must_use]
    pub fn address(&self) -> &str {
        &self.config.address
    }

    /// Get the worker's state
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn state(&self) -> WorkerState {
        *self.state.read().await
    }

    /// Register with the cluster
    ///
    /// # Errors
    ///
    /// Returns error if registration fails
    pub async fn register(&self) -> CoreResult<()> {
        let member = crate::membership::Member::new(self.config.node_id, self.config.address.clone())
            .with_state(crate::membership::MemberState::Active)
            .with_heartbeat(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            );

        self.membership.add_member(member).await?;

        // Add capabilities to member
        // In a real implementation, we'd update the member with capabilities
        let _ = &self.config.capabilities;

        *self.registered.write().await = true;
        Ok(())
    }

    /// Unregister from the cluster
    ///
    /// # Errors
    ///
    /// Returns error if unregistration fails
    pub async fn unregister(&self) -> CoreResult<()> {
        self.membership.remove_member(self.config.node_id).await?;
        *self.registered.write().await = false;
        Ok(())
    }

    /// Check if worker is registered
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn is_registered(&self) -> bool {
        *self.registered.read().await
    }

    /// Send heartbeat
    ///
    /// # Errors
    ///
    /// Returns error if heartbeat fails
    pub async fn heartbeat(&self) -> CoreResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.membership.update_heartbeat(self.config.node_id, now).await?;
        Ok(())
    }

    /// Accept a job for execution
    ///
    /// # Errors
    ///
    /// Returns error if job cannot be accepted
    pub async fn accept_job(&self, event_id: EventId, request: RemoteRequest) -> CoreResult<String> {
        let state = *self.state.read().await;
        if state == WorkerState::Shutdown || state == WorkerState::Draining {
            return Err(CoreError::Validation {
                field: "state".to_string(),
                reason: "Worker not accepting jobs".to_string(),
            });
        }

        let job_count = self.jobs.read().await.len();
        if job_count >= self.config.max_concurrent {
            return Err(CoreError::Validation {
                field: "concurrent".to_string(),
                reason: format!("Worker busy: {}/{} jobs", job_count, self.config.max_concurrent),
            });
        }

        let job = Job::new(event_id, request);
        let job_id = job.job_id.clone();

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id.clone(), job);

        Ok(job_id)
    }

    /// Execute a job
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    pub async fn execute_job(&self, job_id: String) -> CoreResult<Vec<u8>> {
        let (event_id, request) = {
            let mut jobs = self.jobs.write().await;
            let job = jobs.get_mut(&job_id).ok_or_else(|| CoreError::NotFound {
                kind: "job".to_string(),
                id: job_id.clone(),
            })?;

            job.status = JobStatus::Running;
            (job.event_id.clone(), job.request.clone())
        };

        // Execute the event
        // In a real implementation, this would use the executor
        let _ = (event_id, self.executor.clone());

        // Simulate execution
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Complete the job
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.remove(&job_id) {
            let mut completed_job = job;
            completed_job.status = JobStatus::Completed;

            let mut completed = self.completed.write().await;
            completed.insert(job_id.clone(), completed_job);
        }

        Ok(request.payload)
    }

    /// Get job by ID
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn get_job(&self, job_id: String) -> Option<Job> {
        self.jobs.read().await.get(&job_id).cloned()
    }

    /// Get active job count
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn active_job_count(&self) -> usize {
        self.jobs.read().await.len()
    }

    /// Get completed job count
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn completed_job_count(&self) -> usize {
        self.completed.read().await.len()
    }

    /// Start draining (stop accepting new jobs)
    ///
    /// # Errors
    ///
    /// Returns error if state transition fails
    pub async fn start_drain(&self) {
        *self.state.write().await = WorkerState::Draining;
    }

    /// Shutdown the worker
    ///
    /// # Errors
    ///
    /// Returns error if shutdown fails
    pub async fn shutdown(&self) -> CoreResult<()> {
        *self.state.write().await = WorkerState::Shutdown;

        // Wait for active jobs to complete
        while self.active_job_count().await > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        self.unregister().await?;
        Ok(())
    }

    /// Get worker statistics
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn stats(&self) -> WorkerStats {
        WorkerStats {
            node_id: self.config.node_id,
            state: *self.state.read().await,
            active_jobs: self.active_job_count().await,
            completed_jobs: self.completed_job_count().await,
            max_concurrent: self.config.max_concurrent,
        }
    }

    /// Check if worker can accept jobs
    ///
    /// # Errors
    ///
    /// Returns error if check fails
    pub async fn can_accept_jobs(&self) -> bool {
        let state = *self.state.read().await;
        let registered = *self.registered.read().await;
        let job_count = self.active_job_count().await;

        state == WorkerState::Idle
            && registered
            && job_count < self.config.max_concurrent
    }
}

/// Worker statistics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerStats {
    /// Node ID
    pub node_id: NodeId,
    /// Current state
    pub state: WorkerState,
    /// Active job count
    pub active_jobs: usize,
    /// Completed job count
    pub completed_jobs: usize,
    /// Maximum concurrent jobs
    pub max_concurrent: usize,
}

impl Default for Worker {
    fn default() -> Self {
        let config = WorkerConfig::default();
        let membership = Arc::new(Membership::new(config.node_id));
        let executor = Arc::new(Executor::default());

        Self::new(config, membership, executor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_config_new() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "127.0.0.1:8080".to_string());
        assert_eq!(config.node_id, node_id);
        assert_eq!(config.address, "127.0.0.1:8080");
        assert_eq!(config.max_concurrent, 10);
    }

    #[tokio::test]
    async fn test_worker_config_with_max_concurrent() {
        let config = WorkerConfig::new(NodeId::new(), "addr".to_string())
            .with_max_concurrent(5);
        assert_eq!(config.max_concurrent, 5);
    }

    #[tokio::test]
    async fn test_worker_config_with_capability() {
        let config = WorkerConfig::new(NodeId::new(), "addr".to_string())
            .with_capability("wasm".to_string());
        assert_eq!(config.capabilities, vec!["wasm"]);
    }

    #[tokio::test]
    async fn test_job_new() {
        let event_id = EventId::new();
        let request = RemoteRequest::new(NodeId::new(), event_id.clone(), b"data".to_vec());
        let job = Job::new(event_id, request);
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_job_id() {
        let event_id = EventId::new();
        let request = RemoteRequest::new(NodeId::new(), event_id, b"data".to_vec());
        let job = Job::new(event_id, request);
        assert!(!job.id().is_empty());
    }

    #[tokio::test]
    async fn test_worker_new() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string());
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);
        assert_eq!(worker.node_id(), node_id);
        assert_eq!(worker.address(), "addr");
        assert_eq!(worker.active_job_count().await, 0);
    }

    #[tokio::test]
    async fn test_worker_register() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string());
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);
        worker.register().await.unwrap();
        assert!(worker.is_registered().await);
    }

    #[tokio::test]
    async fn test_worker_unregister() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string());
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);
        worker.register().await.unwrap();
        worker.unregister().await.unwrap();
        assert!(!worker.is_registered().await);
    }

    #[tokio::test]
    async fn test_worker_heartbeat() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string());
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);
        worker.register().await.unwrap();
        worker.heartbeat().await.unwrap();
    }

    #[tokio::test]
    async fn test_worker_accept_job() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string());
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);

        let event_id = EventId::new();
        let request = RemoteRequest::new(NodeId::new(), event_id.clone(), b"data".to_vec());

        let job_id = worker.accept_job(event_id, request).await.unwrap();
        assert!(!job_id.is_empty());
        assert_eq!(worker.active_job_count().await, 1);
    }

    #[tokio::test]
    async fn test_worker_accept_job_busy() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string())
            .with_max_concurrent(1);
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);

        let event_id = EventId::new();
        let request = RemoteRequest::new(NodeId::new(), event_id, b"data".to_vec());

        worker.accept_job(event_id, request.clone()).await.unwrap();

        let result = worker.accept_job(event_id, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_worker_execute_job() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string());
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);

        let event_id = EventId::new();
        let request = RemoteRequest::new(NodeId::new(), event_id, b"data".to_vec());

        let job_id = worker.accept_job(event_id.clone(), request.clone()).await.unwrap();
        let result = worker.execute_job(job_id.clone()).await.unwrap();

        assert_eq!(result, b"data");
        assert_eq!(worker.active_job_count().await, 0);
        assert_eq!(worker.completed_job_count().await, 1);
    }

    #[tokio::test]
    async fn test_worker_start_drain() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string());
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);
        worker.start_drain().await;

        assert_eq!(worker.state().await, WorkerState::Draining);
        assert!(!worker.can_accept_jobs().await);
    }

    #[tokio::test]
    async fn test_worker_stats() {
        let node_id = NodeId::new();
        let config = WorkerConfig::new(node_id, "addr".to_string());
        let membership = Arc::new(Membership::new(node_id));
        let executor = Arc::new(Executor::default());

        let worker = Worker::new(config, membership, executor);
        let stats = worker.stats().await;

        assert_eq!(stats.node_id, node_id);
        assert_eq!(stats.active_jobs, 0);
        assert_eq!(stats.completed_jobs, 0);
        assert_eq!(stats.max_concurrent, 10);
    }

    #[test]
    fn test_worker_state_equality() {
        assert_eq!(WorkerState::Idle, WorkerState::Idle);
        assert_ne!(WorkerState::Idle, WorkerState::Busy);
        assert_ne!(WorkerState::Draining, WorkerState::Shutdown);
    }

    #[test]
    fn test_job_status_equality() {
        assert_eq!(JobStatus::Pending, JobStatus::Pending);
        assert_ne!(JobStatus::Pending, JobStatus::Running);
        assert_ne!(JobStatus::Completed, JobStatus::Failed);
    }
}
