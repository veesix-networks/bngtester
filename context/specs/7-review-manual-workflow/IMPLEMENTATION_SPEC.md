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

A structured audit of issues #1-#6 against PROCESS.md requirements. The audit covers:

1. **Spec artifact completeness** — Do all specs have README, IMPLEMENTATION_SPEC, DECISIONS, and review artifacts?
2. **Phase 6 execution** — Was post-implementation review performed when requested?
3. **GitHub label accuracy** — Do issue labels reflect the actual workflow state?
4. **Commit discipline** — Were all artifacts committed and pushed?
5. **README status accuracy** — Do README status tables match reality?

#### Audit Findings

##### Finding 1: Uncommitted Review Artifacts

- **Severity:** HIGH
- **Affected:** Issues #2 (4 files), #4 (1 file)
- **Detail:** Gemini and Codex agents wrote spec-review and code-review files locally but never committed them. These files were sitting as untracked files on the working directory.
- **Root cause:** PROCESS.md said "Commit review artifacts to the same branch" but did not explicitly require push. Agents interpreted the instruction loosely.
- **Resolution:** Already fixed. Files committed in `aeae624`. PROCESS.md updated in `9826aec` to add explicit "Commit and push" requirement with example commit messages for Phases 2, 3, and 6.
- **Automation relevance:** HIGH — an automated pipeline would catch this immediately since it would check for committed artifacts before proceeding.

##### Finding 2: Inconsistent Phase 6 Execution

- **Severity:** MEDIUM
- **Affected:** Issues #1, #3, #4
- **Detail:**
  - Issue #1 (Alpine): Phase 6 never started. No code-reviews directory exists.
  - Issue #3 (Debian): Phase 6 partial — Gemini and Codex reviewed, Claude did not.
  - Issue #4 (Ubuntu): Phase 6 never started. code-reviews directory exists but is empty.
  - Issue #2 (Docker Hub CI): Phase 6 complete — all 3 agents reviewed. This is the only fully complete spec.
- **Root cause:** Phase 6 is optional per PROCESS.md. The human must opt in after Phase 5. There's no tracking mechanism to ensure it happens if requested.
- **Automation relevance:** HIGH — n8n could auto-trigger Phase 6 after PR merge if the issue has `agents:all-three` and no Phase 6 artifacts exist.

##### Finding 3: GitHub Label Drift

- **Severity:** MEDIUM
- **Affected:** Issues #1, #3, #4
- **Detail:**
  - Issue #1: Missing `spec:approved` label. Shows `phase:implementation` but is CLOSED.
  - Issue #3: Has `spec:approved` but still shows `phase:implementation` instead of `phase:done`. Issue is CLOSED.
  - Issue #4: Missing `spec:approved` label entirely. Missing all phase completion labels. Issue is CLOSED.
  - Issue #2: Correct — has both `spec:approved` and `phase:done`.
- **Root cause:** Labels are managed manually. PROCESS.md defines the label lifecycle but there's no enforcement. Agents don't consistently update labels, and the human doesn't always catch it.
- **Automation relevance:** CRITICAL — labels are the contract that n8n keys off. If labels are wrong, automation will misfire. This is the #1 thing automation must own.

##### Finding 4: Issues #5 and #6 Stale

- **Severity:** LOW
- **Affected:** Issues #5 (Rust collector, p0), #6 (bare metal testing review, p1)
- **Detail:** Both issues are approved and labeled but have no spec work started. Issue #5 is `priority:p0` (critical path) but has been open with no Phase 1 executed.
- **Root cause:** Manual process — someone needs to invoke Phase 1 for each issue.
- **Automation relevance:** MEDIUM — n8n could detect `approved` issues without a `phase:spec` label and notify or auto-trigger Phase 1.

##### Finding 5: README Status vs Reality Mismatch

- **Severity:** LOW
- **Affected:** Issue #3
- **Detail:** Issue #3's README says Phase 6 is "Not Started" but Gemini and Codex code reviews exist in the directory. The README was never updated after those reviews were committed.
- **Root cause:** PROCESS.md says "README update: Not required" for Phases 2/3 review agents, and Phase 6 doesn't explicitly require non-Claude agents to update the README. When Claude doesn't run Phase 6, nobody updates it.
- **Automation relevance:** LOW — n8n could auto-update README status based on artifact presence.

