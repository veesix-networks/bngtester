# Implementation Spec: Review Manual Workflow Process

## Overview

Audit the first six issues of bngtester for workflow consistency, identify gaps where agents failed to follow PROCESS.md, and produce actionable decisions on automation tooling (n8n vs alternatives), hosting, and a Phase 4 automation design. This spec does not produce code — it produces findings, decisions, and a follow-up issue for implementation.

## Source Issue

[#7 — Review manual workflow process](https://github.com/veesix-networks/bngtester/issues/7)

## Current State

Five specs have been completed (#0-bootstrap, #1-#4 features). Two approved issues (#5, #6) have no spec work started. The workflow has been executed manually 4 times (issues #1-#4) with varying degrees of consistency, particularly around Phase 6 execution and GitHub label management.

### Known Issues (Pre-Audit)

During a routine check prior to this spec, 5 review artifacts were found uncommitted across issues #2 and #4. These were committed in `aeae624` and a process clarification requiring agents to commit+push was added in `9826aec`. This finding is included in the audit below.

## Design

This spec has three deliverables, each mapping to an acceptance criterion from the issue:

### Deliverable 1: Workflow Consistency Audit

A structured audit of issues #1-#6 measuring two dimensions:

1. **Rules-at-the-time compliance** — Did agents follow the PROCESS.md rules that existed when the work was done?
2. **Current desired state** — Does the issue match today's PROCESS.md requirements (including rules added later)?

This distinction matters because several rules (branch-at-Phase-1, `spec:approved` label, explicit commit+push requirement) were introduced after issues #1 and #4 were already completed. Scoring older work against later rules overstates operator error and conflates backfill debt with process failure.

The audit covers:

1. **Spec artifact completeness** — Do all specs have README, IMPLEMENTATION_SPEC, DECISIONS, and review artifacts?
2. **Phase 6 execution** — Was post-implementation review performed when explicitly requested?
3. **GitHub label accuracy** — Do issue labels reflect the actual workflow state?
4. **Commit discipline** — Were all artifacts committed and pushed?
5. **README status accuracy** — Do README status tables and Key Files sections match reality?

#### Audit Findings

##### Finding 1: Uncommitted Review Artifacts

- **Severity:** HIGH
- **Affected:** Issues #2 (4 files), #4 (1 file)
- **Detail:** Gemini and Codex agents wrote spec-review and code-review files locally but never committed them. These files were sitting as untracked files on the working directory.
- **Root cause:** PROCESS.md said "Commit review artifacts to the same branch" but did not explicitly require push. Agents interpreted the instruction loosely.
- **Resolution:** Already fixed. Files committed in `aeae624`. PROCESS.md updated in `9826aec` to add explicit "Commit and push" requirement with example commit messages for Phases 2, 3, and 6.
- **Automation relevance:** HIGH — an automated pipeline would catch this immediately since it would check for committed artifacts before proceeding.

##### Finding 2: Phase 6 Not Tracked When Opt-In

- **Severity:** LOW
- **Affected:** Issues #1, #3, #4
- **Detail:**
  - Issue #1 (Alpine): Phase 6 never invoked. No code-reviews directory exists.
  - Issue #3 (Debian): Phase 6 partial — Gemini and Codex reviewed, Claude did not.
  - Issue #4 (Ubuntu): Phase 6 never invoked. code-reviews directory exists but is empty.
  - Issue #2 (Docker Hub CI): Phase 6 complete — all 3 agents reviewed. This is the only fully complete spec.
- **Root cause:** Phase 6 is explicitly optional per PROCESS.md — the human must opt in after Phase 5. There is no evidence that Phase 6 was requested for issues #1 or #4, so these are "not invoked" rather than failures. Issue #3 is a genuine partial completion (2/3 agents).
- **Note:** The `agents:all-three` label selects the Phase 1-5 spec pipeline; it does not authorize Phase 6 automatically. Any future automation must key off an explicit Phase 6 trigger (label or command), not infer intent from the agent-selection label.
- **Automation relevance:** MEDIUM — n8n should not auto-trigger Phase 6 based on `agents:all-three`. Instead, it should prompt the human after Phase 5 and only trigger if explicitly confirmed.

##### Finding 3: GitHub Label Drift

- **Severity:** MEDIUM
- **Affected:** Issues #1, #3, #4
- **Detail:**
  - Issue #1: Missing `spec:approved` label. Shows `phase:implementation` but is CLOSED. (Note: `spec:approved` was introduced after #1 completed — this is backfill debt, not a process violation.)
  - Issue #3: Has `spec:approved` but still shows `phase:implementation` instead of `phase:done`. Issue is CLOSED.
  - Issue #4: Missing `spec:approved` label entirely. Missing all phase completion labels. Issue is CLOSED. (Note: `spec:approved` existed when #4 was implemented — this is a genuine miss.)
  - Issue #2: Correct — has both `spec:approved` and `phase:done`.
- **Root cause:** Labels are managed manually. PROCESS.md defines the label lifecycle but there's no enforcement. Agents don't consistently update labels, and the human doesn't always catch it.
- **Automation relevance:** CRITICAL — labels are the contract that n8n keys off. If labels are wrong, automation will misfire. This is the #1 thing automation must own.

##### Finding 4: Issues #5 and #6 Stale

- **Severity:** LOW
- **Affected:** Issues #5 (Rust collector, p0), #6 (bare metal testing review, p1)
- **Detail:** Both issues are approved and labeled but have no spec work started. Issue #5 is `priority:p0` (critical path) but has been open with no Phase 1 executed.
- **Root cause:** Manual process — someone needs to invoke Phase 1 for each issue.
- **Automation relevance:** MEDIUM — n8n could detect `approved` issues without a `phase:spec` label and notify or auto-trigger Phase 1.

##### Finding 5: README Drift (Status and Key Files)

- **Severity:** LOW
- **Affected:** Issues #2, #3
- **Detail:**
  - Issue #3's README says Phase 6 is "Not Started" but Gemini and Codex code reviews exist in the directory. The README was never updated after those reviews were committed.
  - Issue #2's README marks Phase 6 complete but its Key Files section only links spec-reviews — it omits all three code-review artifacts (`code-reviews/CLAUDE.md`, `code-reviews/GEMINI.md`, `code-reviews/CODEX.md`).
  - Issue #3's README has the same Key Files gap — it links spec-reviews but not the Gemini and Codex code reviews that exist in the directory.
- **Root cause:** PROCESS.md says "README update: Not required" for Phases 2/3 review agents, and Phase 6 doesn't explicitly require non-Claude agents to update the README. When Claude doesn't run Phase 6, nobody updates it. Key Files sections are created in Phase 1 and not consistently updated as new artifacts appear.
- **Automation relevance:** LOW — n8n could auto-update README status and Key Files based on artifact presence.

#### Audit Summary

| Issue | Phases 1-5 | Phase 6 | Labels | Commits | README | Notes |
|-------|-----------|---------|--------|---------|--------|-------|
| #1 Alpine | PASS | N/A (not invoked) | BACKFILL (spec:approved added later) | PASS | PASS | Pre-dates current label rules |
| #2 Docker Hub CI | PASS | PASS (3/3) | PASS | FAIL (fixed) | FAIL (missing code-review links) | Reference spec for full lifecycle |
| #3 Debian | PASS | PARTIAL (2/3) | FAIL (stale phase label) | PASS | FAIL (stale status + missing links) | Claude Phase 6 review missing |
| #4 Ubuntu | PASS | N/A (not invoked) | FAIL (missing spec:approved) | FAIL (fixed) | PASS | spec:approved existed but was missed |

**Core workflow (Phases 1-5) is consistently followed at 100%.** The gaps are in optional phases (Phase 6), label management, commit discipline, and README maintenance — all of which are well-suited for automation. Earlier scoring overstated failure by applying rules retroactively to issues that pre-dated them.

### Deliverable 2: Automation Tooling Decision

#### Tool Evaluation

| Criteria | n8n | GitHub Actions | Temporal |
|----------|-----|----------------|----------|
| GitHub integration | Native (webhooks, API) | Native | Requires custom |
| Visual workflow editor | Yes | No (YAML) | No (code) |
| Self-hostable | Yes (Docker) | No (GitHub-hosted) | Yes (complex) |
| LLM agent invocation | HTTP nodes to APIs | Custom actions | Activity workers |
| State management | Built-in | Limited (artifacts) | Built-in |
| Human-in-the-loop | Webhook wait nodes | Issue comments/labels | Signals |
| Cost | Free (self-hosted) | Free tier limits | Free (self-hosted) |
| Learning curve | Low | Low | High |
| Community/ecosystem | Large | Largest | Growing |

#### Recommendation: n8n

n8n is the best fit for this project because:

1. **Visual workflow editor** makes the automation pipeline inspectable by non-developers and easy to modify as the process evolves.
2. **Self-hostable** on BSpendlove's server cluster — no cloud dependency, no cost, full control.
3. **Native GitHub webhooks** — can trigger on issue label changes, PR events, and issue comments without polling.
4. **HTTP Request nodes** can invoke Claude (API), Gemini (API), and Codex (CLI wrapper) directly.
5. **Wait/webhook nodes** support the 24hr grace period pattern described in the issue.
6. **Already mentioned in the issue** — the team has prior familiarity.

#### Hosting Decision

**Self-hosted on BSpendlove's server cluster** using Docker Compose:

- n8n runs as a Docker container alongside its database (PostgreSQL recommended over SQLite for long-lived wait nodes and crash recovery)
- GitHub webhook endpoint exposed via reverse proxy (Caddy/nginx)
- Secrets (GitHub PAT, LLM API keys) stored in n8n's credential manager
- Backup: n8n workflow exports to git (manual or scheduled)

Cloud hosting is not recommended — the project is small, the automation is low-traffic (a few issue events per day), and self-hosting avoids recurring costs and data concerns.

#### Security Model

Self-hosted n8n with repo-mutation capabilities requires explicit security boundaries:

1. **Webhook signature verification:** All GitHub webhook endpoints must validate the `X-Hub-Signature-256` HMAC header. n8n should reject unsigned or invalid payloads.
2. **Replay protection:** Deduplicate webhook deliveries using the `X-GitHub-Delivery` header as an idempotency key.
3. **Least-privilege credentials:** The GitHub PAT used by n8n should have minimal scopes — `issues:write`, `pull_requests:write`, `contents:write` on the bngtester repo only. No org-level or admin scopes.
4. **Command authorization:** `/reject` and `/approve` commands must validate that the commenter has write access to the repo. Non-collaborator comments should be ignored.
5. **Secret rotation:** GitHub PAT and LLM API keys should be rotated on a defined cadence (e.g., 90 days). n8n credential manager supports this.
6. **Audit logging:** All n8n actions that mutate repo state (label changes, comments, spec:approved) should be logged with timestamps and trigger context.

#### Failure Recovery and Idempotency

The automation spans multi-step, long-lived workflows (24hr grace periods). It must handle failures gracefully:

1. **Persisted run key:** Each Phase 4 automation run gets a unique key (e.g., `phase4-<issue>-<commit-sha>`). This key is checked before execution to prevent double-runs from duplicate webhooks.
2. **Deduplicate webhook deliveries:** Use the `X-GitHub-Delivery` header. If n8n has already processed a delivery ID, skip it.
3. **Amended reviews during grace period:** If an agent pushes an updated review commit during the 24hr window, n8n should cancel the current grace timer, re-parse the updated artifact, and restart the grace period with the new findings.
4. **n8n restart recovery:** Use PostgreSQL (not SQLite) so that in-flight wait nodes survive n8n restarts. On startup, n8n resumes any pending grace-period timers.
5. **Partial failure handling:** If Claude updates the spec but the workflow crashes before writing DECISIONS.md or applying `spec:approved`, the run key remains "in-progress." On retry, n8n detects the partial state and resumes from the failed step rather than re-running everything.
6. **Watchdog timers:** When n8n triggers an agent (Phase 2/3/6), it starts a watchdog timer (e.g., 15 minutes). If no artifact commit appears within that window, n8n posts a failure comment on the issue and alerts the admin. This prevents silent hangs from agent crashes or timeouts.

### Deliverable 3: Phase 4 Automation Design

The issue proposes a specific Phase 4 automation pattern. Here's the refined design:

#### Prerequisites

Before the Phase 4 automation can safely parse review artifacts, the project needs a **structured review contract**. Current review artifacts are free-form Markdown with inconsistent formats across agents and issues. n8n cannot reliably extract findings, severities, or IDs from unstructured prose.

**Required contract (to be defined in the follow-up implementation issue):**

A fixed output format for review artifacts — one of:
- YAML front matter with structured findings array
- A fixed Markdown table with required columns (ID, Severity, Finding, Recommendation)
- A JSON sidecar file alongside the Markdown review

Until this contract exists, n8n should use an LLM to parse review artifacts rather than attempting regex/structural parsing. The contract should be implemented before moving to deterministic parsing.

#### Automated Phase 4 Flow

```
[Phase 2/3 agents write review artifacts]
    ↓
[n8n detects new spec-reviews/*.md commits on the branch]
    ↓
[Watchdog: if no commit within 15min of agent trigger, alert admin]
    ↓
[n8n reads artifacts, extracts findings with severity]
    ↓
[n8n posts structured DECISIONS comment on the GitHub issue]
    ↓
[Auto-accept rules apply:]
    ├── CRITICAL → requires human acknowledgement (👀 reaction) before proceeding
    ├── HIGH → accepted immediately
    └── MEDIUM/LOW → 24hr grace period starts
         ↓
    [Grace period: human can act via]
    ├── `/reject <finding-id> <rationale>` — reject a specific finding
    ├── `/approve` or 🚀 reaction — bypass remaining grace period, trigger Phase 4 now
    └── 👎 reaction on a specific finding row — shorthand reject
         ↓
[After grace window closes (or /approve received):]
    ↓
[n8n triggers Claude to execute Phase 4]
    ├── Accepted findings → update spec
    ├── Rejected findings → record rationale in DECISIONS.md
    ├── Contradictory findings (Gemini says X, Codex says not-X) → Claude reconciles,
    │   prioritizing human input if any, otherwise using best judgment. Conflict recorded in DECISIONS.md.
    └── Apply `spec:approved` label
```

**CRITICAL findings require human acknowledgement** rather than auto-accept because a CRITICAL finding like "the entire architecture is flawed" could cause an agent to autonomously restructure the spec in ways that diverge significantly from the human's intent. The 👀 reaction serves as a lightweight "seen and understood" gate without blocking the pipeline for long.

#### Finding ID Convention

Each finding gets a deterministic ID: `<agent-initial><sequence>` (e.g., G1, G2 for Gemini findings; C1, C2 for Codex findings). This allows precise rejection via `/reject C5 <rationale>`.

#### DECISIONS Comment Format (Posted by n8n)

```markdown
## Phase 4 — Review Findings

### From Gemini (spec-reviews/GEMINI.md)

| ID | Severity | Finding | Auto-Action |
|----|----------|---------|-------------|
| G1 | HIGH | Missing health check endpoint | ✅ Auto-accept |
| G2 | MEDIUM | Consider retry logic for API calls | ⏳ 24hr grace |
| G3 | LOW | Variable naming inconsistency | ⏳ 24hr grace |

### From Codex (spec-reviews/CODEX.md)

| ID | Severity | Finding | Auto-Action |
|----|----------|---------|-------------|
| C1 | CRITICAL | Race condition in concurrent access | 🔴 Needs 👀 ack |
| C2 | MEDIUM | Missing edge case for empty input | ⏳ 24hr grace |

---

**Auto-accept policy:**
- 🔴 CRITICAL: requires human acknowledgement (react with 👀 to confirm)
- ✅ HIGH: accepted immediately
- ⏳ MEDIUM/LOW: accepted after 24hrs unless rejected

**Commands:**
- `/reject <ID> <rationale>` — reject a finding (e.g., `/reject G3 not applicable — internal tool only`)
- `/approve` or react 🚀 — bypass remaining grace period, trigger Phase 4 now

**Grace period expires:** 2026-03-20 14:30 UTC
```

#### Non-Admin Contributor Handling

The issue raises a concern about non-admin contributors and stale issues. Proposed policy:

1. **Issue approval** remains admin-only (the `approved` label can only be applied by repo admins).
2. **Stale issue policy:** Issues without `approved` label are managed using explicit states rather than blanket auto-close timers:
   - `blocked` — waiting on a dependency or external factor. Not stale, not closeable.
   - `waiting-on-maintainer` — needs admin review before approval. Reminder after 14 days.
   - `snoozed` — intentionally deferred. No reminders until the snooze expires.
   - Unmarked unapproved issues with no activity for 30 days get a `stale` label and a bot comment asking for status. After 14 more days of inactivity, they are closed with `stale-closed`. This only applies to issues without an explicit state label.
3. **Approved but inactive:** Issues with `approved` but no Phase 1 work within 14 days get a reminder comment. No auto-close — approved issues represent committed work.
4. **Context decay mitigation:** When an approved issue sits idle for >7 days, n8n re-reads SUMMARY.md and checks for upstream dependency changes before triggering Phase 1. If dependencies have changed, it posts a comment flagging the potential impact.

#### Agent Context Delivery

When n8n invokes agents via API (rather than CLI with local repo access), the workflow must include a step to read the current branch state — at minimum the spec file, relevant source files, and SUMMARY.md — and inject that context into the prompt. Agents without repo access cannot review code they cannot see.

## Configuration

No configuration changes in this spec — this is analysis-only. The automation implementation will be a separate issue.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md` | Create | This spec (audit + decisions) |
| `context/specs/7-review-manual-workflow/README.md` | Create | Status tracker |
| `context/specs/7-review-manual-workflow/DECISIONS.md` | Create (Phase 4) | Review finding dispositions |

No code files. No modifications to existing files (the PROCESS.md fix was already applied pre-spec).

## Implementation Order

1. **Phase 1:** This spec (audit findings + automation design)
2. **Phases 2-3:** Gemini and Codex review the audit findings and automation design
3. **Phase 4:** Claude finalizes based on reviews
4. **Phase 5:** No code implementation — deliverable is the audit report, decisions, and a follow-up issue for n8n implementation
5. **Phase 6:** Optional review of the finalized decisions

## Testing

Not applicable — this spec produces analysis and decisions, not code. Validation is:

- [ ] Audit findings are accurate (verifiable against git history and GitHub API)
- [ ] Automation design addresses all acceptance criteria from issue #7
- [ ] Follow-up issue is filed for n8n implementation with clear scope and prerequisites

## Follow-Up Issue Prerequisites

The follow-up n8n implementation issue must include or reference these deliverables to be implementable:

1. **Structured review contract** — define the machine-readable format for review artifacts (YAML front matter, fixed Markdown table, or JSON sidecar). This is a prerequisite for deterministic finding extraction.
2. **Label reconciliation and backfill** — fix label drift on closed issues #1, #3, #4 before automation depends on labels as truth.
3. **Dry-run mode** — n8n workflows must support a dry-run mode that logs intended actions without mutating repo state. Essential for testing and initial deployment.
4. **Observability and alerting** — define what metrics/logs n8n exposes and how admin is alerted on failures (webhook failures, agent timeouts, partial runs).
5. **Database decision** — PostgreSQL over SQLite for long-lived wait nodes and crash recovery (decided in this spec).
6. **Backup and restore** — n8n workflow export strategy and restore procedure.

These should be scoped as either prerequisites in the follow-up issue or split into separate issues if they are independently valuable.

## Not In Scope

- Implementing n8n workflows (separate issue)
- Fixing the label drift on issues #1-#4 (can be done independently)
- Completing missing Phase 6 reviews for issues #1, #3, #4 (separate effort)
- Phase 1 for issues #5 or #6 (separate workflow invocations)
- Code changes of any kind
