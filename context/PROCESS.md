# bngtester AI Workflow

This project uses a structured workflow for spec writing, design review, and code review. All participating agents read this file as their entry point.

**This is a lightweight, public workflow.** Anyone can contribute — human or AI — as long as work follows the process below.

## Requirements

All work starts from a **GitHub issue created by a human**. No spec, no branch, no PR happens without a tracked issue. Issues are the single source of truth for what was requested and why.

Issue templates enforce a consistent format for each type of work:

| Template | Use When |
|----------|----------|
| **Feature** | New capability (subscriber image, collector feature, CI pipeline) |
| **Bug** | Something is broken |
| **Enhancement** | Improvement to an existing feature |
| **Testing** | New test scenario or test infrastructure |

Each issue that triggers the spec workflow must capture:
- **What** — the specific deliverable
- **Why** — the motivation or problem being solved
- **Acceptance criteria** — how to know it's done
- **Scope boundary** — what is explicitly NOT part of this issue
- **Workflow agents** — which agents review the spec (single selection)

### Scope Rules

**One feature per PR. One PR per issue. No exceptions.**

- A PR implements exactly one issue. If an issue requires multiple features, split it into multiple issues first.
- Do not bundle unrelated changes, "while I'm here" improvements, or opportunistic refactors into a feature PR.
- If implementation reveals a needed change outside the issue's scope, file a new issue for it.
- Bug fixes and typo corrections can be small and self-contained — they still need an issue but can skip the full spec workflow.

These rules exist to prevent scope creep, keep reviews focused, and make the git history useful. They also ensure that when we automate the pipeline, each agent run maps cleanly to one issue and one deliverable.

## Agent Participation

The workflow supports flexible agent participation. Any combination works:

- **Claude only** — Claude drafts, reviews, and implements. Phases 2 and 3 are skipped.
- **Claude + Gemini** — Gemini reviews the spec (Phase 2). Codex critique (Phase 3) is skipped.
- **Claude + Codex** — Codex critiques the spec (Phase 3). Gemini review (Phase 2) is skipped.
- **All three** — Full pipeline: Claude drafts, Gemini reviews, Codex critiques, Claude finalizes and implements.

The only requirement is that Claude handles Phase 1 (draft) and Phase 5 (implementation), since it has direct codebase access. Phases 2, 3, and 6 are opt-in based on available agents and the complexity of the work.

## Project Summary

`context/SUMMARY.md` is the project-level state tracker. It records what has been built, key decisions that affect future work, spec dependencies (as a Mermaid graph), and the current codebase state.

**Every agent session should read `context/SUMMARY.md` before starting new work.** It is updated at the end of Phase 5 (implementation) when the spec's deliverables are merged.

## Spec Directory Convention

Every issue that goes through the spec workflow gets a directory at:

```
context/specs/<issue-number>-<slug>/
```

Where `<issue-number>` is the GitHub issue number and `<slug>` is a short lowercase-hyphenated description derived from the issue title. Examples:

- `context/specs/1-alpine-subscriber-image/`
- `context/specs/5-ci-publish-ghcr/`
- `context/specs/12-collector-junit-output/`

This convention is deterministic — any agent can derive the path from the issue number and title. The issue number prefix ensures uniqueness and makes cross-referencing trivial.

## Workflow Phases

### Phase 0: Issue (Human)

- **Actor:** Human
- **Output:** GitHub issue using the appropriate template
- **Gate:** No work begins until the issue exists. Add the `approved` label to signal the issue is ready for the workflow.
- The issue IS the requirements document. Templates enforce structure (what, why, acceptance criteria, scope boundary). No separate requirements gathering step.

### Phase 1: Spec Draft (Claude)

- **Invocation:** Human gives Claude the issue reference:
  > Read context/PROCESS.md and execute Phase 1 for issue #N.
- **Input:** Claude reads the issue via `gh issue view <number>` + reads `context/SUMMARY.md` for project state + reads the existing codebase
- **Branch:** Create a feature branch from `main` before any file edits: `git checkout main && git pull && git checkout -b <type>/<scope>-<description>`. The branch prefix matches the commit type (e.g., `feat/`, `fix/`, `test/`, `docs/`). Never edit files on `main` directly. All work for this issue — spec artifacts, review files, and code — lives on this branch.
- **Output:**
  - `context/specs/<issue-number>-<slug>/IMPLEMENTATION_SPEC.md`
  - `context/specs/<issue-number>-<slug>/README.md` (status tracker — see format below)
