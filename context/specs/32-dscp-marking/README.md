# 32-dscp-marking

**What:** Add `--dscp` and `--stream-dscp` CLI flags to set DSCP/TOS on outgoing packets via `IP_TOS` socket option.

## Source Issue

[#32](https://github.com/veesix-networks/bngtester/issues/32)

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

- [5-rust-collector](../5-rust-collector/) — base crate with traffic generators

### Downstream

- [33-ecn-marking](../33-ecn-marking/) (planned) — shares TOS byte, builds on DSCP
- [34-per-stream-config](../34-per-stream-config/) (planned) — per-stream overrides extend this pattern

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/32-dscp-marking/` — check the README for current phase status.
