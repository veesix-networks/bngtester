# 4-ubuntu-subscriber-image

**What:** Ubuntu 22.04 (Jammy) subscriber container image with isc-dhcp-client, ppp, and test tools.

**Source Issue:** [#4](https://github.com/veesix-networks/bngtester/issues/4)

## Status

| Phase | Status |
|-------|--------|
| Phase 1: Spec Draft (Claude) | Complete |
| Phase 2: Spec Refinement (Gemini) | Complete |
| Phase 3: Spec Critique (Codex) | Complete |
| Phase 4: Spec Finalization (Claude) | Complete |
| Phase 5: Implementation (Claude) | Complete |
| Phase 6: Post-Implementation Review | Not Started |

## Key Files

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — full spec (finalized)
- [DECISIONS.md](DECISIONS.md) — accepted/rejected findings from reviews
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini spec review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex spec critique

## Dependencies

**Upstream:**
- [1-alpine-subscriber-image](../1-alpine-subscriber-image/) — shared entrypoint
- [3-debian-subscriber-image](../3-debian-subscriber-image/) — dhclient entrypoint fixes

**Downstream:** None currently.

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/PROCESS.md` for the workflow. Continue work on `context/specs/4-ubuntu-subscriber-image/`.
