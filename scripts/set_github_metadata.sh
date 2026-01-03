#!/usr/bin/env bash
# set_github_metadata.sh
#
# Set GitHub repository metadata using GitHub CLI.
# This script requires: gh auth login
#
# Usage: ./scripts/set_github_metadata.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

# Check if gh is installed and authenticated
if ! command -v gh &> /dev/null; then
    echo "Error: GitHub CLI (gh) is not installed"
    echo "Install from: https://cli.github.com/"
    exit 1
fi

if ! gh auth status &> /dev/null; then
    echo "Error: GitHub CLI is not authenticated"
    echo "Run: gh auth login"
    exit 1
fi

# Get the repository name
REPO=${GITHUB_REPOSITORY:-$(git config --get remote.origin.url | sed 's/.*:\(.*\)\.git/\1/' || echo "cathedral-fabric/cathedral.fabric")}

echo "Setting metadata for repository: $REPO"

# Set description (one sentence)
gh repo edit "$REPO" \
  --description "A deterministic, distributed, capability-safe execution fabric for agent workflows with verifiable replay and certified audit trails"

# Set homepage (documentation URL)
gh repo edit "$REPO" \
  --homepage "https://cathedral-fabric.github.io/fabric"

# Set topics
gh repo edit "$REPO" \
  --add-topic "deterministic" \
  --add-topic "distributed-systems" \
  --add-topic "workflow-engine" \
  --add-topic "agent-framework" \
  --add-topic "replay" \
  --add-topic "audit-trail" \
  --add-topic "capability-based-security" \
  --add-topic "wasm" \
  --add-topic "raft" \
  --add-topic "hash-chain" \
  --add-topic "verifiable-computing" \
  --add-topic "rust" \
  --add-topic "terminal-ui" \
  --add-topic "cluster" \
  --add-topic "consensus" \
  --add-topic "sandbox" \
  --add-topic "policy-engine" \
  --add-topic "event-sourcing" \
  --add-topic "cqrs" \
  --add-topic "content-addressable-storage" \
  --add-topic "blake3" \
  --add-topic "ed25519"

echo "✓ Repository metadata updated"
echo ""
echo "Repository: https://github.com/$REPO"
echo "Description set"
echo "Topics set (25 total)"
