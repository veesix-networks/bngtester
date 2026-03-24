# 35-multi-subscriber

**What:** Enable bngtester-server to handle multiple concurrent client sessions with per-client metrics, JoinSet supervision, and combined reports.

## Source Issue

[#35](https://github.com/veesix-networks/bngtester/issues/35)

## Status

| Phase | Status |
|-------|--------|
| Phase 1 — Spec Draft (Claude) | Complete |
| Phase 2 — Spec Refinement (Gemini) | Complete |
| Phase 3 — Spec Critique (Codex) | Complete |
| Phase 4 — Spec Finalization (Claude) | Complete |
| Phase 5 — Implementation (Claude) | Complete |
| Phase 6 — Post-Implementation Review | Not Started |

## Key Files

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — full spec (finalized)
- [DECISIONS.md](DECISIONS.md) — accepted findings (11 accepted, 0 rejected)
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini spec review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex spec critique

## Dependencies

### Upstream

- [5-rust-collector](../5-rust-collector/) — base server binary
- [32-dscp-marking](../32-dscp-marking/) — per-stream DSCP
- [33-ecn-marking](../33-ecn-marking/) — ECN counters
- [34-per-stream-config](../34-per-stream-config/) — per-stream config

### Downstream

None.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/35-multi-subscriber/` — check the README for current phase status.