- **The spec MUST reference the source issue number.**
- Claude derives everything from the issue — do not ask the human to re-explain what is already in the issue.
- Claude MUST generate ready-to-paste prompts for all subsequent agents at the end of Phase 1 (see Agent Invocation Prompts below). These prompts MUST include the branch name so review agents check out the correct branch.
- **README update:** Create the README with Phase 1 marked as Complete, all other phases as Not Started or Skipped. Include upstream/downstream dependencies.
- **Push:** Push the branch after committing spec artifacts so review agents and humans can access them.
- **Why Claude:** Direct codebase access means the spec is grounded in real code — real file paths, existing patterns, concrete file plan.

### Phase 2: Spec Refinement (Gemini) — optional

- **Invocation:** Human pastes the Gemini prompt generated at the end of Phase 1.
  > Read context/PROCESS.md for the workflow. Check out branch `<type>/<scope>-<description>`. Review the spec at context/specs/<issue-number>-<slug>/IMPLEMENTATION_SPEC.md. Write findings to context/specs/<issue-number>-<slug>/spec-reviews/GEMINI.md.
- **Branch:** Check out the branch created in Phase 1. Commit review artifacts to the same branch.
- **Input:** `IMPLEMENTATION_SPEC.md`
- **Output:** `context/specs/<issue-number>-<slug>/spec-reviews/GEMINI.md` — a review artifact with suggested changes, corrections, and missing requirements. **Gemini does NOT edit the spec directly.**
- **Why Gemini:** Large context window excels at cross-referencing the full spec against best practices and catching inconsistencies.
- **README update:** Not required (Gemini does not own the README). Claude updates it in Phase 4.
- **Skip when:** Working with Claude only, or the spec is straightforward enough that refinement adds no value.

### Phase 3: Spec Critique (Codex) — optional

- **Invocation:** Human pastes the Codex prompt generated at the end of Phase 1.
  > Read context/PROCESS.md for the workflow. Check out branch `<type>/<scope>-<description>`. Critique the spec at context/specs/<issue-number>-<slug>/IMPLEMENTATION_SPEC.md. Write findings to context/specs/<issue-number>-<slug>/spec-reviews/CODEX.md.
- **Branch:** Check out the branch created in Phase 1. Commit review artifacts to the same branch.
- **Input:** `IMPLEMENTATION_SPEC.md` (same version as Phase 2 — both review the Phase 1 draft) + codebase
- **Output:** `context/specs/<issue-number>-<slug>/spec-reviews/CODEX.md`
- **Focus:** Architectural gaps, missing edge cases, failure modes, scope issues
- **Why Codex:** Best at finding what's NOT there — missing failure paths, dead config, unimplemented spec features.
- **README update:** Not required (Codex does not own the README). Claude updates it in Phase 4.
- **Skip when:** Working with Claude only, or the feature is small enough that a critique pass is overkill.

### Phase 4: Spec Finalization (Claude)

- **Input:** `spec-reviews/GEMINI.md` (if any) + `spec-reviews/CODEX.md` (if any) + human accept/reject decisions
- **Output:** Final `IMPLEMENTATION_SPEC.md` + `DECISIONS.md` + updated `README.md` status
- **DECISIONS.md** records rationale for every accepted and rejected finding from review artifacts, citing the source agent and severity.
- **README update:** Mark Phases 2-4 status. Update Key Files with links to DECISIONS.md and review artifacts.
- **Skip when:** Phases 2 and 3 were both skipped — the Phase 1 draft is the final spec.

### Phase 5: Implementation (Claude)

- **Output:** Code committed to the repo
- **PR references the source issue** (e.g., `Closes #N`).
- After completing implementation, Claude MUST ask the human whether they want Phase 6 (post-implementation review) and with which agents.
- **README update:** Mark Phase 5 as Complete. Add branch/PR info.
- **SUMMARY.md update:** Update `context/SUMMARY.md` with the new spec in Completed Specs, add to the Mermaid dependency graph, record any key decisions that affect future work, and update the Codebase State table.

#### Implementation Rules

1. **Use the existing branch.** The feature branch was created in Phase 1. Check it out if not already on it. Do not create a new branch.
2. **One commit per logical unit.** Each implementation sub-phase gets its own commit as the work is done.
3. **Commit message provided immediately.** After completing each unit, provide the conventional commit message, file list, and any context.

### Phase 6: Post-Implementation Review — optional

- **Gate:** Human decides whether to run Phase 6 and with which agents. Claude asks at the end of Phase 5.

