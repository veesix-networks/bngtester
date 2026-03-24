# 33-ecn-marking

**What:** Add ECN marking on outgoing packets (ECT(0)/ECT(1)) and CE mark detection on received packets via `IP_RECVTOS` + `recvmsg`.

## Source Issue

[#33](https://github.com/veesix-networks/bngtester/issues/33)

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
- [32-dscp-marking](../32-dscp-marking/) — TOS byte, socket2, dscp.rs module

### Downstream

None.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/33-ecn-marking/` — check the README for current phase status.
