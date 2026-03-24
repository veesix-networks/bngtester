# 33-ecn-marking

**What:** Add ECN marking on outgoing packets (ECT(0)/ECT(1)) and full ECN state detection on received packets via `IP_RECVTOS` + `recvmsg`. Tracks all four ECN codepoints: Not-ECT, ECT(0), ECT(1), CE.

## Source Issue

[#33](https://github.com/veesix-networks/bngtester/issues/33)

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
- [DECISIONS.md](DECISIONS.md) — accepted/rejected findings (8 accepted, 1 rejected)
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini spec review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex spec critique

## Dependencies

### Upstream

- [5-rust-collector](../5-rust-collector/) — base crate
- [32-dscp-marking](../32-dscp-marking/) — TOS byte, socket2, dscp.rs module

### Downstream

None.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/33-ecn-marking/` — check the README for current phase status.
