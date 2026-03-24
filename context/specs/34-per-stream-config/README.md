# 34-per-stream-config

**What:** Per-stream packet size, rate, and traffic pattern overrides via CLI flags. Consolidates per-stream DSCP into a unified `StreamConfigOverride`.

## Source Issue

[#34](https://github.com/veesix-networks/bngtester/issues/34)

## Status

| Phase | Status |
|-------|--------|
| Phase 1 — Spec Draft (Claude) | Complete |
| Phase 2 — Spec Refinement (Gemini) | Complete |
| Phase 3 — Spec Critique (Codex) | Complete |
| Phase 4 — Spec Finalization (Claude) | Complete |
| Phase 5 — Implementation (Claude) | Not Started |
| Phase 6 — Post-Implementation Review | Not Started |

## Key Files

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — full spec (finalized)
- [DECISIONS.md](DECISIONS.md) — accepted findings (9 accepted, 0 rejected)
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini spec review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex spec critique

## Dependencies

### Upstream

- [5-rust-collector](../5-rust-collector/) — base crate
- [32-dscp-marking](../32-dscp-marking/) — per-stream DSCP consolidated into this spec

### Downstream

None.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/34-per-stream-config/` — check the README for current phase status.
