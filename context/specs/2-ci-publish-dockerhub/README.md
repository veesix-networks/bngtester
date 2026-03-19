# 2-ci-publish-dockerhub

**CI pipeline to build and publish subscriber images to Docker Hub**

## Source Issue

[#2](https://github.com/veesix-networks/bngtester/issues/2)

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

- [1-alpine-subscriber-image](../1-alpine-subscriber-image/) — provides the first image and shared entrypoint
- [3-debian-subscriber-image](../3-debian-subscriber-image/) — provides the Debian image
- [4-ubuntu-subscriber-image](../4-ubuntu-subscriber-image/) — provides the Ubuntu image

### Downstream

None yet. Future image specs will inherit the CI pipeline from this spec.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Resume work on `context/specs/2-ci-publish-dockerhub/` — check the README for current phase status.
