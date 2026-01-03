//! Remote execution over network.

use cathedral_core::{CoreResult, CoreError, EventId, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Transport errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Timeout
    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid response
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Node unavailable
    #[error("Node unavailable: {0}")]
    NodeUnavailable(NodeId),
}

/// Remote execution request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRequest {
    /// Request ID
    pub request_id: String,
    /// Source node
    pub source: NodeId,
    /// Target event
    pub event_id: EventId,
    /// Request payload
    pub payload: Vec<u8>,
}

impl RemoteRequest {
    /// Create a new remote request
    #[must_use]
    pub fn new(source: NodeId, event_id: EventId, payload: Vec<u8>) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            source,
            event_id,
            payload,
        }
    }
}

/// Remote execution response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteResponse {
    /// Request ID this responds to
    pub request_id: String,
    /// Response payload
    pub payload: Vec<u8>,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl RemoteResponse {
    /// Create a successful response
    #[must_use]
    pub fn success(request_id: String, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            payload,
            success: true,
            error: None,
        }
    }

    /// Create a failed response
    #[must_use]
    pub fn error(request_id: String, error: String) -> Self {
        Self {
            request_id,
            payload: Vec::new(),
            success: false,
            error: Some(error),
        }
    }
}

/// Remote executor client
#[derive(Clone)]
pub struct RemoteClient {
    /// Target node ID
    target: NodeId,
    /// Target address
    address: String,
    /// Request timeout in milliseconds
    timeout_ms: u64,
}

impl RemoteClient {
    /// Create a new remote client
    #[must_use]
    pub fn new(target: NodeId, address: String) -> Self {
        Self {
            target,
            address,
            timeout_ms: 5000,
        }
    }

    /// Get the target node ID
    #[must_use]
    pub fn target(&self) -> NodeId {
        self.target
    }

    /// Send a request to the target node
    ///
    /// # Errors
    ///
    /// Returns error if request fails
    pub async fn send(&self, request: RemoteRequest) -> CoreResult<RemoteResponse> {
        let request_id = request.request_id.clone();

        // In a real implementation, this would use gRPC/HTTP/QUIC
        // For now, simulate a successful response
        let _ = (self.target, self.address.clone(), self.timeout_ms, request);

        // Simulate network delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok(RemoteResponse::success(
            request_id,
            b"executed".to_vec(),
        ))
    }

    /// Set timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Remote executor for handling execution requests
pub struct RemoteExecutor {
    /// Node ID
    node_id: NodeId,
    /// Connected clients
    clients: Arc<RwLock<HashMap<NodeId, RemoteClient>>>,
}

impl RemoteExecutor {
    /// Create a new remote executor
    #[must_use]
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a client connection
    ///
    /// # Errors
    ///
    /// Returns error if add fails
    pub async fn add_client(&self, client: RemoteClient) -> CoreResult<()> {
        let mut clients = self.clients.write().await;
        clients.insert(client.target(), client);
        Ok(())
    }

    /// Remove a client connection
    ///
    /// # Errors
    ///
    /// Returns error if remove fails
    pub async fn remove_client(&self, node_id: NodeId) -> CoreResult<bool> {
        let mut clients = self.clients.write().await;
        Ok(clients.remove(&node_id).is_some())
    }

    /// Get a client by node ID
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn get_client(&self, node_id: NodeId) -> Option<RemoteClient> {
        self.clients.read().await.get(&node_id).cloned()
    }

    /// Execute a request on a remote node
    ///
    /// # Errors
    ///
    /// Returns error if execution fails
    pub async fn execute_remote(&self, target: NodeId, request: RemoteRequest) -> CoreResult<RemoteResponse> {
        let clients = self.clients.read().await;
        let client = clients
            .get(&target)
            .ok_or_else(|| CoreError::NotFound {
                kind: "client".to_string(),
                id: target.to_string(),
            })?;

        client.send(request).await
    }