Any combination of agents can review the completed code:

- **Claude — Bug Hunter:** Line-level bugs, race conditions, resource leaks, security, error handling. Severity-rated table format. Output: `context/specs/<issue-number>-<slug>/code-reviews/CLAUDE.md`
- **Codex — Spec Compliance:** Did we build what the spec says? What's missing? What drifted? Narrative format. Output: `context/specs/<issue-number>-<slug>/code-reviews/CODEX.md`
- **Gemini — Best Practices:** Dockerfile best practices, CI correctness, Go idioms, security review. Checklist format. Output: `context/specs/<issue-number>-<slug>/code-reviews/GEMINI.md`

## Amendments

A human can intervene at any point in the workflow to change requirements. This is expected — requirements evolve as you learn.

### Before a spec exists (Phase 0 or early Phase 1)

Edit the issue body directly. The issue is the source of truth and no spec exists yet, so there's nothing to reconcile.

### After a spec exists (Phases 1-4)

1. **Human comments on the issue** describing the amendment — what changed and why. This comment is the audit trail.
2. **The current phase pauses.**
3. **Claude reads the amendment comment**, updates the spec inline, and adds an entry to `DECISIONS.md`:

```markdown
### <amendment title>
- **Source:** AMENDMENT (human intervention)
- **Phase:** <phase when amendment occurred>
- **Resolution:** <what was changed in the spec>
```

4. **If Phases 2-3 already produced reviews:** The amended sections get a targeted re-review. Agents review only the changed sections, not the full spec. Previous review artifacts are preserved — new findings are appended.
5. **If only Phase 1 completed:** No re-review needed unless the human requests it.
6. **Workflow resumes** from where it paused.

### During implementation (Phase 5)

1. **Human comments on the issue or PR** describing the change.
2. **Claude updates the spec**, adds an `AMENDMENT` entry to `DECISIONS.md`, and adjusts the implementation.
3. **No re-review unless the human requests it.** The amendment is captured in the PR diff and reviewed at merge time.

### Rules

- Amendments must stay within the original issue's scope boundary. If the amendment is actually a new feature, file a new issue.
- The amendment comment on the issue is the audit trail. Do not amend via conversation alone.
- Git history preserves previous spec versions. Overwriting is fine.

## Rework and Rejection

Reviews are iterative. When a reviewer flags issues:

1. **During Phase 4 (spec finalization):** The human tells Claude which findings to accept or reject. Claude updates the spec and records all decisions in `DECISIONS.md`. If a rejected finding is contested, the human has final say.

2. **During Phase 6 (code review):** If reviewers find HIGH or CRITICAL issues, Claude fixes the code on the same branch and commits. The fix commit references the review finding. No new issue or branch is needed for fixes that are within the original issue's scope.

3. **Spec rejected entirely:** If the human rejects the Phase 1 draft, Claude rewrites it in the same directory. The previous draft is overwritten — git history preserves it.

4. **Out-of-scope discovery:** If any phase reveals work that is outside the issue's scope boundary, file a new issue. Do not expand the current spec or PR.

## How to Invoke an Agent

There are two invocation forms depending on the phase:

