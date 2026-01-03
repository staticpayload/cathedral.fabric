# Contributing to CATHEDRAL.FABRIC

Thank you for your interest in contributing! This document outlines the contribution process.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.85 or later
- Git
- For fuzzing: nightly Rust

### Build

```bash
cargo build --release
```

### Test

```bash
# Run all tests
cargo test --workspace

# Run with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test -p cathedral_core test_name
```

### Development Workflow

1. Fork the repository
2. Create a branch for your feature
3. Make your changes
4. Ensure all tests pass
5. Submit a pull request

## Code Style

### Formatting

We use `rustfmt` with default settings:

```bash
cargo fmt
```

### Linting

We use `clippy` with strict settings:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Testing Requirements

### Before Submitting

Your PR must:

1. Pass all existing tests
2. Include new tests for your changes
3. Have documentation for public APIs
4. Not introduce new clippy warnings

### Test Coverage

Aim for high test coverage:
- Unit tests for each function
- Integration tests for interactions
- Property tests for invariants
- Fuzz tests for parsers

## Documentation

### Public APIs

All public items must have documentation:

```rust
/// Does something important.
///
/// # Arguments
///
/// * `arg1` - Description of arg1
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// * `ErrorType` - When this happens
///
/// # Examples
///
/// ```
/// let result = function(arg1);
/// assert_eq!(result, expected);
/// ```
pub fn public_function(arg1: Type) -> Result<ReturnType> {
    // ...
}
```

### Design Docs

For significant changes:
1. Create an RFC in `rfcs/`
2. Get feedback from maintainers
3. Implement after approval

## Commit Messages

Follow conventional commits:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Test changes
- `chore`: Maintenance tasks

Examples:
```
feat(replay): add partial replay from snapshot

The replay engine now supports starting from any snapshot
boundary, not just the beginning.

Fixes #123
```

```
fix(encoding): ensure stable map ordering

Use BTreeMap instead of HashMap for canonical encoding.
```

## Pull Request Process

### Title

Use the same format as commit messages:
```
feat(scope): brief description
```

### Description

Include:
- What you changed
- Why you changed it
- How you tested it
- Related issues

### Checklist

- [ ] Tests pass
- [ ] Documentation updated
- [ ] No new warnings
- [ ] Added tests for new functionality
- [ ] Updated CHANGELOG.md (if applicable)

## RFC Process

### When to Write an RFC

Write an RFC for:
- New features
- Major architectural changes
- Breaking changes
- Changes to non-goals

### RFC Template

```markdown
# Feature Name

## Summary
One paragraph explanation.

## Motivation
Why are we doing this?

## Goals
- Goal 1
- Goal 2

## Non-Goals
- What this explicitly won't do

## Design
Detailed design description.

## Alternatives
What alternatives were considered?

## Security
Security implications.

## Determinism Impact
How this affects determinism.

## Testing Plan
How this will be tested.

## Rollout Plan
How this will be rolled out.
```

## Performance

### Benchmarks

Add benchmarks for performance-sensitive code:

```rust
#[bench]
fn bench_scheduling(b: &mut test::Bencher) {
    let scheduler = setup_scheduler();
    b.iter(|| {
        scheduler.next_decision()
    });
}
```

Run benchmarks:

```bash
cargo bench
```

### Profiling

For performance investigations:

```bash
cargo flamegraph --bin cathedral -- --bench
```

## Release Process

Releases are done by maintainers:

1. Update version in Cargo.toml
2. Update CHANGELOG.md
3. Create git tag
4. Push to crates.io/github

## Community

### Communication

- GitHub Issues: Bug reports and feature requests
- GitHub Discussions: Questions and ideas
- PRs: Code contributions

### Getting Help

1. Check documentation
2. Search existing issues
3. Ask in GitHub Discussions

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.

## Recognition

Contributors are recognized in:
- CONTRIBUTORS.md
- Release notes
- Git history

Thank you for contributing to CATHEDRAL.FABRIC!
