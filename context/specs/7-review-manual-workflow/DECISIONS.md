# Decisions: 7-review-manual-workflow

## Accepted

### Agent execution watchdog timers
- **Source:** GEMINI (G1)
- **Severity:** HIGH
- **Resolution:** Added watchdog timer requirement to the automation flow — if no artifact commit appears within 15 minutes of agent trigger, n8n posts a failure comment and alerts admin.

### Fast-track approval via /approve command
- **Source:** GEMINI (G2)
- **Severity:** MEDIUM
- **Resolution:** Added `/approve` command and 🚀 reaction as mechanisms to bypass the remaining grace period and trigger Phase 4 immediately.

### Context delivery for API-invoked agents
- **Source:** GEMINI (G4)
- **Severity:** LOW
- **Resolution:** Added "Agent Context Delivery" section requiring n8n to read and inject branch state (spec, source files, SUMMARY.md) when invoking agents via API rather than CLI.

### CRITICAL findings require human acknowledgement
- **Source:** GEMINI (G5)
- **Severity:** HIGH (inline finding)
- **Resolution:** Changed CRITICAL auto-accept to require 👀 reaction from human before Phase 4 proceeds. HIGH remains auto-accept. This prevents agents from autonomously restructuring specs on CRITICAL architectural findings without human awareness.

### Conflict resolution for contradictory findings
- **Source:** GEMINI (G6)
- **Severity:** MEDIUM (inline finding)
- **Resolution:** Added explicit conflict resolution step in the Phase 4 flow — Claude reconciles contradictory findings, prioritizes human input, and records the conflict in DECISIONS.md.

### Audit baseline reframed for historical accuracy
- **Source:** CODEX (C1)
- **Severity:** HIGH
- **Resolution:** Reframed the audit to distinguish "violated rules in force at the time" from "does not match today's desired state." Issue #1's missing spec:approved is now noted as backfill debt (rule didn't exist yet), while #4's is a genuine miss (rule existed). Removed percentage-based "overall consistency rate" in favor of nuanced per-issue notes. Core workflow (Phases 1-5) is 100% compliant.

### Phase 6 reframed as opt-in, not failure
- **Source:** CODEX (C2)
- **Severity:** HIGH
- **Resolution:** Changed Phase 6 column from "FAIL (0/3)" to "N/A (not invoked)" for issues #1 and #4. Downgraded finding severity from HIGH to LOW. Added note that agents:all-three does not authorize Phase 6 — future automation must use an explicit trigger label or command. Only issue #3 is a genuine partial completion (2/3 agents).

### README drift broader than originally reported
- **Source:** CODEX (C3)
- **Severity:** MEDIUM
- **Resolution:** Expanded Finding 5 to include issues #2 and #3 (both missing code-review links in Key Files sections), not just #3's stale Phase 6 status.

### Structured review contract required before automation
- **Source:** CODEX (C4)
- **Severity:** HIGH
- **Resolution:** Added "Prerequisites" section before the Phase 4 automation flow. The project needs a machine-readable review format (YAML front matter, fixed Markdown table, or JSON sidecar) before n8n can deterministically parse findings. Interim approach: use LLM parsing until the contract is defined. Added to follow-up issue prerequisites.

### Security model for self-hosted n8n
- **Source:** CODEX (C5)
- **Severity:** HIGH
- **Resolution:** Added full "Security Model" section covering webhook HMAC validation, replay protection, least-privilege GitHub credentials, /reject command authorization, secret rotation cadence, and audit logging.

### Failure recovery and idempotency design
- **Source:** CODEX (C6)
- **Severity:** HIGH
- **Resolution:** Added "Failure Recovery and Idempotency" section covering persisted run keys, webhook deduplication, amended review handling, n8n restart recovery (PostgreSQL required), partial failure handling, and watchdog timers.

### Stale issue policy refined with explicit states
- **Source:** CODEX (C7)
- **Severity:** MEDIUM
- **Resolution:** Replaced blanket 30+7 day auto-close with explicit state labels (blocked, waiting-on-maintainer, snoozed). Auto-close only applies to unmarked unapproved issues after 30+14 days. Approved issues are never auto-closed.

### Follow-up issue prerequisites defined
- **Source:** CODEX (C8)
- **Severity:** MEDIUM
- **Resolution:** Added "Follow-Up Issue Prerequisites" section listing 6 concrete deliverables the n8n implementation issue must include: structured review contract, label backfill, dry-run mode, observability, database decision, and backup/restore.

## Rejected

### Content-based hashes for finding IDs
- **Source:** GEMINI (G3)
- **Severity:** MEDIUM
- **Rationale:** Over-engineering for the current stage. Sequential IDs (G1, G2, C1, C2) are simple and human-readable. The re-run instability concern is real but unlikely in practice — review artifacts are typically written once. If IDs need to be stable across re-runs, a simple slug-based scheme (e.g., `G-missing-health-check`) would suffice without requiring content hashing. Can revisit if re-run frequency becomes a problem.
