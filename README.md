# soter

SMT-backed safety and policy assurance for Converge formations.

Cargo package: `converge-soter-smt`. Rust library name: `soter`.

`soter` is a Converge extension. It keeps SMT query modeling, CVC5 native
bindings, solver reports, and solver-backed suggestors outside the Converge
foundation while still using Converge's shared suggestor and proposal
contracts.

## Why It Exists

Converge needs a place for bounded symbolic search that is stronger than LLM
argumentation but not the same thing as formal proof.

Arbiter can decide concrete Cedar requests at runtime. Prism can provide
analytics and fuzzy risk signals. Mnemos can recall domain facts. Soter answers
a different question:

```text
Can any model exist that violates this encoded invariant?
```

That makes Soter useful for high-risk claims such as:

- Can any non-finance actor commit a high-value expense?
- Can any HITL escalation bypass a hard Cedar deny?
- Can a role combination reach both `approve_payment` and `bypass_audit`?
- Can a generated policy context satisfy contradictory constraints?

The result is evidence for the convergence path. It is not promotion authority.

## Name

The extension follows the workspace alias-purpose convention:

```text
soter-smt
```

- `soter` = safety / preservation / assurance.
- `smt` = satisfiability modulo theories, the purpose.

The extension is intentionally not named `cvc5-*`. CVC5 is the first native
backend, not the permanent contract. The same extension may later host Z3,
Yices, Boolector, or SAT-certificate backends if an app pulls on them.

## What Soter Owns

- SMT query and report types.
- SMT-LIB payload validation and stable hashing.
- Solver status vocabulary: `sat`, `unsat`, `unknown`, `timeout`, `error`.
- Evidence tier mapping for SMT results.
- Typed provenance at the proposal boundary.
- `SmtSuggestor`, which reads `SmtQuery` JSON from context and emits
  `SmtReport` proposals.
- Native CVC5 FFI boundary in `crates/cvc5-sys`.
- Formation-facing capability descriptors under `soter.smt`.

## What Soter Does Not Own

- Cedar runtime authorization. That belongs in `arbiter-policy`.
- Analytics or fuzzy inference. That belongs in `prism-analytics`.
- Optimization models. That belongs in `ferrox-solvers`.
- Promotion decisions. Those belong in Converge.
- Formal proof checking. That remains deferred until a checked proof artifact
  path exists.

## Evidence Semantics

Soter reports are `Searched` evidence.

| SMT status | Meaning | Converge interpretation |
|---|---|---|
| `sat` | A satisfying model exists. | Usually `CounterexampleFound` for invariant queries. |
| `unsat` | No satisfying model exists for the encoded query. | High-assurance searched evidence, not formal proof. |
| `unknown` | Solver could not determine the result. | Evidence exists, but should not satisfy hard assurance requirements. |
| `timeout` | Solver exceeded its budget. | Operational failure or inconclusive evidence. |
| `error` | Query/backend failed. | Diagnostic only. |

Do not label SMT results as `Verified`. `Verified` should be reserved for a
checked artifact from Lean, Coq, Agda, Ethos, or another trusted checker.

## Repository Layout

```text
soter-smt/
  crates/
    cvc5-sys/       unsafe native FFI/link boundary for CVC5
    soter/          safe SMT query/report/suggestor surface
  kb/               local architecture and planning notes
  Makefile          pinned CVC5 source build, explicit only
  Justfile          developer commands
```

## Native Backend Strategy

Soter follows the Ferrox pattern:

- Default builds do not require native solver dependencies.
- Native code is isolated in a `*-sys` crate.
- Native linking is opt-in through a feature.
- Dependency builds are explicit through `make` / `just deps`.
- Ordinary CI can use fake backends.
- Real solver CI can run nightly/manual until the query shape is worth gating.

The first CVC5 FFI milestone links CVC5, retrieves the native version string,
and checks SMT-LIB input through the C API. The safe `Cvc5FfiBackend` maps
native `sat`, `unsat`, `unknown`, `timeout`, and `error` results into
`SmtReport` values while keeping raw pointers inside `crates/cvc5-sys`.

## Why Build From Source

CVC5 publishes prebuilt binaries and source releases. For product assurance we
want the Ferrox-style source-build path because it gives us:

- pinned tag,
- local reproducibility,
- explicit native build artifacts,
- one place to audit native dependency flags,
- one FFI boundary to keep unsafe code contained.

The current pinned tag is:

```text
cvc5-1.3.3
```

That tag is the latest stable release visible on the upstream GitHub release
page at the time this scaffold was created.

