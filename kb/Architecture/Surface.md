---
tags: [architecture, surface, smt]
source: mixed
date: 2026-05-14
---
# Surface

`soter` exposes one canonical safe crate and one native sys crate:

| Package | Library | Role |
|---|---|---|
| `converge-soter-smt` | `soter` | Safe SMT query, report, backend, suggestor, and capability surface. |
| `converge-soter-cvc5-sys` | `soter_cvc5_sys` | Unsafe native CVC5 FFI/link boundary. |

## Public Safe Surface

- `SmtQuery` - deterministic SMT-LIB query payload with timeout and artifact
  options.
- `SmtReport` - searched evidence report with solver, status, hash, model,
  unsat core, and diagnostics fields.
- `SmtStatus` - `Sat`, `Unsat`, `Unknown`, `Timeout`, `Error`.
- `SmtEvidenceTier` - currently always `Searched`.
- `SmtBackend` - async backend trait.
- `FakeSmtBackend` - deterministic CI/test backend.
- `Cvc5FfiBackend` - native CVC5 SMT-LIB backend behind the `cvc5` feature.
- `SmtSuggestor` - Converge suggestor that reads queries and proposes reports.
- `ProvenanceSource` - typed proposal-boundary provenance vocabulary.
- `formation_capabilities()` - Formation-facing `soter.smt` catalog.

## Formation Capability IDs

- `soter.smt.solver`
- `soter.smt.cvc5_ffi`

## Feature Flags

| Feature | Meaning |
|---|---|
| default | Safe types, fake backend, and suggestor surface only. |
| `cvc5` | Links `converge-soter-cvc5-sys` against native CVC5 and enables `Cvc5FfiBackend`. |

Default builds must stay native-free.

## Context Boundary

`SmtSuggestor` defaults to:

```text
ContextKey::Seeds -> ContextKey::Evaluations
```

Inputs are serialized `SmtQuery` values. Outputs are serialized `SmtReport`
values with `soter` provenance.

See also: [[Architecture/CVC5 FFI Boundary]],
[[Architecture/Evidence Semantics]]
