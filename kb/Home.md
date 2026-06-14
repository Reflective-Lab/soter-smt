---
tags: [moc, soter]
source: mixed
date: 2026-05-14
---
# Soter

Knowledge base for `soter-smt`, the SMT-backed safety and policy assurance
extension for Converge.

`soter` produces searched evidence from SMT queries. It does not decide Cedar
authorization, run analytics, optimize plans, or promote facts.

## Positioning

- [[Positioning]] - elevator pitch, LLM synergy, why formal methods matter now

## Architecture

- [[Architecture/Surface]] - public Rust and Formation-facing surfaces
- [[Architecture/CVC5 FFI Boundary]] - native build/link strategy and unsafe boundary
- [[Architecture/Evidence Semantics]] - status and evidence-tier interpretation
- [[Architecture/Cloud Run Deployment]] - soter-server as M2 of the gRPC suggestor pattern (CVC5 stays server-side; marquee-apps call over gRPC)

## Modules

- [[Modules/Soter]] - crate ownership and entry points

## Planning

- [[Planning/MILESTONES]] - staged implementation plan

## External Context

- CVC5 upstream: https://github.com/cvc5/cvc5
- CVC5 documentation: https://cvc5.github.io/docs/
