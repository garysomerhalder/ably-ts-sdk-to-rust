# PROD-005: Set up GitHub Actions CI/CD Pipeline

## 🎯 Objective
Implement comprehensive CI/CD pipeline for automated testing, building, and releasing across all platforms.

## 📋 Task Details

**Priority:** 🔴 CRITICAL (Zero automation currently)
**Effort:** 3 days
**Assignee:** DevOps/Rust Engineer
**Dependencies:** None

## 🔍 Problem Analysis

Currently ZERO automation - everything is manual. This blocks safe releases and quality assurance.

## ✅ Acceptance Criteria

1. [ ] CI runs on every push/PR
2. [ ] Tests run on Linux, Windows, macOS
3. [ ] WASM and Node.js builds automated
4. [ ] Release artifacts generated automatically
5. [ ] Code coverage reporting
6. [ ] Dependency security scanning

## 🛠️ Implementation

### `.github/workflows/ci.yml`
```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  RUST_BACKTRACE: 1
  ABLY_API_KEY_SANDBOX: ${{ secrets.ABLY_API_KEY_SANDBOX }}

jobs:
  test:
    name: Test Suite
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, beta]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${{ matrix.rust }}
      - uses: Swatinem/rust-cache@v2

      - name: Run tests
        run: cargo test --all-features --workspace

      - name: Run clippy
        run: cargo clippy --all-features -- -D warnings

      - name: Check formatting
        run: cargo fmt --all -- --check

  build-wasm:
    name: Build WASM
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: jetli/wasm-pack-action@v0.4.0

      - name: Build WASM package
        run: |
          cd ably-wasm
          wasm-pack build --target web --out-dir pkg

      - name: Upload WASM artifacts
        uses: actions/upload-artifact@v3
        with:
          name: wasm-package
          path: ably-wasm/pkg/

  build-node:
    name: Build Node.js Bindings
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        node: [18, 20]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node }}

      - name: Install dependencies
        run: |
          cd ably-node
          npm ci

      - name: Build native module
        run: |
          cd ably-node
          npm run build

      - name: Run Node.js tests
        run: |
          cd ably-node
          npm test

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
          fail_ci_if_error: true

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v1.4.1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

### `.github/workflows/release.yml`
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    outputs:
      upload_url: ${{ steps.create_release.outputs.upload_url }}
    steps:
      - uses: actions/checkout@v4

      - name: Create Release
        id: create_release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          draft: false
          prerelease: false

  build-binaries:
    name: Build Binaries
    needs: create-release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package
        shell: bash
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ably-rust-${{ matrix.target }}.tar.gz libably_core.* ably-ffi.*

      - name: Upload Release Asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: ./target/${{ matrix.target }}/release/ably-rust-${{ matrix.target }}.tar.gz
          asset_name: ably-rust-${{ matrix.target }}.tar.gz
          asset_content_type: application/gzip

  publish-crates:
    name: Publish to crates.io
    needs: build-binaries
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Publish ably-core
        run: |
          cd ably-core
          cargo publish --token ${{ secrets.CRATES_TOKEN }}

  publish-npm:
    name: Publish to NPM
    needs: build-binaries
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          registry-url: 'https://registry.npmjs.org'

      - name: Publish Node.js bindings
        run: |
          cd ably-node
          npm ci
          npm run build
          npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

## 📊 Success Metrics

- ✅ CI runs < 10 minutes
- ✅ 100% test pass rate
- ✅ Code coverage > 80%
- ✅ Zero security vulnerabilities
- ✅ Automated releases work