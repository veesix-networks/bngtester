# 0-bootstrap: AI Workflow and Project Structure

## What

Initial project setup — AI workflow definition (PROCESS.md, CLAUDE.md), issue templates, README, and contribution rules.

## Source Issue

None (bootstrap — no backing issue).

## Status

| Phase | Status |
|-------|--------|
| Phase 0: Issue | N/A (bootstrap) |
| Phase 1: Spec Draft | N/A (bootstrap — no implementation spec) |
| Phase 2: Spec Refinement (Gemini) | **Complete** |
| Phase 3: Spec Critique (Codex) | **Complete** |
| Phase 4: Spec Finalization | **Complete** — all findings accepted, see DECISIONS.md |
| Phase 5: Implementation | N/A (docs-only) |
| Phase 6: Post-Implementation Review | Skipped |

## Key Files

- [DECISIONS.md](DECISIONS.md) — accept/reject rationale for all review findings
- [reviews/GEMINI.md](reviews/GEMINI.md) — Gemini refinement review
- [reviews/CODEX.md](reviews/CODEX.md) — Codex critique

## Dependencies

- **Upstream:** None (this is the root)
- **Downstream:** All future specs depend on the workflow defined here

## Prompt to Resume

> Read `context/SUMMARY.md` for project state, then read `context/specs/0-bootstrap/README.md`. The bootstrap is complete — workflow docs, issue templates, and project structure are finalized. All Gemini and Codex findings accepted and resolved. Next step: file the first real issue and run the workflow end-to-end.
