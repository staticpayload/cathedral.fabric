#!/usr/bin/env bash
# release.sh
#
# Prepare and create a release for CATHEDRAL.FABRIC.
# This script validates, builds, tags, and publishes a release.
#
# Usage: ./scripts/release.sh [version]
#   version: Version tag (e.g., v0.1.0). Defaults to reading from Cargo.toml.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
    VERSION="v$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | awk '{print $3}' | tr -d '"')"
fi

if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-z0-9]+)?$ ]]; then
    echo "Error: Invalid version format: $VERSION"
    echo "Expected format: vX.Y.Z or vX.Y.Z-rc1"
    exit 1
fi

VERSION_ONLY="${VERSION#v}"

echo "========================================="
echo " CATHEDRAL.FABRIC Release Preparation"
echo "========================================="
echo "Version: $VERSION"
echo "Repository root: $REPO_ROOT"
echo ""

cd "$REPO_ROOT"

# Step 1: Pre-flight checks
echo "[1/8] Pre-flight checks..."

# Check for uncommitted changes
if [[ -n "$(git status --porcelain)" ]]; then
    echo "Error: Uncommitted changes detected"
    git status --short
    exit 1
fi

# Check we're on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$CURRENT_BRANCH" != "main" ]] && [[ "$CURRENT_BRANCH" != "master" ]]; then
    echo "Warning: Not on main/master branch (current: $CURRENT_BRANCH)"
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check gh CLI
if ! command -v gh &> /dev/null; then
    echo "Error: GitHub CLI (gh) is not installed"
    exit 1
fi

echo "✓ Pre-flight checks passed"

# Step 2: Format check
echo "[2/8] Checking formatting..."
cargo fmt --all -- --check
echo "✓ Formatting OK"

# Step 3: Lint check
echo "[3/8] Running linter..."
cargo clippy --all-targets --all-features -- -D warnings
echo "✓ Linting passed"

# Step 4: Run tests
echo "[4/8] Running tests..."
cargo test --workspace
echo "✓ Tests passed"

# Step 5: Verify version consistency
echo "[5/8] Verifying version consistency..."

CARGO_TOML_VERSION=$(grep '^version = ' "$REPO_ROOT/Cargo.toml" | head -1 | awk '{print $3}' | tr -d '"')
if [[ "$VERSION_ONLY" != "$CARGO_TOML_VERSION" ]]; then
    echo "Error: Version mismatch"
    echo "  Tag version: $VERSION_ONLY"
    echo "  Cargo.toml: $CARGO_TOML_VERSION"
    exit 1
fi

echo "✓ Version consistent: $VERSION_ONLY"

# Step 6: Build release artifacts
echo "[6/8] Building release artifacts..."

# Build all crates
cargo build --release --workspace

# Create source tarball
ARCHIVE_NAME="cathedral-fabric-${VERSION_ONLY}"
git archive --format=tar.gz --prefix="${ARCHIVE_NAME}/" -o "${ARCHIVE_NAME}.tar.gz" "HEAD"

# Generate checksums
sha256sum "${ARCHIVE_NAME}.tar.gz" > SHA256SUMS
sha256sum "${ARCHIVE_NAME}.tar.gz" | awk '{print $1}' > "${ARCHIVE_NAME}.tar.gz.sha256"

echo "✓ Release artifacts built:"
echo "  - ${ARCHIVE_NAME}.tar.gz"
echo "  - SHA256SUMS"

# Step 7: Verify package (dry-run)
echo "[7/8] Verifying packages for crates.io..."

# Check that all library crates have proper metadata
for crate in crates/*/; do
    if [[ -f "$crate/Cargo.toml" ]] && [[ -f "$crate/src/lib.rs" ]]; then
        echo "  Checking $(basename "$crate")..."
        cd "$crate"

        # Verify package can be built
        cargo package --allow-dirty

        # Verify package contents
        PKG_NAME="$(cargo read-manifest | jq -r .name)"
        PKG_VERSION="$(cargo read-manifest | jq -r .version)"

        # Check that critical files are included
        if ! cargo package --list | grep -q "Cargo.toml"; then
            echo "    Error: Cargo.toml not in package"
            exit 1
        fi

        echo "    ✓ $PKG_NAME v$PKG_VERSION"
        cd "$REPO_ROOT"
    fi
done

echo "✓ Package verification passed"

# Step 8: Create git tag
echo "[8/8] Creating git tag..."

if git rev-parse "$VERSION" &>/dev/null; then
    echo "Warning: Tag $VERSION already exists"
    read -p "Delete and recreate? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git tag -d "$VERSION"
        git push origin ":refs/tags/$VERSION" || true
    else
        echo "Aborting release"
        exit 1
    fi
fi

# Create annotated tag
git tag -a "$VERSION" -m "Release $VERSION

$(awk '/## \['"$VERSION_ONLY"'\]/,/^## / {print}' "$REPO_ROOT/CHANGELOG.md" | head -n -2 | tail -n +3)"

echo "✓ Tag $VERSION created"
echo ""

# Summary
echo "========================================="
echo " Release Ready!"
echo "========================================="
echo ""
echo "To complete the release:"
echo ""
echo "1. Push the tag:"
echo "   git push origin $VERSION"
echo ""
echo "2. Create GitHub release:"
echo "   gh release create $VERSION \\"
echo "     --title \"Release $VERSION\" \\"
echo "     --notes \"CHANGELOG.md\" \\"
echo "     --attach ${ARCHIVE_NAME}.tar.gz#${ARCHIVE_NAME}.tar.gz \\"
echo "     --attach SHA256SUMS#SHA256SUMS"
echo ""
echo "3. Publish to crates.io:"
echo "   cargo publish -p cathedral_core"
echo "   cargo publish -p cathedral_log"
echo "   cargo publish -p cathedral_replay"
echo "   cargo publish -p cathedral_plan"
echo "   cargo publish -p cathedral_runtime"
echo "   cargo publish -p cathedral_policy"
echo "   cargo publish -p cathedral_tool"
echo "   cargo publish -p cathedral_wasm"
echo "   cargo publish -p cathedral_storage"
echo "   cargo publish -p cathedral_cluster"
echo "   cargo publish -p cathedral_sim"
echo "   cargo publish -p cathedral_certify"
echo ""
echo "4. Update CHANGELOG.md:"
echo "   Move [Unreleased] to new [$VERSION_ONLY] section"
echo "   Add new [Unreleased] section at top"
echo ""
