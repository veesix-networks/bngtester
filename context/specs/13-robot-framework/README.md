# 13-robot-framework

**What:** Robot Framework test runner with standalone subscriber tests and BNG integration tests using the lab/ topology.

## Source Issue

[#13](https://github.com/veesix-networks/bngtester/issues/13)

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

- [1-alpine-subscriber-image](../1-alpine-subscriber-image/) — subscriber images under test
- [3-debian-subscriber-image](../3-debian-subscriber-image/) — Debian image for matrix testing
- [4-ubuntu-subscriber-image](../4-ubuntu-subscriber-image/) — Ubuntu image for matrix testing
- [22-mgmt-iface-awareness](../22-mgmt-iface-awareness/) — MGMT_IFACE tested in integration suite
- [27-containerlab-topology](../27-containerlab-topology/) — lab/ topology used by integration tests

### Downstream

- [5-rust-collector](../5-rust-collector/) (planned) — collector tests will use Robot Framework

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/13-robot-framework/` — check the README for current phase status.