#### Audit Summary

| Issue | Phases 1-5 | Phase 6 | Labels | Commits | README | Overall |
|-------|-----------|---------|--------|---------|--------|---------|
| #1 Alpine | PASS | FAIL (0/3) | FAIL (missing spec:approved, stale phase) | PASS | PASS | 60% |
| #2 Docker Hub CI | PASS | PASS (3/3) | PASS | FAIL (fixed) | PASS | 80% |
| #3 Debian | PASS | PARTIAL (2/3) | FAIL (stale phase label) | PASS | FAIL (stale status) | 60% |
| #4 Ubuntu | PASS | FAIL (0/3) | FAIL (missing spec:approved, no phase label) | FAIL (fixed) | PASS | 40% |

**Overall consistency rate: ~60%.** The core workflow (Phases 1-5) is consistently followed. The gaps are in Phase 6 execution, label management, and commit discipline — all of which are well-suited for automation.

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

- n8n runs as a Docker container alongside its database (PostgreSQL or SQLite)
- GitHub webhook endpoint exposed via reverse proxy (Caddy/nginx)
- Secrets (GitHub PAT, LLM API keys) stored in n8n's credential manager
- Backup: n8n workflow exports to git (manual or scheduled)

Cloud hosting is not recommended — the project is small, the automation is low-traffic (a few issue events per day), and self-hosting avoids recurring costs and data concerns.

### Deliverable 3: Phase 4 Automation Design

The issue proposes a specific Phase 4 automation pattern. Here's the refined design:

#### Automated Phase 4 Flow

```
[Phase 2/3 agents write review artifacts]
    ↓
[n8n detects new spec-reviews/*.md commits on the branch]
    ↓
[n8n reads artifacts, extracts findings with severity]
    ↓
[n8n posts structured DECISIONS comment on the GitHub issue]
    ↓
[Auto-accept rules apply:]
    ├── CRITICAL/HIGH → accepted immediately
    └── MEDIUM/LOW → 24hr grace period starts
         ↓
    [Grace period: human can reject via]
    ├── Emoji reaction on the comment (e.g., 👎 on specific finding)
    └── `/reject <finding-id> <rationale>` comment
         ↓
[After grace window closes:]
    ↓
[n8n triggers Claude to execute Phase 4]
    ├── Accepted findings → update spec
    ├── Rejected findings → record rationale in DECISIONS.md
    └── Apply `spec:approved` label
```

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
| C1 | CRITICAL | Race condition in concurrent access | ✅ Auto-accept |
| C2 | MEDIUM | Missing edge case for empty input | ⏳ 24hr grace |

---

**Auto-accept policy:**
- CRITICAL/HIGH: accepted immediately
- MEDIUM/LOW: accepted after 24hrs unless rejected

**To reject a finding:** Reply with `/reject <ID> <rationale>` (e.g., `/reject G3 not applicable — internal tool only`)

**Grace period expires:** 2026-03-20 14:30 UTC
```

#### Non-Admin Contributor Handling

The issue raises a concern about non-admin contributors and stale issues. Proposed policy:

1. **Issue approval** remains admin-only (the `approved` label can only be applied by repo admins).
2. **Stale issue policy:** Issues without `approved` label that have no activity for 30 days get a `stale` label and a bot comment. After 7 more days of inactivity, they are closed with a `stale-closed` label.
3. **Approved but inactive:** Issues with `approved` but no Phase 1 work within 14 days get a reminder comment. No auto-close — approved issues represent committed work.
4. **Context decay mitigation:** When an approved issue sits idle for >7 days, n8n re-reads SUMMARY.md and checks for upstream dependency changes before triggering Phase 1. If dependencies have changed, it posts a comment flagging the potential impact.

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
- [ ] Follow-up issue is filed for n8n implementation with clear scope

## Not In Scope

- Implementing n8n workflows (separate issue)
- Fixing the label drift on issues #1-#4 (can be done independently)
- Completing missing Phase 6 reviews for issues #1, #3, #4 (separate effort)
- Phase 1 for issues #5 or #6 (separate workflow invocations)
- Code changes of any kind
