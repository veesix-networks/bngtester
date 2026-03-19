# Codex Bootstrap Review

This review covers `CLAUDE.md`, `context/PROCESS.md`, `README.md`, and the issue templates. I found no `CRITICAL` issues, but there are several `HIGH` severity contract gaps that would make a real end-to-end agent run brittle and would block reliable orchestration later.

## HIGH

### 1. `Workflow agents` can encode impossible phase selections
- **Evidence:** `.github/ISSUE_TEMPLATE/feature.yml:44-55`, `.github/ISSUE_TEMPLATE/enhancement.yml:37-49`, and `.github/ISSUE_TEMPLATE/testing.yml:37-49` all use a dropdown with `multiple: true`, but each option already represents a complete combination such as `Claude only` or `All three (Claude + Gemini + Codex)`.
- **Impact:** A user can select contradictory combinations such as `Claude only` and `All three`. An agent or future pipeline then has no deterministic answer to "which phases should run?"
- **Recommendation:** Make this a single-select field if you want bundled combinations, or replace it with separate booleans for each optional phase or reviewer.

### 2. The spec path contract is not deterministic
- **Evidence:** `CLAUDE.md:17` defines the spec directory as `context/specs/<issue-slug>/`, while `context/PROCESS.md:61`, `context/PROCESS.md:76`, `context/PROCESS.md:111`, and `context/PROCESS.md:169` all use `context/specs/<feature>/`.
- **Impact:** Different agents can derive different folder names for the same issue. That breaks handoffs, review file discovery, resume prompts, and any automation that expects one canonical artifact path.
- **Recommendation:** Pick one identifier and define how to derive it. The safest contract is something like `context/specs/<issue-number>-<slug>/`, with a clear slug normalization rule.

### 3. Gemini's Phase 2 edits cannot be reconciled with the Phase 4 decision log
- **Evidence:** `context/PROCESS.md:68-69` says Phase 2 outputs an updated `IMPLEMENTATION_SPEC.md`. `context/PROCESS.md:83-85` then says Phase 4 consumes human accept/reject decisions, and `context/PROCESS.md:147-165` requires `DECISIONS.md` entries with `Source: CODEX | GEMINI`.
- **Impact:** There is no standalone Gemini review artifact to approve, reject, or cite. Either the human must approve an opaque rewritten spec wholesale, or Claude has to reverse-engineer what Gemini changed before it can populate `DECISIONS.md`.
- **Recommendation:** Make Gemini produce a review artifact first, or require a structured change log alongside the edited spec so Phase 4 has something concrete to adjudicate.

### 4. Phase 6 would overwrite the earlier Codex critique
- **Evidence:** `context/PROCESS.md:76` writes the Phase 3 critique to `context/specs/<feature>/reviews/CODEX.md`. `context/PROCESS.md:104` uses the exact same path for the optional post-implementation Codex review.
- **Impact:** Running Phase 6 destroys the Phase 3 critique record. That breaks traceability and leaves `DECISIONS.md` pointing at a file whose contents no longer match the spec-finalization step.
- **Recommendation:** Use separate filenames for spec critique and implementation review, for example `CODEX_SPEC.md` and `CODEX_IMPL.md`.

### 5. Required human approval states are not represented anywhere durable
- **Evidence:** `context/PROCESS.md:54` says work cannot begin until an issue is `assigned/approved`. `context/PROCESS.md:83-85` makes human accept/reject decisions an input to Phase 4. `README.md:89-91` says humans approve or reject review findings.
- **Impact:** The workflow depends on approvals, but it never defines where those approvals live or how an agent should discover them in a later session. A pipeline cannot know when to start, when to finalize a spec, or whether implementation is blocked.
- **Recommendation:** Define a concrete state contract, such as issue labels for approval, a required approval comment format, or a machine-readable status block in `context/specs/<id>/README.md`.

