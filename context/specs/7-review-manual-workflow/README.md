# 7-review-manual-workflow

**What:** Audit workflow consistency across issues #1-#6 and produce decisions on automation tooling, hosting, and Phase 4 automation design.

## Source Issue

[#7](https://github.com/veesix-networks/bngtester/issues/7)

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

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — audit findings, automation design, and decisions

## Dependencies

### Upstream

- [0-bootstrap](../0-bootstrap/) — defines the workflow being audited
- [1-alpine-subscriber-image](../1-alpine-subscriber-image/) — audited spec
- [2-ci-publish-dockerhub](../2-ci-publish-dockerhub/) — audited spec
- [3-debian-subscriber-image](../3-debian-subscriber-image/) — audited spec
- [4-ubuntu-subscriber-image](../4-ubuntu-subscriber-image/) — audited spec

### Downstream

Future n8n implementation issue (to be filed at end of Phase 5).

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/7-review-manual-workflow/` — check the README for current phase status.
