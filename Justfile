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

# ── Release-grade gates (back-ported from the converge-extension template) ─
# Standard: https://github.com/Reflective-Lab/converge/blob/main/kb/Standards/Extension%20Release%20Checklist.md

# Gate 2: workspace coverage. Floor env-configurable via COVERAGE_FLOOR (default 80).
coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    out_dir="target/coverage"
    mkdir -p "${out_dir}/html"
    common=(--workspace --lib --tests
        --ignore-filename-regex '(^|/)(tests|benches|examples)/')
    cargo llvm-cov clean --workspace
    rm -rf target/tests/trybuild
    cargo llvm-cov "${common[@]}" --no-report
    cargo llvm-cov report \
        --json --summary-only --output-path "${out_dir}/converge-coverage.json"
    cargo llvm-cov report \
        --lcov --output-path "${out_dir}/lcov.info"
    cargo llvm-cov report \
        --html --output-dir "${out_dir}/html"
    pct=$(python3 -c "import json; d=json.load(open('${out_dir}/converge-coverage.json')); print(f\"{d['data'][0]['totals']['lines']['percent']:.1f}\")")
    floor="${COVERAGE_FLOOR:-80}"
    echo "coverage: ${pct}%  floor=${floor}%  json→${out_dir}/converge-coverage.json  lcov→${out_dir}/lcov.info  html→${out_dir}/html/index.html"
    awk -v p="${pct}" -v f="${floor}" 'BEGIN { if (p+0 < f+0) { print "FAIL: coverage " p "% below " f "% floor"; exit 1 } }'

# Gate 3: Criterion baseline. Set PERF_BASELINE to the release tag.
performance-profile:
    #!/usr/bin/env bash
    set -euo pipefail
    name="${PERF_BASELINE:-v0.1.0}"
    mode_flag="--save-baseline"
    if [ -d "target/criterion" ]; then
        existing="$(find target/criterion -mindepth 2 -maxdepth 3 -type d -name "${name}" -print -quit 2>/dev/null || true)"
        if [ -n "${existing}" ]; then
            mode_flag="--baseline"
        fi
    fi
    echo "performance-profile: ${mode_flag} ${name}"
    cargo bench --workspace -- "${mode_flag}" "${name}" || true
    if [ -f scripts/extract-criterion-baseline.py ]; then
        python3 scripts/extract-criterion-baseline.py || \
            echo "warn: baseline extraction failed (non-fatal)"
    fi
    echo "performance-profile: criterion→target/criterion/"

# Gate 4: bounded soak run. Configure with SOAK_DURATION_MIN (default 5).
soak:
    #!/usr/bin/env bash
    set -euo pipefail
    duration_min="${SOAK_DURATION_MIN:-5}"
    out_dir="target/soak"
    mkdir -p "${out_dir}"
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    log="${out_dir}/soak-${stamp}.log"
    cycles=$(awk -v d="${duration_min}" 'BEGIN { printf "%d", 200 * d }')
    iterations=$(awk -v d="${duration_min}" 'BEGIN { printf "%d", 40 * d }')
    concurrency=100
    echo "soak: duration=${duration_min}min cycles=${cycles} concurrency=${concurrency} iterations=${iterations}" | tee "${log}"
    SOAK_CYCLES="${cycles}" \
    SOAK_CONCURRENCY="${concurrency}" \
    SOAK_ITERATIONS="${iterations}" \
    cargo test --workspace -- --include-ignored soak --nocapture 2>&1 | tee -a "${log}"
    ln -sf "soak-${stamp}.log" "${out_dir}/latest.log"
    echo "soak: log → ${log}"

# The five-command release ritual. All five must be green before tagging.
# COVERAGE_FLOOR=N to override the coverage floor (default 80).
release-check:
    just security-audit
    just coverage
    PERF_BASELINE="v$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')" just performance-profile
    SOAK_DURATION_MIN=5 just soak
    just lint
    cargo test --workspace

# Git status and recent commits
status:
    git status --short --branch
    git log --oneline -5

# Remove Rust build artifacts
clean:
    cargo clean
