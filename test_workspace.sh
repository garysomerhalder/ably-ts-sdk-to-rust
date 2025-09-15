#!/bin/bash
# RED Phase: Tests that should fail initially

echo "🔴 RED Phase: Testing Rust workspace structure..."

# Test 1: Check if root Cargo.toml exists
if [ -f "Cargo.toml" ]; then
    echo "✅ Root Cargo.toml exists"
else
    echo "❌ Root Cargo.toml missing"
    exit 1
fi

# Test 2: Check if workspace is configured
if grep -q "\[workspace\]" Cargo.toml 2>/dev/null; then
    echo "✅ Workspace configuration found"
else
    echo "❌ Workspace configuration missing"
    exit 1
fi

# Test 3: Check if core crate exists
if [ -d "ably-core" ] && [ -f "ably-core/Cargo.toml" ]; then
    echo "✅ Core crate structure exists"
else
    echo "❌ Core crate missing"
    exit 1
fi

# Test 4: Check if workspace builds
if cargo build 2>/dev/null; then
    echo "✅ Workspace builds successfully"
else
    echo "❌ Workspace build failed"
    exit 1
fi

echo "🟢 All tests passed!"