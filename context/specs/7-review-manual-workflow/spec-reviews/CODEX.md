# Spec Critique: Review Manual Workflow Process (#7)

The spec identifies real workflow pain points, but several audit conclusions are scored against the wrong baseline and the proposed n8n design skips the contracts that would make automation safe.

## Findings

### HIGH: The audit mixes historical workflow runs with rules that were added later, so the consistency score overstates process failure

- The spec says it audits issues `#1-#6` against current `PROCESS.md` requirements and then scores issues `#1-#4` for missing `spec:approved`, stale phase labels, and commit discipline ([`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:25`](../IMPLEMENTATION_SPEC.md), [`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:56`](../IMPLEMENTATION_SPEC.md), [`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:84`](../IMPLEMENTATION_SPEC.md)).
- That baseline is not historically neutral. The branch-at-Phase-1, priority-label, `spec:approved`, and PR-format rules were added later in commit `29cff47`, and the explicit "commit and push review artifacts" rule landed even later in `9826aec`.
- As written, the audit treats older work as if it violated rules that did not yet exist. That makes findings like missing `spec:approved` on issues `#1` and `#4` read like operator error when they are partly backfill debt from later process changes.
- The audit should separate "violated the rules in force at the time" from "does not match today's desired steady state" and recalculate the summary from there.

### HIGH: Phase 6 is treated as a failure when `PROCESS.md` defines it as opt-in, and the proposed automation contradicts that rule

- `PROCESS.md` says Phases 2, 3, and 6 are opt-in based on available agents and complexity ([`context/PROCESS.md:38`](../../PROCESS.md)). It also says that after Phase 5, Claude must ask the human whether they want Phase 6 and with which agents ([`context/PROCESS.md:140`](../../PROCESS.md)).
- This spec still marks issues `#1` and `#4` as `FAIL (0/3)` for Phase 6 and proposes that n8n auto-trigger Phase 6 after PR merge when an issue has `agents:all-three` and no Phase 6 artifacts exist ([`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:44`](../IMPLEMENTATION_SPEC.md), [`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:84`](../IMPLEMENTATION_SPEC.md)).
- Without evidence that the human actually requested Phase 6 for those issues, "not started" should be `N/A` or "not invoked", not a workflow failure. And `agents:all-three` only selects the Phase 1-5 spec pipeline; it does not authorize post-implementation review automatically.
- The audit should explicitly distinguish "requested but missing" from "never requested", and any future automation should key off an explicit Phase 6 trigger label or command instead of inferring intent from the agent-selection label.

### MEDIUM: The audit misses README drift beyond issue `#3`

- `PROCESS.md` requires each spec README to link the spec, decisions, and reviews in its Key Files section ([`context/PROCESS.md:330`](../../PROCESS.md)).
- Issue `#2` marks Phase 6 complete, but its README only links the spec reviews and omits the three code-review artifacts entirely ([`context/specs/2-ci-publish-dockerhub/README.md:20`](../../2-ci-publish-dockerhub/README.md)).
- Issue `#3` has the same discoverability problem once the Gemini and Codex code reviews were added; the README only links spec reviews ([`context/specs/3-debian-subscriber-image/README.md:19`](../../3-debian-subscriber-image/README.md)).
- Because the audit only flags issue `#3` for stale Phase 6 status, it understates the broader README drift problem around review artifact discoverability.

### HIGH: The Phase 4 automation design depends on machine-readable findings that the current workflow explicitly does not provide

