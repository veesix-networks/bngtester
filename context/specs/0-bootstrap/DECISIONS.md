# Decisions: bootstrap

Bootstrap review of the AI workflow documentation and project structure. No backing issue — this is the initial project setup.

## Accepted

### Workflow agents dropdown allows contradictory multi-select
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Changed all issue templates from `multiple: true` to single-select dropdown. Each option represents one complete agent configuration.

### Spec path contract is not deterministic
- **Source:** CODEX, GEMINI
- **Severity:** HIGH
- **Resolution:** Standardized to `context/specs/<issue-number>-<slug>/` everywhere (PROCESS.md, CLAUDE.md). Issue number prefix ensures uniqueness, slug derived from issue title.

### Gemini Phase 2 edits cannot be reconciled with Phase 4 decision log
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Changed Phase 2 output model. Gemini now produces `spec-reviews/GEMINI.md` (a review artifact with suggested changes) instead of editing the spec directly. Claude incorporates accepted changes during Phase 4, making all review artifacts traceable and DECISIONS.md entries citable.

### Phase 6 would overwrite the earlier Codex critique
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Split into separate directories: `spec-reviews/` for Phases 2-3 output, `code-reviews/` for Phase 6 output. No path collisions.

### Required human approval states are not represented anywhere durable
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Added Approval State section to PROCESS.md defining: `approved` label on issues as the work gate, DECISIONS.md as the durable record of spec review approvals, spec README.md status table for phase tracking. Full machine-readable contract deferred to when automation is actually built.

### Bug template does not satisfy the workflow's minimum issue contract
- **Source:** CODEX, GEMINI
- **Severity:** HIGH
- **Resolution:** Added optional acceptance criteria, scope boundary, and workflow agents fields to bug.yml. Optional because simple bugs skip the spec workflow, but available when a complex bug needs it.

### Automation-ready claim is ahead of the actual contract
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Reworded to "aspirational automation" with explicit acknowledgment of gaps (no machine-readable status contract, no structured severity parsing, no automated approval gate). Current value is keeping handoff format consistent.

### Generic invocation prompt does not work for Phase 1
- **Source:** CODEX, GEMINI
- **Severity:** MEDIUM
- **Resolution:** Defined two explicit invocation forms in PROCESS.md: issue-driven for Phase 1 (spec directory doesn't exist yet) and spec-path-driven for Phases 2+.

### Status tracker README has no owning phase
- **Source:** CODEX, GEMINI
- **Severity:** MEDIUM
- **Resolution:** Phase 1 explicitly creates the README.md status tracker. Each subsequent phase is required to update it.

### Rejection and rework loops are underspecified
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Added "Rework and Rejection" section to PROCESS.md covering four scenarios: Phase 4 accept/reject, Phase 6 code fixes, spec rejection/rewrite, and out-of-scope discovery.

### Branch naming rules contradict the declared commit taxonomy
- **Source:** CODEX, GEMINI
- **Severity:** MEDIUM
- **Resolution:** Changed PROCESS.md from hardcoded `feat/<scope>-<description>` to `<type>/<scope>-<description>` where type matches the commit type. Added explicit branch naming section to CLAUDE.md with examples.

### Phase 6 trigger ambiguity
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Resolution:** Added explicit rule: Claude asks the human at the end of Phase 5 whether they want Phase 6 and with which agents. Phase 6 prompts are generated at that point, not during Phase 1.

### Prompt handoff ambiguity
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Resolution:** Clarified that Claude generates all Phase 2-3 prompts at the end of Phase 1. Phase 6 prompts are generated separately at the end of Phase 5 if the human opts in.

## Rejected

None. All findings were valid and accepted.
