//! Decision proofs for policy verification.

use cathedral_core::{CoreResult, EventId, Hash, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kind of proof
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofKind {
    /// Allow decision
    Allow,
    /// Deny decision
    Deny,
    /// Capability check
    CapabilityCheck,
    /// Policy evaluation
    PolicyEval,
}

/// Proof field
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofField {
    /// Field name
    pub name: String,
    /// Field value
    pub value: Vec<u8>,
}

impl ProofField {
    /// Create a new proof field
    #[must_use]
    pub fn new(name: String, value: Vec<u8>) -> Self {
        Self { name, value }
    }

    /// Create a string field
    #[must_use]
    pub fn string(name: String, value: &str) -> Self {
        Self {
            name,
            value: value.as_bytes().to_vec(),
        }
    }

    /// Create a boolean field
    #[must_use]
    pub fn boolean(name: String, value: bool) -> Self {
        Self {
            name,
            value: value.to_string().as_bytes().to_vec(),
        }
    }
}

/// Decision proof for policy verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionProof {
    /// Unique proof ID
    pub id: String,
    /// Proof kind
    pub kind: ProofKind,
    /// Event that triggered this decision
    pub event_id: Option<EventId>,
    /// Node that made the decision
    pub node_id: Option<NodeId>,
    /// Timestamp
    pub timestamp: u64,
    /// Decision result (true = allowed, false = denied)
    pub decision: bool,
    /// Policy ID used
    pub policy_id: Option<String>,
    /// Proof fields
    pub fields: Vec<ProofField>,
    /// Proof signature (hash of all fields)
    pub signature: Hash,
}

impl DecisionProof {
    /// Create a new decision proof
    #[must_use]
    pub fn new(kind: ProofKind, decision: bool) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            event_id: None,
            node_id: None,
            timestamp,
            decision,
            policy_id: None,
            fields: Vec::new(),
            signature: Hash::empty(),
        }
    }

    /// Set event ID
    #[must_use]
    pub fn with_event(mut self, event_id: EventId) -> Self {
        self.event_id = Some(event_id);
        self
    }

    /// Set node ID
    #[must_use]
    pub fn with_node(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    /// Set policy ID
    #[must_use]
    pub fn with_policy(mut self, policy_id: String) -> Self {
        self.policy_id = Some(policy_id);
        self
    }

    /// Add a field
    #[must_use]
    pub fn with_field(mut self, field: ProofField) -> Self {
        self.fields.push(field);
        self
    }

    /// Finalize and compute signature
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn finalize(mut self) -> CoreResult<Self> {
        // Compute signature from all fields
        let mut data = Vec::new();
        data.extend_from_slice(self.id.as_bytes());
        data.extend_from_slice(format!("{:?}", self.kind).as_bytes());
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        data.extend_from_slice(&(if self.decision { 1u8 } else { 0u8 }).to_be_bytes());

        for field in &self.fields {
            data.extend_from_slice(field.name.as_bytes());
            data.extend_from_slice(&field.value);
        }

        self.signature = Hash::compute(&data);
        Ok(self)
    }

    /// Verify the proof signature
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub fn verify(&self) -> CoreResult<bool> {
        // Recompute signature
        let mut data = Vec::new();
        data.extend_from_slice(self.id.as_bytes());
        data.extend_from_slice(format!("{:?}", self.kind).as_bytes());
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        data.extend_from_slice(&(if self.decision { 1u8 } else { 0u8 }).to_be_bytes());

        for field in &self.fields {
            data.extend_from_slice(field.name.as_bytes());
            data.extend_from_slice(&field.value);
        }

        let computed = Hash::compute(&data);
        Ok(computed == self.signature)
    }

    /// Get a field by name
    #[must_use]
    pub fn get_field(&self, name: &str) -> Option<&ProofField> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get all field names
    #[must_use]
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }
}

/// Proof log for storing decision proofs
pub struct ProofLog {
    /// Stored proofs
    proofs: Vec<DecisionProof>,
    /// Indexed by event ID
    by_event: HashMap<EventId, usize>,
    /// Indexed by node ID
    by_node: HashMap<NodeId, Vec<usize>>,
}

impl ProofLog {
    /// Create a new proof log
    #[must_use]
    pub fn new() -> Self {
        Self {
            proofs: Vec::new(),
            by_event: HashMap::new(),
            by_node: HashMap::new(),
        }
    }

    /// Add a proof to the log
    ///
    /// # Errors
    ///
    /// Returns error if proof is invalid
    pub fn add(&mut self, proof: DecisionProof) -> CoreResult<()> {
        proof.verify()?;

        let idx = self.proofs.len();

        if let Some(event_id) = proof.event_id {
            self.by_event.insert(event_id, idx);
        }

        if let Some(node_id) = proof.node_id {
            self.by_node.entry(node_id).or_default().push(idx);
        }

        self.proofs.push(proof);
        Ok(())
    }

    /// Get proof by index
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&DecisionProof> {
        self.proofs.get(idx)
    }