- `PROCESS.md` says the current handoff format is not production-ready for automation and explicitly lacks "machine-readable status contract" and "structured severity parsing" ([`context/PROCESS.md:264`](../../PROCESS.md)).
- Historical review artifacts are not uniform enough for safe parsing. For example, [`context/specs/1-alpine-subscriber-image/spec-reviews/CODEX.md`](../../1-alpine-subscriber-image/spec-reviews/CODEX.md) is organized by severity buckets, [`context/specs/2-ci-publish-dockerhub/spec-reviews/CODEX.md`](../../2-ci-publish-dockerhub/spec-reviews/CODEX.md) embeds severity in heading text, and [`context/specs/4-ubuntu-subscriber-image/spec-reviews/CODEX.md`](../../4-ubuntu-subscriber-image/spec-reviews/CODEX.md) uses a different narrative structure again.
- This spec still assumes n8n can detect review commits, extract findings plus severities, assign deterministic IDs, and then auto-approve the spec when no unresolved `CRITICAL` or `HIGH` findings remain ([`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:137`](../IMPLEMENTATION_SPEC.md)).
- That dependency needs to be called out as prerequisite work. A structured review contract such as YAML front matter, a fixed Markdown table, or a JSON sidecar has to exist before auto-accept or `spec:approved` automation is safe.

### HIGH: The n8n architecture is missing the security and secret-management model for a system that can mutate repo state

- The hosting section exposes a GitHub webhook endpoint and stores GitHub plus LLM credentials in n8n ([`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:122`](../IMPLEMENTATION_SPEC.md)), but it does not define webhook signature verification, replay protection, actor authorization for `/reject` or emoji reactions, least-privilege GitHub credentials, or secret rotation.
- Those are not optional details here. This workflow can trigger agents, post issue comments, and apply `spec:approved`, so a spoofed webhook or over-scoped token can directly change project state.
- The spec should add explicit requirements for GitHub HMAC validation, repo and branch allow-listing, command authorization rules, credential scope boundaries, rotation cadence, and audit logging before it recommends self-hosted n8n as the default architecture.

### HIGH: Failure recovery and idempotency are not designed, so the workflow can double-run or get stuck mid-Phase 4

- The proposed flow is triggered by review-artifact commits and then spans a 24-hour wait window before applying decisions and labels ([`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:137`](../IMPLEMENTATION_SPEC.md)).
- The spec does not say what happens on duplicate webhook deliveries, amended review commits during the grace period, force-pushes, n8n restarts, or partial failure where Claude updates the spec but the workflow crashes before `DECISIONS.md` is written or `spec:approved` is applied.
- "n8n has built-in state management" is not enough as a failure model. Multi-step automation that changes repo state needs a persisted run key, dedupe rules, timeouts, retry rules, and reconciliation logic.
- This is especially important because the current design uses commits as the trigger boundary, which is easy to duplicate or supersede in normal Git workflows.

### MEDIUM: The stale-issue and non-admin-contributor policy is too blunt to match a real maintainer backlog

- The proposed policy auto-closes unapproved issues after 30 days of inactivity plus a 7-day stale window ([`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:199`](../IMPLEMENTATION_SPEC.md)).
- That assumes inactive unapproved issues are noise. In practice, they can be valid backlog, intentionally deferred work, or dependency-blocked items. Closing them because Phase 1 was not triggered turns backlog management into bot churn.
- The approved-issue reminder is also under-specified because the system still does not know who owns the issue, whether it is blocked, or whether the delay is intentional.
- A more realistic policy is to model explicit states such as `blocked`, `waiting-on-maintainer`, or `snoozed`, and reserve auto-close for issues that a human has explicitly marked as disposable.

### MEDIUM: The spec chooses n8n but does not define the prerequisite deliverables that make the follow-up issue implementable

- The Testing section only checks that the audit is accurate, the design addresses the issue, and a follow-up issue gets filed ([`context/specs/7-review-manual-workflow/IMPLEMENTATION_SPEC.md:230`](../IMPLEMENTATION_SPEC.md)).
- Missing from this spec are the concrete deliverables that the follow-up issue will need: the machine-readable review schema, label reconciliation and backfill strategy, dry-run mode, observability and alerting, backup and restore requirements, and a durability decision between SQLite and PostgreSQL for long-lived waits.
- Without those prerequisites, "file a follow-up issue for n8n" is still too vague. The next issue will have to rediscover the same missing contracts before it can implement anything safely.
- This spec should either add a "follow-up issue contents" section or explicitly split the automation work into prerequisite issues versus orchestration work.
