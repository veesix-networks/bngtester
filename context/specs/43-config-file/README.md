# 43-config-file

**What:** Add `--config <PATH>` flag to load test configuration from a YAML file. CLI flags override config values.

## Source Issue

[#43](https://github.com/veesix-networks/bngtester/issues/43)

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
- [DECISIONS.md](DECISIONS.md) — accepted/rejected findings (9 accepted, 1 rejected)
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini spec review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex spec critique

## Dependencies

### Upstream

- [5-rust-collector](../5-rust-collector/) — base crate
- [32-dscp-marking](../32-dscp-marking/) — DSCP config
- [33-ecn-marking](../33-ecn-marking/) — ECN config
- [34-per-stream-config](../34-per-stream-config/) — per-stream overrides
- [35-multi-subscriber](../35-multi-subscriber/) — multi-subscriber server config
- [44-bind-interface](../44-bind-interface/) — bind config

### Downstream

- [45-config-docs](../45-config-docs/) (planned) — documents all config parameters

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/43-config-file/` — check the README for current phase status.
