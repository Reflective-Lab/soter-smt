---
tags: [positioning, pitch, formal-methods]
source: llm
date: 2026-06-12
---
# Positioning

Why Soter exists, why it plays well with LLMs, and why formal methods matter
more than ever. Companion pitches live in the Ferrox and Arbiter knowledge
bases; this note is the Soter chapter of the same story.

## Elevator Pitch

Soter is the **adversarial imagination of the Converge platform**: an
SMT-backed assurance extension that asks the question no test suite can —
*can any model exist, anywhere in the space of possibilities, that violates
this invariant?* Where Arbiter decides the request in front of it and Ferrox
finds the best plan, Soter hunts for the request nobody thought to write a
test for.

It wraps native CVC5 bindings (`crates/cvc5-sys`) behind a typed Rust
contract — `SmtQuery` in, `SmtReport` out — with a strict five-word status
vocabulary (`sat`, `unsat`, `unknown`, `timeout`, `error`) and SHA-256-hashed
SMT-LIB payloads, so every result is pinned to exactly what was asked.

The name is the thesis: *Soter* — safety, preservation — with SMT as the
means, not the identity. CVC5 is the first backend, not the contract; Z3,
Yices, or SAT-certificate backends can slot in behind the same surface.

## Why It Plays Well With LLMs

An LLM can argue that a policy is safe; Soter can *search every modeled
possibility* and either produce the concrete counterexample or report that
none exists. That is the missing rung between "the model says it's fine" and
mathematical proof: **bounded symbolic search stronger than LLM argumentation
but not the same thing as formal proof.**

In an agentic loop:

1. The LLM formulates the high-risk claim ("can any role combination reach
   both `approve_payment` and `bypass_audit`?").
2. Soter exhausts the encoded model.
3. A `sat` result hands back a concrete, replayable counterexample the LLM
   can explain in plain language; `unsat` is high-assurance searched
   evidence.

Soter is epistemically honest by construction:

- Results are `Searched` evidence — never relabeled `Verified`. That tier is
  reserved for checked artifacts from Lean, Coq, Agda, Ethos, or an
  equivalent trusted checker.
- `unknown` and `timeout` explicitly do not satisfy hard assurance gates.
- The solver feeds the Converge promotion path; it never *is* the promotion
  path.

## Why Formal Methods Matter More Than Ever

**Code generation has been industrialized, but assurance has not.** When LLMs
write the policies, the configs, and the glue, the volume of
plausible-looking artifacts explodes while the human review budget stays
flat — and plausibility is exactly the failure mode formal methods exist to
kill.

Testing samples the input space; an attacker — or an agent following
instructions a bit too literally — searches it. SMT solving (CDCL(T) over
decidable theories) searches it back, exhaustively within the model.

The Soter–Arbiter bridge is the worked proof of the approach: Cedar policies
are symbolically compiled (SymCC) into SMT and executed through Soter's
native CVC5 FFI, so the invariant "no non-finance actor can commit a
high-value expense, even with every approval present" is checked against the
*real* policy model, not a hand-coded abstraction.

The honest caveat that makes the whole stack credible: an `unsat` only covers
the *encoded* model. **Model adequacy, not proof technology, is the hard
part** — which is why the assurance lane runs Gherkin fixtures and
mutant-policy tests before the solver ever fires.

In an era where software increasingly writes software, the scarce resource is
not generation — it is *justified confidence*, and Soter is the machine that
manufactures it.

## Boundaries (One-Line Reminders)

- Arbiter answers: *should this concrete request be allowed now?* (`Decided`)
- Ferrox answers: *what is the best feasible plan?* (`Searched`, optimization)
- Soter answers: *can any modeled request violate this invariant?*
  (`Searched`, symbolic)
- A future Lean lane answers: *can this invariant be checked as a theorem?*
  (`Verified`)
