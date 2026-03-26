# 50-cgnat-reporting

**What:** CGNAT-aware reporting showing both translated (peer) and real (subscriber) addresses.

## Source Issue

[#50](https://github.com/veesix-networks/bngtester/issues/50)

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

- [35-multi-subscriber](../35-multi-subscriber/) — ClientReport struct, combined reports
- [44-bind-interface](../44-bind-interface/) — source_ip in HelloMsg

### Downstream

None.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/50-cgnat-reporting/` — check the README for current phase status.
