# soter development commands
# Install: brew install just  |  cargo install just
# Usage:   just --list

set dotenv-load := true

# Show available recipes
default:
    @just --list

# Build pure Rust/default targets
build:
    cargo build --workspace

# Build with native CVC5 enabled. Requires `just deps`.
build-cvc5:
    cargo build --workspace --features converge-soter-smt/cvc5

# Check pure Rust/default targets
check:
    cargo check --workspace

# Check native CVC5 feature. Requires `just deps`.
check-cvc5:
    cargo check --workspace --features converge-soter-smt/cvc5

# Run pure Rust/default tests
test:
    cargo test --workspace

# Run tests with native CVC5 enabled. Requires `just deps`.
test-cvc5:
    cargo test --workspace --features converge-soter-smt/cvc5

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Format code
fmt:
    cargo fmt --all

# Run clippy for default targets
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Formatting plus clippy
lint: fmt-check clippy

# Local release security audit
security-audit:
    cargo audit --deny warnings

# Build pinned native CVC5 dependency
deps:
    make cvc5

# Remove native dependency build artifacts
deps-clean:
    make clean

# Remove native dependency checkout and build artifacts
deps-distclean:
    make distclean

# Generate docs for default targets
doc:
    cargo doc --no-deps --workspace

# Git status and recent commits
status:
    git status --short --branch
    git log --oneline -5

# Remove Rust build artifacts
clean:
    cargo clean
