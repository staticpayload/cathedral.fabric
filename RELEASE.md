# Release Checklist

This checklist must be completed for every release.

## Pre-Release

### Code Quality
- [ ] All tests pass: `cargo test --workspace`
- [ ] Formatting OK: `cargo fmt --all -- --check`
- [ ] Clippy clean: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Security audit passes: `cargo audit`
- [ ] Fuzzing CI passes (for relevant releases)

### Documentation
- [ ] CHANGELOG.md updated with version date
- [ ] All new APIs documented
- [ ] Migration guide added (if breaking)
- [ ] README.md reflects new features
- [ ] ARCHITECTURE.md updated (if needed)

### Version
- [ ] Version bumped in `Cargo.toml`
- [ ] Version matches semver rules:
  - MAJOR: Breaking changes
  - MINOR: New features, backwards compatible
  - PATCH: Bug fixes only
- [ ] No `-alpha`, `-beta`, `-rc` in stable release

## Release Build

### Artifacts
- [ ] Source tarball built: `cathedral-fabric-X.Y.Z.tar.gz`
- [ ] SHA256SUMS file generated
- [ ] Tarball verifies against checksum

### Package Verification
- [ ] All library crates `cargo package` successfully
- [ ] All packages include README.md
- [ ] All packages include LICENSE
- [ ] Dry-run publish passes for each crate

## Tag Creation

### Git
- [ ] Working tree clean: `git status`
- [ ] On main branch
- [ ] Changelog committed
- [ ] Tag created: `git tag -a vX.Y.Z -m "Release X.Y.Z"`
- [ ] Tag annotated (not lightweight)
- [ ] Tag message includes CHANGELOG excerpt
- [ ] Tag pushed: `git push origin vX.Y.Z`

## GitHub Release

### Creation
- [ ] Release created from tag
- [ ] Title: "Release X.Y.Z"
- [ ] Notes include CHANGELOG section
- [ ] Source tarball attached
- [ ] SHA256SUMS attached
- [ ] Release marked as latest (if stable)

### Metadata
- [ ] GitHub description set
- [ ] Topics updated
- [ ] Milestone closed (if applicable)

## Registry Publishing

### crates.io
Publish in dependency order:
- [ ] `cathedral_core`
- [ ] `cathedral_log`
- [ ] `cathedral_replay`
- [ ] `cathedral_plan`
- [ ] `cathedral_runtime`
- [ ] `cathedral_policy`
- [ ] `cathedral_tool`
- [ ] `cathedral_wasm`
- [ ] `cathedral_storage`
- [ ] `cathedral_cluster`
- [ ] `cathedral_sim`
- [ ] `cathedral_certify`

### Verification
- [ ] Each crate page renders on crates.io
- [ ] README appears on crate page
- [ ] Documentation link works
- [ ] Badge versions updated

## Post-Release

### Announcements
- [ ] GitHub release announcement
- [ ] Release notes published
- [ ] Tags/mailing lists notified

### Next Version
- [ ] Version bumped to next dev version in Cargo.toml
- [ ] New `[Unreleased]` added to CHANGELOG.md
- [ ] New milestone created (if applicable)

## Emergency Rollback

If critical issues found:
- [ ] Issue published to GitHub
- [ ] Crate yanked (if security/data-loss): `cargo yank --vers X.Y.Z crate`
- [ ] Rollback announcement published
- [ ] Fix release prepared
