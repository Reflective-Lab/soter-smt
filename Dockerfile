# ─── Stage 1: Build CVC5 ──────────────────────────────────────────────────────
# Mirrors the `make cvc5` target. CVC5 pinned to cvc5-1.3.3 (commit
# 8ff882e3...). `--no-poly` matches Makefile's CVC5_CONFIGURE_FLAGS default.
FROM debian:trixie-slim AS cpp-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake git ca-certificates \
    python3 python3-pip python3-venv \
    libgmp-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

ARG CVC5_TAG=cvc5-1.3.3
RUN git clone --depth 1 --branch ${CVC5_TAG} \
      https://github.com/cvc5/cvc5 /build/cvc5 && \
    cd /build/cvc5 && \
    ./configure.sh production \
      --prefix=/build/cvc5/build/install \
      --auto-download \
      --no-poly && \
    cmake --build /build/cvc5/build -j"$(nproc)" && \
    cmake --install /build/cvc5/build

# ─── Stage 2: Compile Rust server ─────────────────────────────────────────────
FROM rust:1.96-trixie AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake clang libclang-dev protobuf-compiler \
    git ca-certificates pkg-config libgmp-dev \
    && rm -rf /var/lib/apt/lists/*

# Bring the CVC5 install tree into the builder. soter-cvc5-sys's build.rs
# reads SOTER_CVC5_ROOT and links against $SOTER_CVC5_ROOT/lib.
COPY --from=cpp-builder /build/cvc5/build/install /opt/cvc5

WORKDIR /workspace

# Build context is the soter-smt repo root.
COPY . /workspace/

# Clone sibling Reflective platform repos that [patch.crates-io] in
# Cargo.toml references. Mirrors the pattern from
# mosaic-extensions/ferrox-solvers/Dockerfile (M1 fixup 312605d).
RUN mkdir -p /bedrock-platform && \
    git clone --depth=1 --quiet https://github.com/Reflective-Lab/converge.git /bedrock-platform/converge

RUN sed -i \
      -e 's|path = "../../bedrock-platform/|path = "/bedrock-platform/|g' \
      /workspace/Cargo.toml && \
    rm -f /workspace/Cargo.lock

ENV SOTER_CVC5_ROOT=/opt/cvc5

# soter-cvc5-sys's build.rs asserts headers exist — fail fast if missing.
RUN test -f /opt/cvc5/include/cvc5/c/cvc5.h \
    || (echo "CVC5 headers missing at /opt/cvc5"; exit 1)

RUN cargo build --release \
    --package converge-soter-server --features converge-soter-server/full

# ─── Stage 3: Minimal runtime ─────────────────────────────────────────────────
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 libgmp10 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Ship libcvc5 + its deps alongside the binary.
COPY --from=cpp-builder /build/cvc5/build/install/lib /usr/local/lib

RUN ldconfig

COPY --from=rust-builder \
     /workspace/target/release/soter-server \
     /usr/local/bin/soter-server

VOLUME ["/tls"]
EXPOSE 50051

ENV SOTER_ADDR=0.0.0.0:50051
ENV SOTER_TLS_CERT=/tls/server.crt
ENV SOTER_TLS_KEY=/tls/server.key

ENTRYPOINT ["/usr/local/bin/soter-server"]