    /// Broadcast a request to all connected nodes
    ///
    /// # Errors
    ///
    /// Returns error if broadcast fails
    pub async fn broadcast(&self, request: RemoteRequest) -> CoreResult<Vec<RemoteResponse>> {
        let clients = self.clients.read().await;
        let mut responses = Vec::new();

        for client in clients.values() {
            match client.send(request.clone()).await {
                Ok(response) => responses.push(response),
                Err(_) => continue,
            }
        }

        Ok(responses)
    }

    /// Get connected node count
    ///
    /// # Errors
    ///
    /// Returns error if lock acquisition fails
    pub async fn connection_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

impl Default for RemoteExecutor {
    fn default() -> Self {
        Self::new(NodeId::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remote_request_new() {
        let source = NodeId::new();
        let event_id = EventId::new();
        let payload = b"data".to_vec();
        let request = RemoteRequest::new(source, event_id, payload);

        assert_eq!(request.source, source);
        assert_eq!(request.payload, b"data");
    }

    #[tokio::test]
    async fn test_remote_response_success() {
        let response = RemoteResponse::success("req-1".to_string(), b"data".to_vec());
        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.payload, b"data");
    }

    #[tokio::test]
    async fn test_remote_response_error() {
        let response = RemoteResponse::error("req-1".to_string(), "test error".to_string());
        assert!(!response.success);
        assert_eq!(response.error, Some("test error".to_string()));
    }

    #[tokio::test]
    async fn test_remote_client_new() {
        let target = NodeId::new();
        let client = RemoteClient::new(target, "127.0.0.1:8080".to_string());
        assert_eq!(client.target(), target);
        assert_eq!(client.address, "127.0.0.1:8080");
    }

    #[tokio::test]
    async fn test_remote_client_send() {
        let target = NodeId::new();
        let client = RemoteClient::new(target, "127.0.0.1:8080".to_string());

        let source = NodeId::new();
        let event_id = EventId::new();
        let request = RemoteRequest::new(source, event_id, b"test".to_vec());

        let response = client.send(request).await.unwrap();
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_remote_executor_new() {
        let node_id = NodeId::new();
        let executor = RemoteExecutor::new(node_id);
        assert_eq!(executor.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_remote_executor_add_client() {
        let node_id = NodeId::new();
        let executor = RemoteExecutor::new(node_id);

        let target = NodeId::new();
        let client = RemoteClient::new(target, "addr".to_string());
        executor.add_client(client).await.unwrap();

        assert_eq!(executor.connection_count().await, 1);
    }

    #[tokio::test]
    async fn test_remote_executor_get_client() {
        let node_id = NodeId::new();
        let executor = RemoteExecutor::new(node_id);

        let target = NodeId::new();
        let client = RemoteClient::new(target, "addr".to_string());
        executor.add_client(client.clone()).await.unwrap();

        let retrieved = executor.get_client(target).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_remote_executor_remove_client() {
        let node_id = NodeId::new();
        let executor = RemoteExecutor::new(node_id);

        let target = NodeId::new();
        let client = RemoteClient::new(target, "addr".to_string());
        executor.add_client(client).await.unwrap();

        let removed = executor.remove_client(target).await.unwrap();
        assert!(removed);
        assert_eq!(executor.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_remote_executor_execute_remote() {
        let node_id = NodeId::new();
        let executor = RemoteExecutor::new(node_id);

        let target = NodeId::new();
        let client = RemoteClient::new(target.clone(), "addr".to_string());
        executor.add_client(client).await.unwrap();

        let source = NodeId::new();
        let event_id = EventId::new();
        let request = RemoteRequest::new(source, event_id, b"test".to_vec());

        let response = executor.execute_remote(target, request).await.unwrap();
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_transport_error_display() {
        let err = TransportError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("Connection failed"));
    }

    #[test]
    fn test_remote_client_with_timeout() {
        let client = RemoteClient::new(NodeId::new(), "addr".to_string())
            .with_timeout(1000);
        assert_eq!(client.timeout_ms, 1000);
    }
}
