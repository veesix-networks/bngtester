# Gemini Spec Review: Review Manual Workflow Process (#7)

## Overview
This review evaluates the audit of the manual workflow and the proposed n8n automation design, focusing on robustness, edge cases, and the soundness of the auto-accept pattern.

## Audit Findings Accuracy
The audit findings are **HIGHLY ACCURATE**. I have verified the label drift, missing phase 6 artifacts, and stale status tables against the live GitHub issues and the current repository state.

- **Verified:** Issue #4 is indeed missing all phase labels and spec approval despite being closed.
- **Verified:** Issue #2 is the only issue following the full lifecycle correctly (Phases 1-6).
- **Verified:** Uncommitted artifacts were a significant bottleneck now addressed by the `PROCESS.md` update.

## Automation Design Gaps

### 1. Agent Execution Failure & Timeout
The design triggers on commits, but there is no mechanism for detecting **non-events**.
- **Scenario:** An agent is invoked (Phase 2) but crashes or times out before committing.
- **Gap:** n8n will wait indefinitely for a commit that never comes.
- **Recommendation:** Implement a watchdog timer in n8n. If an agent is triggered and no artifact is committed within N minutes (e.g., 10m), n8n should post a failure comment on the issue and alert the admin.

### 2. Codebase Context for API-Invoked Agents
The spec suggests n8n can invoke Claude/Gemini APIs directly.
- **Gap:** API agents (especially Gemini/Claude via API vs CLI) do not have "local" access to the repository unless it is provided in the prompt.
- **Recommendation:** The n8n workflow must include a step to read the current state of the branch (at least the spec and relevant file listings) and inject it into the prompt, or use a "Generalist" sub-agent pattern that has tool access to the repo.

### 3. Fast-Track Approval
- **Gap:** The 24hr grace period is excellent for passive safety, but may frustrate active development.
- **Recommendation:** Add a `/approve` or `🚀` reaction to the DECISIONS comment to bypass the remaining grace period and trigger Phase 4 immediately.

### 4. Deterministic ID Stability
- **Gap:** If an agent is re-invoked (e.g., due to an amendment), it might generate different findings or the same findings in a different order, breaking deterministic IDs like `G1`, `G2`.
- **Recommendation:** Instruct agents to generate IDs based on a hash of the finding's title or a persistent slug, rather than simple sequence numbers, to ensure `/reject` commands remain valid across re-runs.

## Phase 4 Auto-Accept Pattern

### 1. The "CRITICAL = Auto-Accept" Paradox
- **Finding:** The design auto-accepts CRITICAL/HIGH findings.
- **Risk:** If a finding is "CRITICAL: The entire architecture is flawed," auto-accepting it means the agent will attempt to "fix" the architecture autonomously in Phase 4. While this aligns with the goal of autonomy, it might lead to significant drift from the human's original intent without a "Stop" button.
- **Recommendation:** CRITICAL findings should perhaps **block** the grace period and require explicit human acknowledgement (e.g., a "seen" reaction) before Phase 4 can proceed, even if the "Resolution" is auto-applied to the spec.

### 2. Conflict Resolution
- **Gap:** Gemini might suggest X, and Codex might suggest "Not X" for the same line.
- **Recommendation:** Phase 4 (Claude) is already tasked with reconciling findings. The n8n design should explicitly state that if findings are contradictory, Claude must prioritize human input (if any) and then use its best judgment, recording the conflict in `DECISIONS.md`.

## Summary of Recommendations
| Severity | Finding | Recommendation |
|----------|---------|----------------|
| **HIGH** | Lack of timeout/failure detection | Add watchdog timers to n8n workflows. |
| **MEDIUM** | Grace period friction | Add `/approve` command to bypass wait. |
| **MEDIUM** | ID instability | Use content-based hashes or persistent slugs for finding IDs. |
| **LOW** | Context delivery | Ensure n8n provides spec/file context to API-based agents. |
