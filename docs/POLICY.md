# Policy System Specification

## Overview

The CATHEDRAL policy system controls all runtime decisions: what tools can run, what data can be accessed, and what side effects are allowed. All decisions are logged with proof objects.

## Policy Language

### Syntax

```policy
# Comments start with #

policy "name" {
    # Metadata
    version: "1.0.0"
    description: "Example policy"

    # Default allow/deny
    default: deny

    # Capability grants
    grant NetRead {
        allowlist: ["*.trusted.com", "api.service.com"]
    }

    grant FsRead {
        prefixes: ["./data", "./config"]
    }

    # Capability denies
    deny NetWrite

    # Rules with conditions
    rule "api_access" {
        match {
            tool: "http_fetch"
        }
        allow {
            domains: ["api.service.com"]
        }
        require {
            capability: NetRead
        }
    }

    rule "output_write" {
        match {
            tool: "write_file"
        }
        allow {
            prefixes: ["./outputs"]
        }
        redact {
            fields: ["api_key", "secret"]
        }
    }

    # Rate limits
    rate_limit "api_calls" {
        max_per_minute: 60
        applies_to: ["http_fetch", "http_post"]
    }

    # Multi-tenant boundaries
    tenant "user_a" {
        allow_prefixes: ["./data/user_a"]
        deny_prefixes: ["./data/user_b"]
    }
}
```

### AST Types

```rust
pub struct PolicyAst {
    pub name: String,
    pub version: String,
    pub rules: Vec<Rule>,
    pub grants: Vec<CapabilityGrant>,
    pub denies: Vec<CapabilityDeny>,
    pub rate_limits: Vec<RateLimit>,
    pub tenants: Vec<TenantPolicy>,
}

pub struct Rule {
    pub name: String,
    pub match_expr: MatchExpr,
    pub allow: Option<AllowExpr>,
    pub deny: Option<DenyExpr>,
    pub require: Vec<Capability>,
    pub redactions: Vec<RedactionRule>,
}

pub enum MatchExpr {
    Tool { name: String },
    Capability { cap: Capability },
    And { left: Box<MatchExpr>, right: Box<MatchExpr> },
    Or { left: Box<MatchExpr>, right: Box<MatchExpr> },
}
```

## Policy Compiler

```rust
pub struct PolicyCompiler {
    validators: Vec<Box<dyn PolicyValidator>>,
}

impl PolicyCompiler {
    pub fn compile(&self, ast: PolicyAst) -> Result<CompiledPolicy, CompileError> {
        // Type check
        self.validate_types(&ast)?;

        // Check for conflicts
        self.check_conflicts(&ast)?;

        // Build decision tree
        let decision_tree = self.build_decision_tree(&ast)?;

        Ok(CompiledPolicy {
            ast,
            decision_tree,
            redactions: self.extract_redactions(&ast),
        })
    }
}
```

## Decision Engine

```rust
pub struct PolicyEngine {
    compiled: CompiledPolicy,
}

impl PolicyEngine {
    pub fn decide(&self, context: &MatchContext) -> DecisionProof {
        // Evaluate rules in order
        for rule in &self.compiled.ast.rules {
            if self.matches(&rule.match_expr, context) {
                return self.evaluate_rule(rule, context);
            }
        }

        // Check grants
        for grant in &self.compiled.ast.grants {
            if self.matches_grant(grant, context) {
                return DecisionProof {
                    decision_id: DecisionId::new(),
                    allowed: true,
                    rule: None,
                    grant: Some(grant.clone()),
                    reasoning: Reasoning::GrantedCapability,
                    timestamp: Timestamp::now(),
                };
            }
        }

        // Check denies (deny takes precedence)
        for deny in &self.compiled.ast.denies {
            if self.matches_deny(deny, context) {
                return DecisionProof {
                    decision_id: DecisionId::new(),
                    allowed: false,
                    rule: None,
                    deny: Some(deny.clone()),
                    reasoning: Reasoning::DeniedCapability,
                    timestamp: Timestamp::now(),
                };
            }
        }

        // Default
        DecisionProof {
            decision_id: DecisionId::new(),
            allowed: self.compiled.ast.default == DefaultDecision::Allow,
            rule: None,
            reasoning: Reasoning::Default,
            timestamp: Timestamp::now(),
        }
    }
}
```

