# Publishing Guide

This document describes how to publish a release of CATHEDRAL.FABRIC.

## Prerequisites

### Required Tools

- **Rust**: 1.85+ with `rustup` installed
- **GitHub CLI** (`gh`): [Installation](https://cli.github.com/)
- **crates.io token**: For publishing packages

### Authentication

#### GitHub CLI
```bash
gh auth login
```

#### crates.io Token
```bash
cargo login
# Paste your API token from https://crates.io/me
```

Set the token in CI as `CARGO_REGISTRY_TOKEN`.

## Version Bump Procedure

1. **Update `Cargo.toml`**
   ```toml
   [workspace.package]
   version = "X.Y.Z"  # Update this
   ```

2. **Update `CHANGELOG.md`**
   - Move items from `[Unreleased]` to `[X.Y.Z]`
   - Add release date
   - Create new `[Unreleased]` section

3. **Commit the changes**
   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "Release X.Y.Z"
   ```

4. **Create and push tag**
   ```bash
   git tag -a vX.Y.Z -m "Release X.Y.Z"
   git push origin main
   git push origin vX.Y.Z
   ```

## Local Release Build

From a clean checkout:

```bash
# Clone fresh
git clone https://github.com/cathedral-fabric/cathedral.fabric.git
cd cathedral.fabric

# Checkout tag
git checkout vX.Y.Z

# Run release script
./scripts/release.sh vX.Y.Z
```

Or manually:

```bash
# Format check
cargo fmt --all -- --check

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Test
cargo test --workspace

# Build release
cargo build --release --workspace

# Package each crate
for crate in cathedral_{core,log,replay,plan,runtime,policy,tool,wasm,storage,cluster,sim,certify}; do
    cd "crates/$crate"
    cargo package --allow-dirty
    cd ../..
done
```

## Dry Run Publish

Verify packages without publishing:

```bash
# Dry run each crate (in dependency order)
cargo publish --dry-run -p cathedral_core
cargo publish --dry-run -p cathedral_log
cargo publish --dry-run -p cathedral_replay
cargo publish --dry-run -p cathedral_plan
cargo publish --dry-run -p cathedral_runtime
cargo publish --dry-run -p cathedral_policy
cargo publish --dry-run -p cathedral_tool
cargo publish --dry-run -p cathedral_wasm
cargo publish --dry-run -p cathedral_storage
cargo publish --dry-run -p cathedral_cluster
cargo publish --dry-run -p cathedral_sim
cargo publish --dry-run -p cathedral_certify
```

## Publishing from CI

The release workflow automatically:

1. Runs all tests
2. Builds release artifacts
3. Creates GitHub release with assets
4. Publishes to crates.io (manual approval required)

To trigger:
- Push a tag: `git push origin vX.Y.Z`
- Or use GitHub Actions UI: `workflows/publish.yml` → Run workflow

## Required CI Secrets

| Secret Name | Purpose |
|-------------|---------|
| `CARGO_REGISTRY_TOKEN` | crates.io publish token |

## Publishing Order

Crates must be published in dependency order:

1. `cathedral_core` — Foundation, no internal dependencies
2. `cathedral_log` — Depends on core
3. `cathedral_replay` — Depends on log, core
4. `cathedral_plan` — Depends on core
5. `cathedral_runtime` — Depends on plan, log, tool, policy, cluster
6. `cathedral_policy` — Depends on core
7. `cathedral_tool` — Depends on core, policy
8. `cathedral_wasm` — Depends on core, tool
9. `cathedral_storage` — Depends on core
10. `cathedral_cluster` — Depends on log, storage
11. `cathedral_sim` — Depends on runtime, cluster
12. `cathedral_certify` — Depends on sim, core

**Binary crates** (no publish needed):
- `cathedral_cli`
- `cathedral_server`
- `cathedral_tui`

## Automated Publish Commands

After the release is created, use these commands:

```bash
# Publish all library crates
for crate in \
  cathedral_core \
  cathedral_log \
  cathedral_replay \
  cathedral_plan \
  cathedral_runtime \
  cathedral_policy \
  cathedral_tool \
  cathedral_wasm \
  cathedral_storage \
  cathedral_cluster \
  cathedral_sim \
  cathedral_certify
do
    echo "Publishing $crate..."
    cargo publish -p "$crate"
    sleep 10  # Wait for crates.io to index
done
```

## Rollback and Yanking

### Crates.io Yanking

If a critical issue is found:

```bash
# Yank a specific version
cargo yank --vers 0.1.0 cathedral_core

# Un-yank (if issue was fixed in same version)
cargo yank --vers 0.1.0 cathedral_core --undo
```

**Yanking policy:**
- Yank only for security issues, data loss, or critical bugs
- Do NOT yank for API breakage (use semver correctly next time)
- Document yank reason in CHANGELOG

### GitHub Release

To remove or update a GitHub release:

```bash
# Delete release (keeps tag)
gh release delete vX.Y.Z

# Or edit release notes
gh release edit vX.Y-Z --notes "Updated notes"
```

## Verification After Release

After publishing, verify:

1. **crates.io pages render correctly**
   - README appears
   - Documentation builds
   - Badge versions update

2. **GitHub release has assets**
   - Source tarball
   - SHA256SUMS

3. **Fresh install works**
   ```bash
   cargo new test_project && cd test_project
   echo 'cathedral-core = "X.Y.Z"' >> Cargo.toml
   cargo build
   ```

4. **CI passes on tag**
   - Check Actions tab for green builds

## Post-Release Tasks

1. **Announce release**
   - GitHub Releases discussion
   - Project mailing list/Discord

2. **Update documentation**
   - Add version-specific notes
   - Update getting started guide

3. **Next version**
   - Update `Cargo.toml` to next version
   - Add `[Unreleased]` to CHANGELOG.md

## Troubleshooting

### "crate already exists" error
- Version was published but Cargo.lock/registry is stale
- Wait 5-10 minutes for crates.io propagation

### "invalid license" error
- Ensure both MIT and Apache-2.0 licenses are in LICENSE file
- Check Cargo.toml `license` field format

### "missing required file" error
- Ensure README.md exists in each published crate
- Ensure LICENSE file is present

### Token issues
- Regenerate token at https://crates.io/me
- Ensure token has "publish" scope
