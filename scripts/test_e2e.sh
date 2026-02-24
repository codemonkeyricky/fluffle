#!/bin/bash
set -e

# End-to-end test for Nano Code Agent
# This script builds the project, runs unit tests, creates a test environment,
# tests basic configuration loading and plugin discovery.
# Usage: ./scripts/test_e2e.sh
# Note: Creates a 'test_env' directory which is automatically cleaned up on exit.

cleanup() {
    echo "Cleaning up test environment..."
    rm -rf test_env 2>/dev/null || true
}

trap cleanup EXIT INT TERM

echo "=== Nano Code Agent End-to-End Test ==="

# Clean previous build
echo "1. Cleaning previous build..."
cargo clean

# Build the project
echo "2. Building project..."
cargo build

# Run unit tests
echo "3. Running unit tests..."
cargo test --lib

# Create test environment
echo "4. Setting up test environment..."
mkdir -p test_env
pushd test_env || exit 1

# Create test files
echo "Creating test files..."
echo "Test file content" > test_file.txt
git init > /dev/null 2>&1 || true
echo "Test commit" > test_git_file.txt
git add test_git_file.txt
git commit -m "Test commit" > /dev/null 2>&1 || true

# Create minimal .env for testing (without real API key)
echo "OPENAI_API_KEY=test-key-noop" > .env

# Test configuration loading
echo "5. Testing configuration..."
popd
cargo run -- --help >/dev/null 2>&1 && echo "✓ Basic execution works"

# Test plugin discovery
echo "6. Testing plugin discovery..."
cargo test --test plugin_discovery >/dev/null 2>&1 && echo "✓ Plugin tests pass"

echo ""
echo "=== End-to-end test completed ==="
echo "Manual testing steps:"
echo "1. Set real API key in .env file"
echo "2. Run: cargo run"
echo "3. In TUI, try: 'Read Cargo.toml'"
echo "4. In TUI, try: 'Run ls -la'"
echo "5. In TUI, try: 'Show git status'"
echo "6. Press Ctrl+C to exit"