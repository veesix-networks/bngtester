# bngtester

<p align="center">
  <a href="https://github.com/veesix-networks/bngtester/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0-blue.svg?style=for-the-badge" alt="GPL-3.0 License"></a>
  <a href="https://dsc.gg/osvbng"><img src="https://img.shields.io/discord/1483536004337107017?label=Discord&logo=discord&logoColor=white&color=5865F2&style=for-the-badge" alt="Discord"></a>
  <a href="https://github.com/veesix-networks/bngtester/issues"><img src="https://img.shields.io/github/issues/veesix-networks/bngtester?style=for-the-badge" alt="Issues"></a>
</p>

Open source BNG subscriber testing framework. Multi-matrix OS testing with in-depth reporting, metrics and built for CI/CD.

> **This project is fully LLM-driven.** All code, specs, and documentation are produced by AI agents following a [structured workflow](context/PROCESS.md). Humans file issues and approve results — agents do the rest. See [Contributing](#contributing) to get involved.

## What is bngtester?

bngtester validates BNG functionality by spinning up real subscribers — the same operating systems and software that run in production networks. A subscriber is whatever device the customer plugs in: a Linux box, a VyOS router, an OPNsense firewall, a Ubiquiti gateway. bngtester tests them all.

It's not a traffic generator or a protocol blaster. Tools like [BNG Blaster](https://github.com/rtbrick/bngblaster) and [Cisco TRex](https://trex-tgn.cisco.com/) already do that well. bngtester fills a different gap: proving that real subscribers on real operating systems can connect through your BNG, get addressing, reach the internet, and maintain quality of service.

## What it does

- Spins up containerized subscribers across multiple operating systems and platforms
- Supports any access method the BNG handles — QinQ, single-tagged VLANs, PPPoE, IPoE
- Obtains addressing via native clients (DHCPv4, DHCPv6) or PPP negotiation depending on the access method
- Runs connectivity and performance tests (ping, iperf3, HTTP, DNS) through the BNG
- Collects structured metrics: throughput, latency, jitter, packet loss
- Produces test reports suitable for CI/CD pipelines
- Validates that different subscriber platforms all work correctly with your BNG across all access methods

## What it is not

bngtester is not designed to stress-test your BNG with maximum PPS or thousands of sessions per second. Use BNG Blaster or TRex for that. bngtester focuses on correctness and real-world validation at modest scale, with reporting and metrics you can integrate into your release process.

## Architecture

```
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│    Alpine    │ │    Debian    │ │    Ubuntu    │
│   (dhcpcd)   │ │  (dhclient)  │ │  (dhclient)  │
│ + bng-client │ │ + bng-client │ │ + bng-client │
└──────┬───────┘ └──────┬───────┘ └──────┬───────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
                 ┌──────┴───────┐
                 │     BNG      │
                 └──────┬───────┘
                        │
                 ┌──────┴───────┐
                 │  bng-server  │
                 │  (metrics +  │
                 │   reports)   │
                 └──────────────┘
```

## Subscriber Images

Custom-built minimal images for Linux subscribers with OS-native DHCP clients and test tools:

| Image | Platform | Status |
|-------|----------|--------|
| `bngtester-alpine` | Alpine (dhcpcd) | Planned |
| `bngtester-debian` | Debian 12 (isc-dhcp-client) | Planned |
| `bngtester-ubuntu` | Ubuntu 22.04 (isc-dhcp-client) | Planned |

For platforms like VyOS, OPNsense, and pfSense, bngtester uses their official images with test configuration — not custom builds. Each platform integration is added via its own issue.

## Status

Early development. Nothing is built yet — the workflow and issue templates are in place, implementation starts from filed issues.

## Contributing

Anyone can contribute. The process is:

1. **File an issue** using one of the [issue templates](https://github.com/veesix-networks/bngtester/issues/new/choose). Be as detailed as possible — the issue is your requirements document. Templates are provided for features, bugs, enhancements, and testing.

2. **Clone the repo** and open it in [Claude Code](https://github.com/anthropics/claude-code):

```bash
git clone https://github.com/veesix-networks/bngtester.git
cd bngtester
claude
```

3. **Give Claude the issue reference** to start the workflow:

```
Read context/PROCESS.md and execute Phase 1 for issue #N.
```

4. Claude reads the issue, reads the codebase, drafts the spec, and drives the rest of the workflow. It will provide prompts for additional review agents (Gemini, Codex) if the issue requested them.

Your job as a human: write a good issue, approve or reject review findings when asked, and merge the PR. Everything else is agent-driven.

### Rules

- **One feature per PR. One PR per issue.** No bundling unrelated changes.
- **No work without an issue.** PRs without a backing issue will be closed.
- Bug fixes and typos still need an issue but can skip the full spec workflow.
- If implementation reveals a needed change outside the issue's scope, file a new issue for it.

### Workflow details

The full agent workflow (phases, formats, severity scales) lives in [`context/PROCESS.md`](context/PROCESS.md). You don't need to read it — the agents do.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

Copyright The bngtester Authors.
