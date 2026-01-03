# Determinism Certification

## Overview

Certification proves that a run is deterministic and reproducible across platforms.

## Certification Levels

### Level 1: Hash Chain Valid

All events have valid hash chain:

```bash
cathedral certify -b run-001.cath-bundle --level 1
```

Checks:
- Hash chain continuity
- Content hash verification
- Event ordering correctness

### Level 2: Cross-Platform Reproducible

Run reproduces on different platforms:

```bash
cathedral certify -b run-001.cath-bundle --level 2 --platforms linux,macos,windows
```

Checks:
- Level 1 checks
- Replay on Linux produces same state
- Replay on macOS produces same state
- Replay on Windows produces same state
- All byte-for-byte identical

### Level 3: Formal Verification

Formal verification of critical properties:

```bash
cathedral certify -b run-001.cath-bundle --level 3
```

Checks:
- Level 2 checks
- Model checker verifies properties
- TLA+ spec validated

## Certification Process

### Step 1: Load Bundle

```rust
pub struct Certifier {
    bundle: ReplayBundle,
    config: CertifyConfig,
}

impl Certifier {
    pub fn load(bundle_path: &Path) -> Result<Self, CertifyError> {
        let bundle = ReplayBundle::load(bundle_path)?;
        Ok(Self {
            bundle,
            config: CertifyConfig::default(),
        })
    }
}
```

### Step 2: Validate Hash Chain

```rust
impl Certifier {
    pub fn validate_hash_chain(&self) -> Result<HashChainReport, CertifyError> {
        let mut validator = HashChainValidator::new();
        let mut report = HashChainReport::new();

        for event in &self.bundle.events {
            match validator.validate(event) {
                Ok(_) => {}
                Err(e) => {
                    report.add_violation(HashChainViolation {
                        event_id: event.event_id,
                        error: e,
                    });
                }
            }
        }

        if report.has_violations() {
            Err(CertifyError::HashChainInvalid { report })
        } else {
            Ok(report)
        }
    }
}
```

### Step 3: Replay and Verify

```rust
impl Certifier {
    pub async fn replay_and_verify(&self) -> Result<ReplayReport, CertifyError> {
        let mut replay = ReplayEngine::new(&self.bundle)?;
        let result = replay.replay().await?;

        // Verify state hashes match
        for (i, state) in result.states.iter().enumerate() {
            let event = &self.bundle.events[i];
            if let Some(expected) = event.post_state_hash {
                if state.hash() != expected {
                    return Err(CertifyError::StateHashMismatch {
                        event_index: i,
                        expected,
                        actual: state.hash(),
                    });
                }
            }
        }

        Ok(ReplayReport {
            events_processed: self.bundle.events.len(),
            final_state: result.final_state,
            divergences: result.divergences,
        })
    }
}
```

### Step 4: Cross-Platform Check

```rust
impl Certifier {
    pub async fn check_cross_platform(
        &self,
        platforms: &[Platform],
    ) -> Result<CrossPlatformReport, CertifyError> {
        let mut results = Vec::new();

        for platform in platforms {
            let result = self.replay_on_platform(*platform).await?;
            results.push((*platform, result));
        }

        // Check all results match
        let first = &results[0].1;
        for (platform, result) in &results[1..] {
            if result != first {
                return Err(CertifyError::PlatformMismatch {
                    platform: *platform,
                    difference: result.diff(first),
                });
            }
        }

        Ok(CrossPlatformReport { results })
    }

    async fn replay_on_platform(
        &self,
        platform: Platform,
    ) -> Result<ReplayResult, CertifyError> {
        // In practice, this would run on actual target platform
        // For CI, we use docker or VMs
        todo!()
    }
}
```

## Certification Output

### Report Format

```json
{
    "bundle_id": "bundle_abc123",
    "run_id": "run_001",
    "certified_at": "2025-01-15T10:30:00Z",
    "level": 2,
    "result": "PASS",
    "checks": {
        "hash_chain": "PASS",
        "replay": "PASS",
        "cross_platform": "PASS"
    },
    "platforms": [
        {"os": "linux", "arch": "x86_64", "result": "PASS"},
        {"os": "macos", "arch": "x86_64", "result": "PASS"},
        {"os": "macos", "arch": "aarch64", "result": "PASS"},
        {"os": "windows", "arch": "x86_64", "result": "PASS"}
    ],
    "metrics": {
        "events": 1234,
        "duration_ms": 234,
        "peak_memory_mb": 64
    },
    "signature": "sig_def456..."
}
```

### Certificate File

```bash
cathedral certify -b run-001.cath-bundle --output cert.json
```

Produces a certificate file that can be verified independently:

```bash
cathedral verify-cert cert.json
```

## Verification

### Verify Certificate

```rust
pub fn verify_certificate(cert_path: &Path) -> Result<VerifyResult, VerifyError> {
    let cert = Certificate::load(cert_path)?;

    // Check signature
    if !cert.verify_signature()? {
        return Err(VerifyError::InvalidSignature);
    }

    // Check bundle still exists
    if !cert.bundle_path.exists() {
        return Err(VerifyError::BundleNotFound);
    }

    // Re-run certification checks
    let certifier = Certifier::load(&cert.bundle_path)?;
    let current_result = certifier.certify()?;

    if current_result != cert.result {
        return Err(VerifyError::ResultChanged);
    }

    Ok(VerifyResult::Valid)
}
```

## CI Integration

### GitHub Action

```yaml
name: certify

on:
  push:
    branches: [main]
  pull_request:

jobs:
  certify:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Certify
        run: |
          cathedral certify -b test-bundles/run-001.cath-bundle \
            --level 2 \
            --output cert-${{ matrix.os }}.json
      - name: Upload certificate
        uses: actions/upload-artifact@v3
        with:
          name: certificate-${{ matrix.os }}
          path: cert-${{ matrix.os }}.json
```

## Formal Verification

### TLA+ Specs

Critical protocols have TLA+ specs:

```tla
---- MODULE EventLog ----
EXTENDS Naturals, Sequences

VARIABLES log, stateHash

TypeOK == /\ log \in Seq(Event)
          /\ stateHash \in Hashes

HashChainValid == \A i \in 1..Len(log):
    log[i].priorHash =
        IF i = 1 THEN InitHash
        ELSE log[i-1].postHash

====
```

### Model Checker Integration

```bash
# Run model checker
cd specs
python model_check.py event_log.tla

# Output in CI
echo "::notice::EventLog model check passed"
```

## Testing

### Certification Tests

```rust
#[tokio::test]
async fn test_certify_deterministic_run() {
    let bundle = create_deterministic_bundle().await;

    let certifier = Certifier::load(bundle.path()).unwrap();
    let result = certifier.certify().await.unwrap();

    assert!(result.passed());
    assert_eq!(result.level, 2);
}

#[tokio::test]
async fn test_certify_detects_nondeterminism() {
    let bundle = create_nondeterministic_bundle().await;

    let certifier = Certifier::load(bundle.path()).unwrap();
    let result = certifier.certify().await;

    assert!(result.is_err());
    assert!(matches!(result, Err(CertifyError::ReplayDiverged(_))));
}
```

## Performance

- Certification time: O(events)
- Memory usage: O(largest state)
- Parallelization: Cross-platform checks in parallel

## Best Practices

1. **Certify before release** - All bundles should be certified
2. **Store certificates** - Keep certificates with bundles
3. **Periodic re-certification** - Detect regressions
4. **Sign certificates** - Use signatures for distribution
5. **Version specs** - Keep formal specs in sync with code