## Decision Proof

Every policy decision produces a proof:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionProof {
    pub decision_id: DecisionId,
    pub allowed: bool,
    pub rule: Option<String>,
    pub grant: Option<CapabilityGrant>,
    pub deny: Option<CapabilityDeny>,
    pub reasoning: Reasoning,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Reasoning {
    GrantedCapability,
    DeniedCapability,
    RuleMatch { rule_name: String, explanation: String },
    Default,
    Conflict { conflicts: Vec<String> },
}
```

### Proof Example

```json
{
    "decision_id": "dec_abc123def456",
    "allowed": true,
    "rule": "api_access",
    "grant": null,
    "deny": null,
    "reasoning": {
        "RuleMatch": {
            "rule_name": "api_access",
            "explanation": "Tool http_fetch matched rule, domain api.service.com in allowlist"
        }
    },
    "timestamp": "2025-01-15T10:30:00Z"
}
```

## Redaction

Rules specify redactions for logs and bundles:

```policy
rule "sensitive_api" {
    match { tool: "api_call" }
    redact {
        fields: ["api_key", "password", "token"]
        patterns: ["Bearer .*"]
    }
}
```

```rust
pub struct Redactor {
    rules: Vec<RedactionRule>,
}

impl Redactor {
    pub fn redact(&self, value: &Value) -> Value {
        let mut redacted = value.clone();

        for rule in &self.rules {
            match rule {
                RedactionRule::Field { field } => {
                    redacted.remove_field(field);
                    redacted.insert_field(field, Value::String("***REDACTED***"));
                }
                RedactionRule::Pattern { pattern } => {
                    // Apply regex replacement
                }
            }
        }

        redacted
    }
}
```

## Rate Limiting

```policy
rate_limit "api" {
    max_per_minute: 60
    max_per_hour: 1000
    applies_to: ["http_fetch", "http_post"]
}
```

```rust
pub struct RateLimiter {
    limits: HashMap<String, RateLimit>,
    counters: HashMap<String, TokenBucket>,
}

impl RateLimiter {
    pub fn check(&mut self, tool: &str) -> RateLimitResult {
        if let Some(limit) = self.limits.get(tool) {
            self.counters
                .entry(tool.to_string())
                .or_insert_with(|| TokenBucket::new(limit))
                .try_acquire()
        } else {
            RateLimitResult::Allowed
        }
    }
}
```

## Multi-Tenancy

```policy
tenant "user_a" {
    allow_prefixes: ["./data/user_a"]
    deny_prefixes: ["./data/user_b", "./data/system"]
}

tenant "user_b" {
    allow_prefixes: ["./data/user_b"]
    deny_prefixes: ["./data/user_a", "./data/system"]
}
```

## Policy Composition

Policies can be composed:

```rust
pub fn compose(policies: Vec<CompiledPolicy>) -> CompiledPolicy {
    // Merge rules with precedence
    // Later policies override earlier ones
    // Detect and report conflicts
}
```

## Testing

### Property Tests

```rust
#[proptest]
fn test_policy_deterministic(policies: Vec<PolicyAst>, context: MatchContext) {
    let engine = PolicyEngine::new(&policies);
    let proof1 = engine.decide(&context);
    let proof2 = engine.decide(&context);

    assert_eq!(proof1, proof2);  // Same inputs → same decision
}
```

### Fuzzing

```rust
fuzz_target!(|data: &[u8]| {
    if let Ok(policy) = parse_policy(data) {
        let engine = PolicyEngine::new(&policy);
        // Check no crashes on various contexts
    }
});
```

## CLI Usage

```bash
# Validate policy file
cathedral policy validate policy.cath

# Test policy against scenarios
cathedral policy test policy.cath --scenario scenarios/

# Explain a decision
cathedral policy explain --decision-id dec_abc...
```
