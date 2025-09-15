#!/bin/bash
# RED Phase: Test expectations for CI/CD pipeline (Integration-First)
# These tests WILL FAIL initially - that's the point!

set -e
echo "🔴 RED Phase: Testing CI/CD Pipeline Expectations"

# Test 1: GitHub Actions workflow exists
echo "Test 1: Checking for GitHub Actions workflow..."
if [ ! -f ".github/workflows/ci.yml" ]; then
    echo "❌ FAIL: No CI workflow found at .github/workflows/ci.yml"
    exit 1
fi

# Test 2: Workflow contains required jobs
echo "Test 2: Checking workflow structure..."
if ! grep -q "name: CI" ".github/workflows/ci.yml"; then
    echo "❌ FAIL: Workflow missing 'name: CI'"
    exit 1
fi

if ! grep -q "cargo test" ".github/workflows/ci.yml"; then
    echo "❌ FAIL: Workflow missing 'cargo test' command"
    exit 1
fi

if ! grep -q "cargo clippy" ".github/workflows/ci.yml"; then
    echo "❌ FAIL: Workflow missing 'cargo clippy' command"
    exit 1
fi

if ! grep -q "cargo fmt.*check" ".github/workflows/ci.yml"; then
    echo "❌ FAIL: Workflow missing 'cargo fmt --check' command"
    exit 1
fi

# Test 3: Rust toolchain properly configured
echo "Test 3: Checking Rust toolchain configuration..."
if ! grep -q "actions-rs/toolchain\|dtolnay/rust-toolchain" ".github/workflows/ci.yml"; then
    echo "❌ FAIL: No Rust toolchain action configured"
    exit 1
fi

# Test 4: Multi-platform testing (for GREEN phase)
echo "Test 4: Checking for multi-platform matrix..."
if ! grep -q "strategy:" ".github/workflows/ci.yml"; then
    echo "⚠️ WARNING: No build matrix found (acceptable for YELLOW phase)"
fi

echo "✅ All CI/CD pipeline requirements validated!"
echo "🎯 Ready for YELLOW phase implementation"