    /// Get proof by event ID
    #[must_use]
    pub fn get_by_event(&self, event_id: EventId) -> Option<&DecisionProof> {
        self.by_event
            .get(&event_id)
            .and_then(|&idx| self.proofs.get(idx))
    }

    /// Get proofs by node ID
    #[must_use]
    pub fn get_by_node(&self, node_id: NodeId) -> Vec<&DecisionProof> {
        self.by_node
            .get(&node_id)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.proofs.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all proofs
    #[must_use]
    pub fn all(&self) -> &[DecisionProof] {
        &self.proofs
    }

    /// Get proof count
    #[must_use]
    pub fn len(&self) -> usize {
        self.proofs.len()
    }

    /// Check if log is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.proofs.is_empty()
    }

    /// Clear all proofs
    pub fn clear(&mut self) {
        self.proofs.clear();
        self.by_event.clear();
        self.by_node.clear();
    }
}

impl Default for ProofLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Proof builder for constructing proofs
pub struct ProofBuilder {
    proof: DecisionProof,
}

impl ProofBuilder {
    /// Create a new proof builder
    #[must_use]
    pub fn new(kind: ProofKind, decision: bool) -> Self {
        Self {
            proof: DecisionProof::new(kind, decision),
        }
    }

    /// Set event ID
    #[must_use]
    pub fn event(mut self, event_id: EventId) -> Self {
        self.proof = self.proof.with_event(event_id);
        self
    }

    /// Set node ID
    #[must_use]
    pub fn node(mut self, node_id: NodeId) -> Self {
        self.proof = self.proof.with_node(node_id);
        self
    }

    /// Set policy ID
    #[must_use]
    pub fn policy(mut self, policy_id: String) -> Self {
        self.proof = self.proof.with_policy(policy_id);
        self
    }

    /// Add a field
    #[must_use]
    pub fn field(mut self, field: ProofField) -> Self {
        self.proof = self.proof.with_field(field);
        self
    }

    /// Build the proof
    ///
    /// # Errors
    ///
    /// Returns error if building fails
    pub fn build(self) -> CoreResult<DecisionProof> {
        self.proof.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_proof_new() {
        let proof = DecisionProof::new(ProofKind::Allow, true);
        assert!(proof.decision);
    }

    #[test]
    fn test_decision_proof_with_event() {
        let event_id = EventId::new();
        let proof = DecisionProof::new(ProofKind::Allow, true).with_event(event_id);
        assert_eq!(proof.event_id, Some(event_id));
    }

    #[test]
    fn test_proof_field_new() {
        let field = ProofField::new("key".to_string(), b"value".to_vec());
        assert_eq!(field.name, "key");
    }

    #[test]
    fn test_proof_field_string() {
        let field = ProofField::string("key".to_string(), "value");
        assert_eq!(field.value, b"value");
    }

    #[test]
    fn test_proof_field_boolean() {
        let field = ProofField::boolean("key".to_string(), true);
        assert_eq!(field.value, b"true");
    }

    #[test]
    fn test_proof_finalize() {
        let proof = DecisionProof::new(ProofKind::Allow, true)
            .finalize()
            .unwrap();
        assert_ne!(proof.signature, Hash::empty());
    }

    #[test]
    fn test_proof_verify() {
        let proof = DecisionProof::new(ProofKind::Allow, true)
            .finalize()
            .unwrap();
        assert!(proof.verify().unwrap());
    }

    #[test]
    fn test_proof_log_new() {
        let log = ProofLog::new();
        assert!(log.is_empty());
    }

    #[test]
    fn test_proof_log_add() {
        let mut log = ProofLog::new();
        let proof = DecisionProof::new(ProofKind::Allow, true)
            .finalize()
            .unwrap();
        log.add(proof).unwrap();
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_proof_log_get_by_event() {
        let mut log = ProofLog::new();
        let event_id = EventId::new();
        let proof = DecisionProof::new(ProofKind::Allow, true)
            .with_event(event_id)
            .finalize()
            .unwrap();
        log.add(proof).unwrap();

        let found = log.get_by_event(event_id);
        assert!(found.is_some());
    }

    #[test]
    fn test_proof_log_clear() {
        let mut log = ProofLog::new();
        let proof = DecisionProof::new(ProofKind::Allow, true)
            .finalize()
            .unwrap();
        log.add(proof).unwrap();
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn test_proof_builder() {
        let event_id = EventId::new();
        let node_id = NodeId::new();

        let proof = ProofBuilder::new(ProofKind::Allow, true)
            .event(event_id)
            .node(node_id)
            .policy("test-policy".to_string())
            .build()
            .unwrap();

        assert!(proof.decision);
        assert_eq!(proof.event_id, Some(event_id));
        assert_eq!(proof.node_id, Some(node_id));
    }

    #[test]
    fn test_proof_kind() {
        assert_eq!(ProofKind::Allow, ProofKind::Allow);
        assert_ne!(ProofKind::Allow, ProofKind::Deny);
    }
}
