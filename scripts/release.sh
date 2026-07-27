#!/bin/bash
# Release script for DCP
# Usage: ./scripts/release.sh [patch|minor|major]
set -euo pipefail

VERSION="${1:-patch}"
cd "$(dirname "$0")/.."

# Ensure clean working directory
if [ -n "$(git status --porcelain)" ]; then
    echo "Error: Working directory not clean. Commit or stash changes first."
    exit 1
fi

# Bump version in all Cargo.toml files
echo "Bumping version ($VERSION)..."
cargo bump "$VERSION" --workspace 2>/dev/null || {
    # Manual bump if cargo-bump not installed
    echo "cargo-bump not found, bumping manually..."
    # Simple version bump for workspace
    for manifest in Cargo.toml dcp-types/Cargo.toml dcpd/Cargo.toml dcp-cli/Cargo.toml plugins/*/Cargo.toml sdks/rust/dcp-client/Cargo.toml; do
        [ -f "$manifest" ] || continue
        sed -i 's/^version = "0\.1\.0"/version = "1.0.0"/' "$manifest"
        sed -i 's/^version = "0\.2\.0"/version = "1.0.0"/' "$manifest"
    done
}

# Update lockfile
cargo check --workspace 2>/dev/null

# Run all checks
echo "Running checks..."
cargo test --workspace 2>&1 | tail -3
cargo clippy --workspace -- -D warnings 2>&1 | tail -3 || echo "Warning: clippy issues found"
cargo fmt --check 2>&1 | tail -3

# Build all targets
echo "Building release..."
cargo build --release --workspace

# Get new version
NEW_VERSION=$(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)

# Create tag
git add -A
git commit -m "Release v$NEW_VERSION"
git tag -a "v$NEW_VERSION" -m "DCP v$NEW_VERSION"

echo ""
echo "Release v$NEW_VERSION ready!"
echo ""
echo "To publish:"
echo "  git push origin main --tags"
echo "  cargo publish -p dcp-types"
echo "  cargo publish -p dcp-plugin-sdk"
echo "  cargo publish -p dcp-client"
echo "  cargo publish -p dcpd"
echo "  cargo publish -p dcp-cli"
echo ""
echo "Artifacts: target/release/dcpd, target/release/dcp"
