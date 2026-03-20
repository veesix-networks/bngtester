# 27-containerlab-topology

**What:** Containerlab topology deploying osvbng as BNG with a bngtester subscriber and FRR-based server for end-to-end IPoE validation.

## Source Issue

[#27](https://github.com/veesix-networks/bngtester/issues/27)

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

- [1-alpine-subscriber-image](../1-alpine-subscriber-image/) — subscriber container image used in the topology
- [22-mgmt-iface-awareness](../22-mgmt-iface-awareness/) — `MGMT_IFACE` env var used to remove management default route

### Downstream

- [13-robot-framework-tests](../13-robot-framework-tests/) (planned) — Robot tests will reference this topology
- [5-rust-collector](../5-rust-collector/) (planned) — collector needs a real BNG path for end-to-end testing

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/27-containerlab-topology/` — check the README for current phase status.
