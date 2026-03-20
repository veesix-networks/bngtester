# 5-rust-collector

**What:** Rust crate producing `bngtester-server` and `bngtester-client` binaries for traffic generation and measurement with sub-microsecond precision, RRUL bufferbloat detection, and multi-stream concurrent testing.

## Source Issue

[#5](https://github.com/veesix-networks/bngtester/issues/5)

## Status

| Phase | Status |
|-------|--------|
| Phase 1 — Spec Draft (Claude) | Complete |
| Phase 2 — Spec Refinement (Gemini) | Complete |
| Phase 3 — Spec Critique (Codex) | Complete |
| Phase 4 — Spec Finalization (Claude) | Complete |
| Phase 5 — Implementation (Claude) | Not Started |
| Phase 6 — Post-Implementation Review | Not Started |

## Key Files

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — full spec (finalized)
- [DECISIONS.md](DECISIONS.md) — accepted/rejected findings (15 accepted, 2 rejected)
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini spec review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex spec critique

## Dependencies

### Upstream

- [1-alpine-subscriber-image](../1-alpine-subscriber-image/) — Dockerfile to modify
- [3-debian-subscriber-image](../3-debian-subscriber-image/) — Dockerfile to modify
- [4-ubuntu-subscriber-image](../4-ubuntu-subscriber-image/) — Dockerfile to modify
- [2-ci-publish-dockerhub](../2-ci-publish-dockerhub/) — CI pipeline needs build context update

### Downstream

- [13-robot-framework-tests](../13-robot-framework-tests/) (planned) — test runner that invokes bngtester-client/server

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/5-rust-collector/` — check the README for current phase status.
