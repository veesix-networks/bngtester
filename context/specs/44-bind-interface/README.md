# 44-bind-interface

**What:** Add `--bind-iface`, `--source-ip`, and `--control-bind-ip` flags for bare metal and loopback BNG testing.

## Source Issue

[#44](https://github.com/veesix-networks/bngtester/issues/44)

## Status

| Phase | Status |
|-------|--------|
| Phase 1 — Spec Draft (Claude) | Complete |
| Phase 2 — Spec Refinement (Gemini) | Not Started |
| Phase 3 — Spec Critique (Codex) | Not Started |
| Phase 4 — Spec Finalization (Claude) | Complete |
| Phase 5 — Implementation (Claude) | Complete |
| Phase 6 — Post-Implementation Review | Not Started |

## Key Files

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — full spec (finalized)
- [DECISIONS.md](DECISIONS.md) — accepted findings (9 accepted, 0 rejected)
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini spec review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex spec critique

## Dependencies

### Upstream

- [5-rust-collector](../5-rust-collector/) — base crate with socket2 generators
- [32-dscp-marking](../32-dscp-marking/) — socket2 pattern for pre-connect socket setup

### Downstream

- [43-config-file](../43-config-file/) (planned) — config file will include bind settings
- [45-config-docs](../45-config-docs/) (planned) — documentation for bind parameters

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/44-bind-interface/` — check the README for current phase status.
