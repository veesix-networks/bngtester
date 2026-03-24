# 34-per-stream-config

**What:** Per-stream packet size, rate, and traffic pattern overrides via `--stream-size`, `--stream-rate`, `--stream-pattern` CLI flags.

## Source Issue

[#34](https://github.com/veesix-networks/bngtester/issues/34)

## Status

| Phase | Status |
|-------|--------|
| Phase 1 — Spec Draft (Claude) | Complete |
| Phase 2 — Spec Refinement (Gemini) | Not Started |
| Phase 3 — Spec Critique (Codex) | Not Started |
| Phase 4 — Spec Finalization (Claude) | Not Started |
| Phase 5 — Implementation (Claude) | Not Started |
| Phase 6 — Post-Implementation Review | Not Started |

## Key Files

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — full spec

## Dependencies

### Upstream

- [5-rust-collector](../5-rust-collector/) — base crate
- [32-dscp-marking](../32-dscp-marking/) — per-stream DSCP pattern this extends

### Downstream

None.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/34-per-stream-config/` — check the README for current phase status.
