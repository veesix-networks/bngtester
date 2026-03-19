# bngtester — Project Summary

This file is the project-level state tracker. Every agent session should read this before starting new work. It tracks what has been built, key decisions that affect future work, and how specs relate to each other.

**Updated after every spec is finalized.**

## Current State

Three subscriber images (Alpine, Debian, and Ubuntu) are built. The shared entrypoint script supports all access methods and encapsulation types, with auto-detected DHCP client dispatch for both dhcpcd and dhclient. The AI workflow has been refined with early branching, priority labels, spec approval gates, and a standardized PR format.

## Completed Specs

| Spec | Issue | Status | Summary |
|------|-------|--------|---------|
| [0-bootstrap](specs/0-bootstrap/) | N/A | Complete | AI workflow (PROCESS.md, CLAUDE.md), issue templates, README, contribution rules |
| [1-alpine-subscriber-image](specs/1-alpine-subscriber-image/) | [#1](https://github.com/veesix-networks/bngtester/issues/1) | Complete | Alpine subscriber image + shared entrypoint (VLAN, IPoE, PPPoE) |
| [3-debian-subscriber-image](specs/3-debian-subscriber-image/) | [#3](https://github.com/veesix-networks/bngtester/issues/3) | Complete | Debian 12 subscriber image + dhclient entrypoint fixes |
| [4-ubuntu-subscriber-image](specs/4-ubuntu-subscriber-image/) | [#4](https://github.com/veesix-networks/bngtester/issues/4) | Complete | Ubuntu 22.04 subscriber image (Dockerfile only, no entrypoint changes) |

## Spec Dependencies

```mermaid
graph TD
    B[0-bootstrap<br/>AI workflow + project structure]
    A[1-alpine-subscriber-image<br/>Alpine image + shared entrypoint]
    D[3-debian-subscriber-image<br/>Debian image + dhclient fixes]
    U[4-ubuntu-subscriber-image<br/>Ubuntu image]

    B --> A
    A --> D
    A --> U
    D --> U

    style B fill:#2da44e,color:#fff
    style A fill:#2da44e,color:#fff
    style D fill:#2da44e,color:#fff
    style U fill:#2da44e,color:#fff
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

### From 0-bootstrap

- **Gemini produces review artifacts, not direct spec edits.** All review agents write to `spec-reviews/` — Claude is the only agent that modifies the spec itself (Phase 4).
- **Spec paths use `<issue-number>-<slug>/` convention.** Deterministic, derived from the GitHub issue.
- **One feature per PR, one PR per issue.** No bundling. Out-of-scope discoveries become new issues.
- **`approved` label gates work.** No spec work begins until the issue has the `approved` label.

## Codebase State

| Component | Exists | Notes |
|-----------|--------|-------|
| `images/` | Yes | Alpine + Debian + Ubuntu images, shared entrypoint (`images/shared/entrypoint.sh`, `images/alpine/Dockerfile`, `images/debian/Dockerfile`, `images/ubuntu/Dockerfile`) |
| `collector/` | No | Go collector not started |
| `.github/workflows/` | No | No CI pipelines yet |
| `context/` | Yes | Workflow docs and bootstrap spec |
| `.github/ISSUE_TEMPLATE/` | Yes | Feature, bug, enhancement, testing templates |