### 6. The bug template does not satisfy the workflow's own minimum issue contract
- **Evidence:** `context/PROCESS.md:20-25` says each issue must capture `What`, `Why`, `Acceptance criteria`, and `Scope boundary`. `context/PROCESS.md:33-34` and `README.md:41-42` allow complex bugs to enter the fuller workflow. But `.github/ISSUE_TEMPLATE/bug.yml:5-36` only captures what happened, expected behavior, repro steps, and optional context.
- **Impact:** The moment a bug is complex enough to require Phases 1-4, the required inputs are missing. The agent must either ask follow-up questions or invent scope and acceptance criteria, which defeats the "issue is the source of truth" rule.
- **Recommendation:** Add acceptance criteria, not-in-scope, and workflow-agent selection to `bug.yml`, or explicitly state that bug issues never use the spec workflow.

### 7. The "automation-ready" claim is ahead of the actual contract
- **Evidence:** `context/PROCESS.md:115-120` says normalized inputs and outputs will let an automated pipeline invoke agents programmatically and parse results. But `context/PROCESS.md:103-105` defines review outputs as a table, a narrative, and a checklist, and no schema is provided for phase status, blocking severity, skipped phases, or next actions.
- **Impact:** An orchestrator would have to parse free-form markdown to decide whether a run passed, whether a finding blocks implementation, what human decision is still pending, and what phase should run next. That is not a stable automation boundary.
- **Recommendation:** Add a minimal machine-readable contract, such as YAML front matter or sidecar JSON with phase name, status, severity counts, source issue, artifact paths, and next-step hints.

## MEDIUM

### 8. The "single agent invocation" prompt is not actually single
- **Evidence:** `context/PROCESS.md:59-63` defines Phase 1 as issue-driven: `Read issue #N and execute Phase 1.` `context/PROCESS.md:111` then says the instruction to any agent for any phase is `Execute Phase N for context/specs/<feature>/`.
- **Impact:** The generic prompt does not work for Phase 1 because the spec directory does not exist yet. It also does not fit the bug path when the full spec workflow is skipped.
- **Recommendation:** Document two explicit invocation forms: one for issue-driven entry phases and one for spec-path-driven follow-up phases.

### 9. The required status tracker has no owning phase
- **Evidence:** `context/PROCESS.md:61` lists Phase 1 output as only `IMPLEMENTATION_SPEC.md`. `context/PROCESS.md:84` lists Phase 4 output as `IMPLEMENTATION_SPEC.md` plus `DECISIONS.md`. `context/PROCESS.md:169-175` nevertheless says every spec directory `MUST` contain a `README.md` tracker.
- **Impact:** The workflow mandates a tracker file but never assigns responsibility for creating or updating it. In practice, the one file meant to support resumption is likely to be absent or stale.
- **Recommendation:** Make Phase 1 create the tracker and require each later phase to update its status block.

### 10. Rejection and rework loops are underspecified
- **Evidence:** `context/PROCESS.md:83-86` covers spec finalization after critiques, and `context/PROCESS.md:99-105` defines optional post-implementation review outputs, but there is no explicit transition for "spec rejected", "reviewers disagree", or "post-implementation review found a HIGH issue". `context/PROCESS.md:32` and `README.md:98` only cover the out-of-scope case by filing a new issue.
- **Impact:** The happy path is linear, but real reviews are iterative. Agents will have to improvise whether to rerun Phase 1, return to Phase 4, patch code in Phase 5, or stop and wait for a human.
- **Recommendation:** Add explicit loop-back rules, including who owns the revision, which files must be updated, and whether a review rerun overwrites or appends.

### 11. Branch naming rules contradict the declared commit taxonomy
- **Evidence:** `context/PROCESS.md:95` hardcodes `git checkout -b feat/<scope>-<description>`. `CLAUDE.md:72` defines multiple conventional commit types, and `README.md:37-42` maps issue types to `feat`, `fix`, `refactor`, and `test`.
- **Impact:** Bug and testing issues would be forced onto `feat/` branches even when the same docs classify them as `fix` or `test` work. That creates avoidable ambiguity in the branch-to-issue mapping.
- **Recommendation:** Either make branch prefixes follow the same type taxonomy as commits, or explicitly decouple branch prefixes from commit types and say so.
