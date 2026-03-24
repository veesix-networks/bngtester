# 35-multi-subscriber

**What:** Enable bngtester-server to handle multiple concurrent client sessions with per-client metrics and combined reports.

## Source Issue

[#35](https://github.com/veesix-networks/bngtester/issues/35)

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

- [5-rust-collector](../5-rust-collector/) — base server binary
- [32-dscp-marking](../32-dscp-marking/) — per-stream DSCP in reports
- [33-ecn-marking](../33-ecn-marking/) — ECN counters in reports
- [34-per-stream-config](../34-per-stream-config/) — per-stream config in reports

### Downstream

None.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/35-multi-subscriber/` — check the README for current phase status.
