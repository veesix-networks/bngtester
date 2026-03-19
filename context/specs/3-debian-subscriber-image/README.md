# 3-debian-subscriber-image

**What:** Debian 12 (Bookworm) subscriber image using isc-dhcp-client for DHCPv4/DHCPv6 and ppp for PPPoE, with minimal entrypoint fixes for dhclient compatibility.

**Source Issue:** [#3](https://github.com/veesix-networks/bngtester/issues/3)

## Status

| Phase | Status |
|-------|--------|
| Phase 0: Issue | Complete |
| Phase 1: Spec Draft (Claude) | Complete |
| Phase 2: Spec Refinement (Gemini) | Complete |
| Phase 3: Spec Critique (Codex) | Complete |
| Phase 4: Spec Finalization (Claude) | Complete |
| Phase 5: Implementation (Claude) | Not Started |
| Phase 6: Post-Implementation Review | Not Started |

## Key Files

- [IMPLEMENTATION_SPEC.md](IMPLEMENTATION_SPEC.md) — full implementation spec (finalized)
- [DECISIONS.md](DECISIONS.md) — accepted/rejected review findings
- [spec-reviews/GEMINI.md](spec-reviews/GEMINI.md) — Gemini refinement review
- [spec-reviews/CODEX.md](spec-reviews/CODEX.md) — Codex critique

## Dependencies

**Upstream:** [1-alpine-subscriber-image](../1-alpine-subscriber-image/) — shared entrypoint (`images/shared/entrypoint.sh`) established here (complete)

**Downstream:** None currently. Future Ubuntu image will follow the same pattern.

## Prompt to Resume

```
Read context/SUMMARY.md for project state, then read context/PROCESS.md for the workflow. Resume work on context/specs/3-debian-subscriber-image/.
```
