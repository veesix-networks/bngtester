# bngtester — Project Summary

This file is the project-level state tracker. Every agent session should read this before starting new work. It tracks what has been built, key decisions that affect future work, and how specs relate to each other.

**Updated after every spec is finalized.**

## Current State

Three subscriber images (Alpine, Debian, and Ubuntu) are built and published to Docker Hub via a CI pipeline. The shared entrypoint script supports all access methods and encapsulation types, with auto-detected DHCP client dispatch for both dhcpcd and dhclient. A containerlab topology (`lab/`) deploys osvbng as a VPP-based BNG with a bngtester subscriber and FRR-based server for end-to-end IPoE validation — DHCP lease acquisition, gateway reachability, and cross-BNG connectivity through a real data plane. The AI workflow has been refined with early branching, priority labels, spec approval gates, and a standardized PR format. A workflow consistency audit (#7) found core spec/implementation phases consistently followed, with gaps in label lifecycle, Phase 6 tracking, and commit discipline — n8n has been selected as the automation tool to address these.

## Completed Specs

| Spec | Issue | Status | Summary |
|------|-------|--------|---------|
| [0-bootstrap](specs/0-bootstrap/) | N/A | Complete | AI workflow (PROCESS.md, CLAUDE.md), issue templates, README, contribution rules |
| [1-alpine-subscriber-image](specs/1-alpine-subscriber-image/) | [#1](https://github.com/veesix-networks/bngtester/issues/1) | Complete | Alpine subscriber image + shared entrypoint (VLAN, IPoE, PPPoE) |
| [3-debian-subscriber-image](specs/3-debian-subscriber-image/) | [#3](https://github.com/veesix-networks/bngtester/issues/3) | Complete | Debian 12 subscriber image + dhclient entrypoint fixes |
| [4-ubuntu-subscriber-image](specs/4-ubuntu-subscriber-image/) | [#4](https://github.com/veesix-networks/bngtester/issues/4) | Complete | Ubuntu 22.04 subscriber image (Dockerfile only, no entrypoint changes) |
| [2-ci-publish-dockerhub](specs/2-ci-publish-dockerhub/) | [#2](https://github.com/veesix-networks/bngtester/issues/2) | Complete | CI pipeline to build and publish subscriber images to Docker Hub |
| [7-review-manual-workflow](specs/7-review-manual-workflow/) | [#7](https://github.com/veesix-networks/bngtester/issues/7) | Complete | Workflow consistency audit + n8n automation design |
| [22-mgmt-iface-awareness](specs/22-mgmt-iface-awareness/) | [#22](https://github.com/veesix-networks/bngtester/issues/22) | Complete | Management interface default route removal |
| [27-containerlab-topology](specs/27-containerlab-topology/) | [#27](https://github.com/veesix-networks/bngtester/issues/27) | Complete | Containerlab topology with osvbng BNG, bngtester subscriber, FRR server |
| [13-robot-framework](specs/13-robot-framework/) | [#13](https://github.com/veesix-networks/bngtester/issues/13) | Complete | Robot Framework test runner with standalone + BNG integration tests |

## Spec Dependencies

```mermaid
graph TD
    B[0-bootstrap<br/>AI workflow + project structure]
    A[1-alpine-subscriber-image<br/>Alpine image + shared entrypoint]
    D[3-debian-subscriber-image<br/>Debian image + dhclient fixes]
    U[4-ubuntu-subscriber-image<br/>Ubuntu image]
    CI[2-ci-publish-dockerhub<br/>CI pipeline to Docker Hub]

    WF[7-review-manual-workflow<br/>Workflow audit + n8n design]

    B --> A
    A --> D
    A --> U
    D --> U
    A --> CI
    D --> CI
    U --> CI
    B --> WF
    A --> WF
    D --> WF
    U --> WF
    CI --> WF
    A --> MI[22-mgmt-iface-awareness<br/>MGMT_IFACE route removal]
    A --> CL[27-containerlab-topology<br/>osvbng lab topology]
    MI --> CL
    CL --> RF[13-robot-framework<br/>Robot Framework tests]
    A --> RF

    style B fill:#2da44e,color:#fff
    style A fill:#2da44e,color:#fff
    style D fill:#2da44e,color:#fff
    style U fill:#2da44e,color:#fff
    style CI fill:#2da44e,color:#fff
    style WF fill:#2da44e,color:#fff
    style MI fill:#2da44e,color:#fff
    style CL fill:#2da44e,color:#fff
    style RF fill:#2da44e,color:#fff
```

Legend: green = complete, blue = in progress, grey = planned

## Key Decisions

Decisions that affect future specs. Read these before proposing new work.

### From #8, #9, #10, #11 (workflow improvements)

- **Branch at Phase 1, not Phase 5.** All work for an issue — spec artifacts, reviews, and code — lives on a single feature branch from the start. Review agents check out the branch.
- **Priority labels decouple order from issue number.** `priority:p0` (critical path), `priority:p1` (important), `priority:p2` (nice to have). All issue templates have a priority dropdown.
- **Spec approval gate between Phase 4 and Phase 5.** `spec:approved` label required before implementation. Human contributors open a draft PR for spec review. n8n auto-approves when no unresolved CRITICAL/HIGH findings.
- **PR creation is a required final step of Phase 5.** Conventional Commits title format. Standardized body template with summary, spec link, files, testing. Agent-agnostic attribution.

### From 1-alpine-subscriber-image

- **Shared entrypoint auto-detects DHCP client.** `images/shared/entrypoint.sh` uses `command -v dhcpcd` / `command -v dhclient` at runtime. Future images (Debian, Ubuntu) use the same script — no per-image entrypoints needed.
- **Build context is `images/`, not per-image.** All Dockerfiles use `docker build -f images/<distro>/Dockerfile images/` so they can COPY from `shared/`.
- **bng-client will replace the shell entrypoint.** The planned Rust binary handles VLAN setup, client management, and health reporting. The current entrypoint is the minimum viable approach.
- **Subscriber containers require a dedicated network interface.** Default Docker bridge is not suitable. Use `--network none` + injected veth/macvlan, a dedicated Docker network, or `--network host`.

### From 3-debian-subscriber-image

- **dhclient requires config file for DHCP_TIMEOUT.** dhclient has no CLI flag for timeout — the entrypoint generates `/tmp/dhclient-bngtester.conf` with `timeout N;` and passes it via `-cf`. Future images using dhclient inherit this automatically.
- **Debian images need `ca-certificates` and `netbase`.** `bookworm-slim` lacks CA certs (needed for curl HTTPS) and `/etc/protocols` + `/etc/services` (needed by networking tools). Future Debian-based images should include both.

### From 4-ubuntu-subscriber-image

- **Ubuntu ships `timeout 300;` in stock dhclient.conf.** The entrypoint's `generate_dhclient_conf()` handles this correctly by appending `timeout $DHCP_TIMEOUT;` at the end of the copied config — dhclient uses the last directive. Future dhclient-based images should verify their stock config for conflicting directives.
- **`DEBIAN_FRONTEND=noninteractive` for Ubuntu Dockerfiles.** Ubuntu's apt may trigger interactive prompts during package installation. Use `DEBIAN_FRONTEND=noninteractive` inline in the RUN command.

### From 2-ci-publish-dockerhub

- **Three-job pipeline: discover → build → push.** The push job only runs after all build legs succeed, preventing partial Docker Hub publication. The build job validates Dockerfiles without pushing.
- **Dynamic image discovery.** The workflow finds `images/*/Dockerfile` directories automatically. Adding a new subscriber image requires only its Dockerfile — no workflow edits needed.
- **`latest` strictly follows `main`.** Semver tags publish version tags only. `latest` is never updated by tag events.
- **PR trigger for build-only validation.** Pull requests targeting `main` run the build job without pushing, catching Dockerfile errors before merge.
- **Docker Hub secrets required.** `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` must be configured in repository settings.

### From 7-review-manual-workflow

- **n8n is the automation tool, self-hosted via Docker Compose on BSpendlove's server cluster.** PostgreSQL backend (not SQLite) for crash recovery of long-lived wait nodes.
- **Phase 4 auto-accept: CRITICAL requires human 👀 ack, HIGH auto-accepts, MEDIUM/LOW get 24hr grace period.** `/reject <ID> <rationale>` to reject, `/approve` or 🚀 to fast-track.
- **Review agents must commit and push artifacts.** PROCESS.md updated to require explicit commit+push for Phases 2, 3, and 6. This was the most common failure mode in manual workflow.
- **Labels are the automation contract.** Label drift on closed issues must be backfilled before n8n depends on them. n8n must own label lifecycle going forward.
- **Phase 6 is opt-in, not implied by agents:all-three.** Automation must use an explicit trigger (label or command), not infer intent from agent-selection labels.
- **Structured review contract needed before deterministic parsing.** Current review artifacts are free-form Markdown. n8n should use LLM parsing as interim, then migrate to a fixed format (YAML front matter, Markdown table, or JSON sidecar).
- **Stale issue policy uses explicit states.** `blocked`, `waiting-on-maintainer`, `snoozed` labels prevent premature auto-close. Only unmarked unapproved issues get stale-closed after 30+14 days.
- **Security model for self-hosted n8n.** Webhook HMAC verification, replay protection via `X-GitHub-Delivery`, least-privilege repo-scoped PAT, `/reject`/`/approve` command authorization (write-access only), repo and branch allow-listing, 90-day secret rotation, and audit logging for all repo-mutation actions.
- **Failure recovery and idempotency required.** Persisted run keys to prevent double-runs, webhook deduplication, PostgreSQL for wait-node crash recovery, partial failure handling with resume-from-failed-step, and watchdog timers (15min) for agent execution.
- **API-invoked agents need context injection.** When n8n invokes agents via API (not CLI), it must read and inject branch state (spec, source files, SUMMARY.md) into the prompt.

### From 22-mgmt-iface-awareness

- **`MGMT_IFACE` removes only the default route, not the interface.** The connected route for the management subnet is preserved so orchestrators can still reach the container's management IP. This is critical for containerlab and similar tools where management access is needed for API/metrics.

### From 27-containerlab-topology

- **Lab topology lives in `lab/`, not `tests/`.** The `lab/` directory contains the containerlab topology, osvbng config, FRR server config, smoke test, and README. Robot Framework tests (#13) will reference topologies from `lab/` rather than bundling their own.
- **osvbng test 18 is the reference pattern.** The topology is adapted from osvbng's own `tests/18-ipoe-linux-client/` — same IP scheme, same VLAN scheme (S-VLAN 100, C-VLAN 10), same OSPF design. This keeps the two repos aligned.
- **Server node is FRR, not bngtester-server.** The Rust collector (#5) defines `bngtester-server` but it is not implemented yet. The FRR-based server node provides OSPF routing and iperf3 as an interim far-side endpoint. When the Rust binary is ready, it can replace or augment this node.
- **`dataplane.lcp-netns` is required in osvbng.yaml.** Without `lcp-netns: dataplane`, VPP cannot sync interfaces into the Linux control plane namespace where FRR and DHCP operate. This must be present in any osvbng configuration used with containerlab.
- **Image override via environment variables.** `OSVBNG_IMAGE` and `BNGTESTER_IMAGE` allow swapping images at deploy time (e.g., testing Debian subscriber or local osvbng build). Use `sudo -E` to pass env vars through to containerlab.

### From 13-robot-framework

- **Two test tiers: standalone and integration.** Standalone tests (01-03) use `docker run` and need only Docker. Integration tests (04+) use containerlab + osvbng and are tagged `integration` for exclusion from CI.
- **Log-based verification for standalone VLAN/cleanup tests.** dhcpcd crashes in standalone Docker environments (sysctl restrictions), so VLAN creation is verified via entrypoint log messages rather than `docker exec ip link show`. This is a known limitation.
- **`sudo -E containerlab` preserves image override env vars.** Plain `sudo` strips environment variables. The `CLAB_BIN` variable in common.robot uses `sudo -E` so `OSVBNG_IMAGE` and `BNGTESTER_IMAGE` pass through to containerlab.
- **Image matrix via `--variable SUBSCRIBER_IMAGE:`** — all suites accept this Robot variable. Integration tests also accept `OSVBNG_IMAGE` for the BNG image.
- **Shared keyword interface matches osvbng's common.robot.** Same keyword signatures (Deploy Topology, Destroy Topology, Wait For osvbng Healthy, Execute VPP Command, etc.) enable future interop where osvbng tests can import bngtester keywords.

### From 0-bootstrap

- **Gemini produces review artifacts, not direct spec edits.** All review agents write to `spec-reviews/` — Claude is the only agent that modifies the spec itself (Phase 4).
- **Spec paths use `<issue-number>-<slug>/` convention.** Deterministic, derived from the GitHub issue.
- **One feature per PR, one PR per issue.** No bundling. Out-of-scope discoveries become new issues.
- **`approved` label gates work.** No spec work begins until the issue has the `approved` label.

## Codebase State

| Component | Exists | Notes |
|-----------|--------|-------|
| `images/` | Yes | Alpine + Debian + Ubuntu images, shared entrypoint (`images/shared/entrypoint.sh`, `images/alpine/Dockerfile`, `images/debian/Dockerfile`, `images/ubuntu/Dockerfile`) |
| `lab/` | Yes | Containerlab topology (`bngtester.clab.yml`), osvbng config, FRR server config, smoke test, README |
| `tests/` | Yes | Robot Framework suites: `01-entrypoint-validation`, `02-vlan-modes`, `03-cleanup` (standalone), `04-ipoe-bng` (integration). Shared keywords in `common.robot` + `subscriber.robot`. Runner: `rf-run.sh` |
| `collector/` | No | Go collector not started |
| `.github/workflows/` | Yes | `publish-images.yml` — builds and publishes subscriber images to Docker Hub |
| `context/` | Yes | Workflow docs and bootstrap spec |
| `.github/ISSUE_TEMPLATE/` | Yes | Feature, bug, enhancement, testing templates |