**Phase 1 (issue-driven — spec directory doesn't exist yet):**

> Read `context/PROCESS.md` in the bngtester repo for the workflow. Execute Phase 1 for issue #N.

**Phases 2+ (spec-path-driven — spec directory exists):**

> Read `context/PROCESS.md` in the bngtester repo for the workflow. Execute Phase N for `context/specs/<issue-number>-<slug>/`.

### Why Normalized Inputs and Outputs

The structured prompts and fixed output paths serve two purposes:

1. **Human-driven today** — ready-to-paste prompts so a human can copy directly into each agent's terminal without re-explaining context.
2. **Aspirational automation** — normalized inputs (spec path, codebase path, focus areas) and outputs (fixed file paths, severity scale, format templates) are designed so that a future pipeline could invoke agents programmatically. This is not production-ready today — there is no machine-readable status contract, no structured severity parsing, and no automated approval gate. Those would need to be added when automation is actually built. The current value is keeping the handoff format consistent so the gap to automation stays small.

### Agent Invocation Prompts

After completing Phase 1 (spec draft), Claude MUST provide ready-to-paste prompts for **all** optional agents the human selected in the issue, covering both spec review (Phases 2-3) and noting that Phase 6 will be offered after implementation. This means:

- If the issue selected "All three": generate prompts for Gemini (Phase 2) and Codex (Phase 3) at the end of Phase 1.
- If "Claude + Gemini": generate only the Gemini prompt.
- If "Claude + Codex": generate only the Codex prompt.
- Phase 6 prompts are generated later, at the end of Phase 5, if the human opts in.

Each prompt must include:

1. The exact path to the spec being reviewed
2. A brief summary of what the spec covers
3. Key areas to focus on specific to that agent's mandate
4. The codebase path

## Spec Format

### IMPLEMENTATION_SPEC.md

Must contain these sections:

1. **Overview** — what and why, 2-3 sentences max
2. **Source Issue** — link to the GitHub issue that triggered this spec
3. **Current State** — what exists today
4. **Design** — architecture, data flow, key decisions
5. **Configuration** — environment variables, config files, with examples
6. **File Plan** — every file to create or modify, with purpose
7. **Implementation Order** — numbered phases, each independently testable
8. **Testing** — what to test, how to test it
9. **Not In Scope** — what this spec explicitly does not cover

### DECISIONS.md

```markdown
# Decisions: <issue-number>-<slug>

## Accepted

### <finding title>
- **Source:** CODEX | GEMINI
- **Severity:** CRITICAL | HIGH | MEDIUM | LOW
- **Resolution:** <what was changed in the spec>

## Rejected

### <finding title>
- **Source:** CODEX | GEMINI
- **Severity:** CRITICAL | HIGH | MEDIUM | LOW
- **Rationale:** <why this was rejected>
```

### README.md — Status Tracker

Every `context/specs/<issue-number>-<slug>/` directory MUST contain a `README.md`. **Phase 1 creates it. Every subsequent phase updates it** (see per-phase README update rules above).

Required content:

- **What** — one-line description
- **Source Issue** — link to the GitHub issue (e.g., `#N`)
- **Status** — table showing each phase's status (Complete, In Progress, Skipped, Not Started)
- **Key Files** — links to the spec, decisions, reviews
- **Dependencies** — upstream specs this depends on + downstream specs that depend on this
- **Prompt to Resume** — ready-to-paste prompt for continuing in a new session (must reference `context/SUMMARY.md` first)

## Labels

Labels track workflow state and agent configuration. These are the contract that n8n (or any future automation) keys off.

### Workflow State Labels

| Label | Applied When | Removed When |
|-------|-------------|--------------|
| `approved` | Human approves the issue for work | Never (stays for audit) |
| `phase:spec` | Phase 1 starts | Phase 4 completes (or Phase 1 if no review) |
| `phase:review` | Phase 2 or 3 starts | Phase 4 completes |
| `phase:implementation` | Phase 5 starts | Phase 5 completes |
| `phase:done` | All phases complete + PR merged | Never |

### Agent Selection Labels

| Label | Meaning |
|-------|---------|
| `agents:claude-only` | Phases 2, 3 skipped |
| `agents:claude-gemini` | Phase 3 skipped |
| `agents:claude-codex` | Phase 2 skipped |
| `agents:all-three` | Full pipeline |

The agent label is applied based on the "Workflow agents" dropdown selection in the issue template. Today this is manual — the human adds the label after creating the issue. When n8n is implemented, it will read the dropdown value and apply the label automatically.

### Issue Type Labels

Auto-applied by issue templates: `feature`, `bug`, `enhancement`, `testing`.

## Approval State

- **Issue approval:** The `approved` label on the GitHub issue signals that work can begin. No label = no work.
- **Spec review approval:** The human communicates accept/reject decisions to Claude in conversation during Phase 4. These are recorded durably in `DECISIONS.md`.
- **Implementation approval:** Standard PR review and merge process.
- **Phase status:** Tracked in the spec `README.md` status table and the issue's phase labels. Both must stay in sync.

## Severity Scale

All agents use these definitions:

| Severity | Definition |
|----------|------------|
| **CRITICAL** | Will cause failures, security vulnerabilities, or broken builds in production |
| **HIGH** | Significant bugs that affect correctness but have workarounds, or missing functionality that the spec promised |
| **MEDIUM** | Edge cases, performance issues, non-ideal error handling |
| **LOW** | Cosmetic issues, future-proofing concerns |

## Anti-Patterns (All Agents)

1. **Capability summaries** — "The implementation supports X, Y, and Z" is not a review finding. State what is broken, missing, or wrong.
2. **Style reviews** — Do not comment on naming, formatting, or code style. These are not bugs.
3. **Speculative issues** — "This could potentially cause problems if..." without a concrete path is noise.
4. **Generic recommendations** — "Consider adding more tests" is not a finding.
