# 1-alpine-subscriber-image

**What:** Alpine Linux subscriber image with a shared entrypoint supporting IPoE (DHCPv4/DHCPv6) and PPPoE over configurable VLAN encapsulation.

**Source Issue:** [#1](https://github.com/veesix-networks/bngtester/issues/1)

## Status

| Phase | Status |
|-------|--------|
| Phase 0: Issue | Complete |
| Phase 1: Spec Draft (Claude) | Complete |
| Phase 2: Spec Refinement (Gemini) | Complete |
| Phase 3: Spec Critique (Codex) | Complete |
| Phase 4: Spec Finalization (Claude) | Complete |
| Phase 5: Implementation (Claude) | Complete |
| Phase 6: Post-Implementation Review | Not Started |

## Key Files

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — full implementation spec (finalized)
- [DECISIONS.md](DECISIONS.md) — accepted/rejected review findings
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini refinement review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex critique

## Dependencies

**Upstream:** [0-bootstrap](../0-bootstrap/) — project structure and workflow (complete)

**Downstream:** Future Debian and Ubuntu subscriber images will depend on the shared entrypoint established here.

## Prompt to Resume

```
Read context/SUMMARY.md for project state, then read context/PROCESS.md for the workflow. Resume work on context/specs/1-alpine-subscriber-image/.
```
