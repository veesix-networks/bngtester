# 49-continuous-mode

**What:** Continuous/resilient operating mode — `--duration 0` or `--continuous` runs indefinitely, UDP resilient to loss, TCP control reconnect, failover metrics.

## Source Issue

[#49](https://github.com/veesix-networks/bngtester/issues/49)

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

- [5-rust-collector](../5-rust-collector/) — base client/server with generator loop
- [35-multi-subscriber](../35-multi-subscriber/) — server handles reconnects as new sessions
- [43-config-file](../43-config-file/) — YAML config for continuous/max_reconnects

### Downstream

None.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/49-continuous-mode/` — check the README for current phase status.
