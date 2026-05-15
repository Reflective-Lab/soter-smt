---
tags: [architecture, evidence, smt]
source: mixed
date: 2026-05-14
---
# Evidence Semantics

Soter emits searched evidence.

It is stronger than an LLM argument and different from a Cedar runtime
decision. It is still not formal proof unless an independent trusted checker
accepts a proof artifact.

Every `SmtReport` carries Converge's shared `execution_identity`. For native
CVC5 reports this includes linked version, expected and actual checkout commit,
source mode, configure flags, runtime query options, and producer crate version.
This keeps audit/replay from treating the same SMT-LIB query run against
different native bits or solver settings as identical evidence.

## Status Mapping

| Status | Meaning | Suggested claim status |
|---|---|---|
| `Sat` | Solver found a model satisfying the query. | `CounterexampleFound` for invariant-violation queries. |
| `Unsat` | Solver found no model satisfying the query. | `Supported` / `NoCounterexample` for the encoded query. |
| `Unknown` | Solver cannot determine. | `Unknown`. |
| `Timeout` | Solver exceeded budget. | `Timeout`. |
| `Error` | Query or backend failed. | `VerificationFailed` or diagnostic, depending on caller. |

## Important Boundary

`Unsat` means:

```text
No model satisfies this exact SMT encoding.
```

It does not mean:

```text
The natural-language claim is universally true.
```

The trust boundary is the translation from Arbiter/Cedar/product semantics into
SMT-LIB. That translation needs tests and review.

## First Useful Claim

The first claim worth wiring is an Arbiter conditional invariant:

```text
No non-finance principal can commit a high-value expense, even with receipt,
manager approval, and explicit human approval.
```

The SMT query should encode the existence of a violating request. For that
shape:

- `sat` is a concrete counterexample.
- `unsat` supports the invariant for the encoded model.
- `unknown` and `timeout` do not satisfy assurance requirements.

## Relationship To Other Extensions

- Arbiter: runtime Cedar decisions are `Decided`.
- Soter: SMT counterexample search is `Searched`.
- Prism: fuzzy or analytic signals are `Observed` / `Argued` depending on use.
- Mnemos: retrieved facts are source-provenanced knowledge, not proof.
- Lean/Coq/Agda: checked proof artifacts are `Verified`.
