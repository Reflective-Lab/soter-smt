---
tags: [architecture, cvc5, ffi]
source: mixed
date: 2026-05-14
---
# CVC5 FFI Boundary

Soter follows the Ferrox native dependency pattern:

- keep unsafe code isolated in a `*-sys` crate,
- require explicit native dependency builds,
- keep default checks independent of native libraries,
- link native backends only through opt-in features.

## Native Crate

```text
crates/cvc5-sys/
```

The sys crate owns:

- locating an installed CVC5 prefix,
- compiling a tiny C++ wrapper,
- linking `libcvc5`,
- copying runtime libraries into Cargo output where needed,
- exposing raw/native functions to the safe crate.

Unsafe code is allowed only here.

## Build Source

The native build recipe pins:

```text
CVC5_TAG := cvc5-1.3.3
```

The recipe clones:

```text
https://github.com/cvc5/cvc5
```

into:

```text
vendor/cvc5
```

and installs into:

```text
vendor/cvc5/build/install
```

The default recipe passes:

```text
--no-poly
```

This avoids platform-specific LibPoly failures in the first FFI path. The
initial Arbiter use is policy/invariant search, not nonlinear polynomial
reasoning. Re-enable or customize through `CVC5_CONFIGURE_FLAGS` when a concrete
invariant needs it.

## Environment Override

Set `SOTER_CVC5_ROOT` to use an existing install prefix:

```bash
export SOTER_CVC5_ROOT=/opt/cvc5
just check-cvc5
```

The prefix must contain CVC5 headers and libraries:

```text
include/cvc5/c/cvc5.h
lib/libcvc5.*
```

## Current FFI Scope

The current native surface exposes:

- linked CVC5 version retrieval,
- SMT-LIB parse and `check-sat` execution,
- status mapping for `sat`, `unsat`, `unknown`, `timeout`, and `error`,
- optional `sat` model extraction,
- optional `unsat` unsat-core extraction.

Next FFI slices:

1. richer timeout diagnostics and cancellation behavior,
2. `check-sat-assuming` coverage beyond status parsing,
3. proof artifact extraction only if a product needs checked proof evidence.

## Non-Goals

- Do not build CVC5 during ordinary `cargo check`.
- Do not hide native downloads inside a build script.
- Do not expose raw pointers from the safe crate.
- Do not call CVC5 results `Verified`.

See also: [[Architecture/Evidence Semantics]]