## Building

Default build, no native CVC5:

```bash
just check
just test
just lint
```

Build pinned CVC5 from source:

```bash
just deps
```

Then check and test the native feature:

```bash
just check-cvc5
just test-cvc5
```

The source build disables LibPoly by default through
`CVC5_CONFIGURE_FLAGS=--no-poly`. That keeps the first FFI path portable on
current macOS toolchains and is enough for the initial policy/invariant work.
Set `CVC5_CONFIGURE_FLAGS` explicitly if a later invariant needs a different
CVC5 configuration.

If CVC5 is already installed elsewhere, set:

```bash
export SOTER_CVC5_ROOT=/path/to/cvc5/install-prefix
just check-cvc5
```

`SOTER_CVC5_ROOT` must contain:

```text
include/cvc5/c/cvc5.h
lib/libcvc5.*
```

or a platform-equivalent `lib64/` directory.

## Current Public Surface

```rust
use soter::{FakeSmtBackend, SmtQuery, SmtSuggestor};

let query = SmtQuery::new(
    "arbiter.expense.non_finance_commit",
    "(set-logic QF_LIA)\n(check-sat)",
);

let suggestor = SmtSuggestor::new(FakeSmtBackend::unsat());
```

Native CVC5 execution, behind the `cvc5` feature:

```rust
use soter::{Cvc5FfiBackend, SmtBackend, SmtQuery};

let query = SmtQuery::new(
    "arbiter.expense.non_finance_commit",
    "(set-logic QF_LIA)\n(assert false)\n(check-sat)",
);

let report = Cvc5FfiBackend.solve(&query).await?;
assert_eq!(report.solver, "cvc5");
```

Formation discovery:

```rust
use soter::formation_capabilities;

for capability in formation_capabilities() {
    println!("{} -> {}", capability.id, capability.surface);
}
```

Stable capability family:

```text
soter.smt
```

Current capability IDs:

| Capability | Surface | Evidence tier |
|---|---|---|
| `soter.smt.solver` | `SmtSuggestor` | `searched` |
| `soter.smt.cvc5_ffi` | `Cvc5FfiBackend` under `cvc5` | `searched` |

## Integration With Arbiter

The first useful product pull should be conditional Arbiter invariant queries.

Bad first query:

```text
Is the whole policy always denying all modeled requests?
```

Useful first query:

```text
Does there exist a modeled request where:
  principal is non-finance
  action is commit
  resource type is expense
  amount > 5000
  receipt gate is present
  manager approval gate is present
  human approval is present
  Cedar-modeled decision is allow?
```

For that query:

- `sat` means Soter found a counterexample to the invariant.
- `unsat` means no counterexample exists in the encoded model.
- `unknown` / `timeout` means no assurance should be granted.

The hard part is the encoding. Soter does not magically understand Cedar. A
product or Arbiter adapter must compile the relevant policy semantics into a
sound SMT query.

## CI Policy

Initial policy:

- Required PR/push CI: default Rust tests with fake backend.
- Optional native CI: `just check-cvc5` after `just deps`.
- Nightly/manual CI: real CVC5 solver tests after native linking is stable.
- Required real CVC5 invariant CI: deferred until conditional Arbiter queries
  encode actual claims.

This mirrors the Arbiter CVC5 policy: fake solver required, real solver
scheduled/manual until the query is semantically useful.

## Safety Boundary

Unsafe code is allowed only in `crates/cvc5-sys`.

The safe crate must expose:

- owned Rust strings and values,
- explicit error types,
- no raw pointers,
- deterministic report payloads,
- stable hashes for replay and audit.

## Roadmap

1. Scaffold repo, docs, typed reports, fake backend, suggestor surface.
2. Prove CVC5 native link on macOS and Linux with a version smoke test.
3. Add a narrow FFI solving API for SMT-LIB payloads.
4. Add one Arbiter conditional invariant query fixture.
5. Wire nightly/manual real CVC5 CI for solver-backed fixtures.
6. Promote real CVC5 to required CI only for selected high-risk invariant
   classes.
7. Consider independent proof checking only if CVC5 proof artifacts become a
   product requirement.

## Project Files

- [AGENTS.md](AGENTS.md) - agent entrypoint and boundary rules.
- [CHANGELOG.md](CHANGELOG.md) - release notes.
- [kb/Home.md](kb/Home.md) - local knowledge base.
- [Makefile](Makefile) - pinned native CVC5 build.
- [Justfile](Justfile) - developer commands.

## License

MIT - see [LICENSE](LICENSE).
