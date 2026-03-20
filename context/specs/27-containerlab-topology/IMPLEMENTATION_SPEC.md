# Implementation Spec: Containerlab Topology with osvbng

## Overview

Containerlab topology definition that deploys osvbng as a VPP-based BNG with a bngtester subscriber container on the access side and an FRR-based server node on the network side. Provides end-to-end IPoE subscriber validation through a real BNG data plane — DHCP lease acquisition, gateway reachability, and cross-BNG connectivity. Includes a shell-based smoke test and deployment documentation.

## Source Issue

[#27 — Containerlab topology with osvbng for end-to-end BNG testing](https://github.com/veesix-networks/bngtester/issues/27)

## Current State

- Subscriber images (Alpine, Debian, Ubuntu) exist at `images/` with the shared entrypoint at `images/shared/entrypoint.sh`. All support IPoE (DHCPv4) with QinQ VLANs and `MGMT_IFACE` default route removal.
- osvbng has a proven containerlab integration pattern in its own test suite (`tests/18-ipoe-linux-client/`), which uses bngtester subscriber images. This spec adapts that pattern into the bngtester repo so the topology is self-contained and independently deployable.
- The Rust collector spec (#5) is finalized but Phase 5 (implementation) has not started. The `bngtester-server` and `bngtester-client` binaries are defined in the spec but do not exist on `main` yet. The topology uses iperf3 as the interim far-side service.
- Robot Framework integration (#13) is the next planned issue. The topology directory structure is designed so #13 can reference it directly from Robot test suites.

## Design

### Topology Architecture

```
                    ┌──────────────────────────────────┐
                    │        Management Network         │
                    │    (containerlab docker bridge)    │
                    └──┬────────────┬───────────────┬──┘
                       │ eth0       │ eth0          │ eth0
                ┌──────┴──────┐ ┌──┴──────────┐ ┌──┴──────────┐
                │ subscriber  │ │    bng1      │ │   server    │
                │ (bngtester) │ │  (osvbng)    │ │   (FRR)     │
                │             │ │              │ │             │
                │ DHCP client │ │ VPP dataplane│ │ OSPF peer   │
                │ QinQ VLANs  │ │ DHCP server  │ │ iperf3      │
                │ iperf3      │ │ OSPF routing │ │ ping target │
                └──────┬──────┘ └──┬───────┬──┘ └──┬──────────┘
                       │ eth1      │ eth1  │ eth2   │ eth1
                       │           │       │        │
                       └───────────┘       └────────┘
                     Access Link          Core Link
                   (BNG ← Subscriber)   (BNG → Network)
```

Three nodes connected by two point-to-point links:

| Node | Image | Role | Interfaces |
|------|-------|------|------------|
| **bng1** | `veesixnetworks/osvbng:latest` | VPP-based BNG — terminates subscriber VLANs, runs DHCP server, OSPF routing | eth0 (mgmt), eth1 (access), eth2 (core) |
| **subscriber** | `veesixnetworks/bngtester:alpine-latest` | Real Linux subscriber — QinQ VLAN setup, DHCPv4 via shared entrypoint | eth0 (mgmt), eth1 (access to BNG) |
| **server** | `frrouting/frr:v8.4.1` | Network-side router — OSPF peer, iperf3/ping endpoint, return route to subscribers | eth0 (mgmt), eth1 (core to BNG) |

### IP Addressing

| Subnet | Purpose | Addresses |
|--------|---------|-----------|
| `10.0.0.0/30` | Core link (BNG ↔ server) | bng1=10.0.0.1, server=10.0.0.2 |
| `10.254.0.1/32` | BNG control plane loopback | bng1 (OSPF router-id) |
| `10.254.0.2/32` | Server loopback | server (OSPF router-id) |
| `10.255.0.0/16` | Subscriber pool (DHCP) | Gateway=10.255.0.1, subscribers get 10.255.0.x |

### VLAN Scheme

| Parameter | Value | Notes |
|-----------|-------|-------|
| S-VLAN | 100 | osvbng subscriber group accepts 100-110 |
| C-VLAN | 10 | Any C-VLAN within the S-VLAN range |
| Resulting interface | `eth1.100.10` | Created by entrypoint.sh on subscriber |

### Subscriber Flow

1. Containerlab deploys all three nodes and creates veth pairs
2. Subscriber entrypoint.sh waits for `eth1`, creates QinQ interface `eth1.100.10` (S-VLAN 100, C-VLAN 10)
3. `MGMT_IFACE=eth0` removes the management default route so DHCP-acquired route takes precedence
4. DHCPv4 client (dhcpcd on Alpine) sends DHCP Discover on `eth1.100.10`
5. osvbng VPP receives tagged frame on eth1, matches subscriber group (S-VLAN 100, C-VLAN any)
6. osvbng built-in DHCP server assigns IP from `10.255.0.0/16` pool with gateway `10.255.0.1`
7. Subscriber gets IP, default route via BNG — can now reach server at `10.0.0.2` through the BNG
8. OSPF between bng1 and server ensures bidirectional routing: server has a route back to `10.255.0.0/16` via bng1

### osvbng Configuration

Adapted from osvbng's own `tests/18-ipoe-linux-client/config/bng1/osvbng.yaml` with simplifications (no PPPoE, no IPv6, no BGP/MPLS/CGNAT). The following sections are required in `lab/config/bng1/osvbng.yaml`:

| Section | Purpose | Key Settings |
|---------|---------|--------------|
| `interfaces` | Define BNG interfaces | `eth1` with `bng_mode: access`; `eth2` with `bng_mode: core`, `lcp: true`, address `10.0.0.1/30`; `loop0` (router-id `10.254.0.1/32`, `lcp: true`); `loop100` (subscriber gateway `10.255.0.1/32`, `lcp: true`) |
| `subscriber-groups` | Match subscriber VLANs to IPoE | Group `default`: `access-type: ipoe`, S-VLAN `100-110`, C-VLAN `any`, interface `loop100`, `aaa-policy: default-policy` |
| `ipv4-profiles` | DHCP pool and gateway | Profile `default`: gateway `10.255.0.1`, pool `10.255.0.0/16`, DNS `8.8.8.8`/`8.8.4.4`, lease-time `3600` |
| `dhcp` | Enable built-in DHCP | `provider: local` |
| `aaa` | Authentication policy | `auth_provider: local`, policy `default-policy` with `format: $mac-address$`, `max_concurrent_sessions: 1` |
| `plugins` | API and local auth | `northbound.api` on `:8080` (required for session verification); `subscriber.auth.local` with `allow_all: true` |
| `protocols.ospf` | Core routing | `router-id: 10.254.0.1`, area `0.0.0.0` with `eth2` (point-to-point), `loop0` and `loop100` (passive) |
| `dataplane` | VPP LCP integration | `lcp-netns: dataplane` — required for VPP to sync interfaces into the Linux control plane namespace where FRR and DHCP operate |
| `logging` | Debug output | `format: text`, `level: info` |

### Server Configuration (FRR)

Minimal FRR setup matching osvbng test 18's corerouter1 pattern:

- **OSPF** — point-to-point adjacency with bng1 on core link, loopback `10.254.0.2/32` as passive
- **Static route** — `10.255.0.0/16 via 10.0.0.1` as fallback for initial convergence. The smoke test validates OSPF adjacency independently so this route does not mask routing failures.
- **IP forwarding** enabled
- **iperf3** — installed via `apk add` in the entrypoint script (FRR image is Alpine-based). Runs as a daemon (`iperf3 -s -D`) so it is available for throughput tests without manual setup.
- **Startup script** — waits for eth1, enables forwarding, installs iperf3, starts FRR, reloads config

### Smoke Test

Shell script that validates the topology post-deployment. Each stage has a concrete timeout and dumps diagnostic output on failure.

| Stage | Check | Timeout | On Failure |
|-------|-------|---------|------------|
| 1 | osvbng healthy (log marker: `"osvbng started successfully"`) | 120s (12 retries × 10s) | Dump `docker logs clab-bngtester-bng1` |
| 2 | Subscriber QinQ interface `eth1.100.10` exists | 60s (12 retries × 5s) | Dump `ip link` and subscriber container logs |
| 3 | Subscriber has non-link-local IPv4 on `eth1.100.10` | 90s (18 retries × 5s) | Dump `ip addr`, `ip route`, subscriber logs |
| 4 | OSPF adjacency established (server sees bng1 as Full neighbor) | 60s (12 retries × 5s) | Dump `vtysh -c "show ip ospf neighbor"` on server and bng1 VPP OSPF state |
| 5 | Ping gateway (`10.255.0.1`) from subscriber | 3 packets, 2s timeout | Dump subscriber routes |
| 6 | Ping server (`10.0.0.2`) from subscriber through BNG | 30s (6 retries × 5s) | Dump routes on both subscriber and server |
| 7 | iperf3 throughput (subscriber → server) | 5s test | Log result; non-fatal if iperf3 fails |

Exit code 0 on success, non-zero on any failure at stages 1-6. Stage 7 (iperf3) is informational — logged but does not fail the smoke test. The script accepts the lab name as an argument (default: `bngtester`) to derive container names (`clab-<lab-name>-<node>`).

## Configuration

### Environment Variables (Containerlab)

Override at deploy time via shell environment:

| Variable | Default | Purpose |
|----------|---------|---------|
| `OSVBNG_IMAGE` | `veesixnetworks/osvbng:latest` | osvbng Docker image |
| `BNGTESTER_IMAGE` | `veesixnetworks/bngtester:alpine-latest` | Subscriber image (swap for debian/ubuntu) |

Example: `BNGTESTER_IMAGE=veesixnetworks/bngtester:debian-latest clab deploy -t lab/bngtester.clab.yml`

### Environment Variables (Subscriber Container)

Set in the topology file, consumed by `images/shared/entrypoint.sh`:

| Variable | Value | Purpose |
|----------|-------|---------|
| `ACCESS_METHOD` | `dhcpv4` | IPoE via DHCPv4 |
| `ENCAP` | `qinq` | QinQ VLAN encapsulation |
| `PHYSICAL_IFACE` | `eth1` | Access interface toward BNG |
| `SVLAN` | `100` | Outer VLAN tag |
| `CVLAN` | `10` | Inner VLAN tag |
| `MGMT_IFACE` | `eth0` | Remove management default route |

### osvbng Container Requirements

| Capability | Why |
|------------|-----|
| `SYS_ADMIN` | VPP hugepage allocation, namespace management |
| `NET_ADMIN` | Interface configuration, VPP dataplane |
| `IPC_LOCK` | VPP shared memory (stats segment) |
| `SYS_NICE` | VPP worker thread priority |

### Subscriber Container Requirements

| Capability | Why |
|------------|-----|
| `NET_ADMIN` | VLAN interface creation, IP configuration |

## File Plan

### New Files

| File | Purpose |
|------|---------|
| `lab/bngtester.clab.yml` | Containerlab topology definition (3 nodes, 2 links) |
| `lab/config/bng1/osvbng.yaml` | osvbng IPoE configuration (subscriber groups, DHCP, OSPF, interfaces) |
| `lab/config/server/daemons` | FRR daemon selection (zebra, ospfd, staticd) |
| `lab/config/server/frr.conf` | FRR routing config (OSPF, static route to subscriber pool) |
| `lab/config/server/entrypoint.sh` | Server startup script (wait for interface, enable forwarding, start FRR) |
| `lab/smoke-test.sh` | Post-deployment validation script |
| `lab/README.md` | Deployment guide: prerequisites, deploy/destroy, image override, troubleshooting (including QinQ MTU overhead note) |
| `context/specs/27-containerlab-topology/IMPLEMENTATION_SPEC.md` | This spec |
| `context/specs/27-containerlab-topology/README.md` | Status tracker |

### Modified Files

None. This is a new standalone directory.

## Implementation Order

### Phase A: Topology and Configuration Files

Create the lab directory structure with all configuration files:

1. `lab/bngtester.clab.yml` — topology definition
2. `lab/config/bng1/osvbng.yaml` — osvbng config
3. `lab/config/server/daemons` — FRR daemons
4. `lab/config/server/frr.conf` — FRR routing
5. `lab/config/server/entrypoint.sh` — server startup

**Testable:** `clab deploy -t lab/bngtester.clab.yml` creates all three containers. `clab inspect -t lab/bngtester.clab.yml` shows all nodes in running state.

### Phase B: Smoke Test Script

Create `lab/smoke-test.sh` with retry-loop validation checks. Must be runnable standalone after `clab deploy`.

**Testable:** `./lab/smoke-test.sh` exits 0 when the topology is healthy, non-zero on failure.

### Phase C: Documentation

Create `lab/README.md` covering prerequisites, deployment, image overrides, troubleshooting, and the relationship to future Robot tests (#13) and Rust collector (#5).

**Testable:** Documentation accurately describes the deployment workflow.

## Testing

### Manual Validation (Deployment Host)

Prerequisites: containerlab installed, Docker running, osvbng image available (pull or local build).

1. **Deploy:** `clab deploy -t lab/bngtester.clab.yml`
2. **Inspect:** `clab inspect -t lab/bngtester.clab.yml` — all 3 nodes running
3. **Smoke test:** `./lab/smoke-test.sh` — exits 0
4. **Manual checks:**
   - `docker exec clab-bngtester-subscriber ip -4 addr show eth1.100.10` — has 10.255.x.x address
   - `docker exec clab-bngtester-subscriber ping -c 3 10.255.0.1` — gateway reachable
   - `docker exec clab-bngtester-subscriber ping -c 3 10.0.0.2` — server reachable through BNG
   - `docker exec clab-bngtester-bng1 curl -s http://localhost:8080/api/show/subscriber/sessions | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('data',[])));"` — shows 1 session
5. **Image swap:** `BNGTESTER_IMAGE=veesixnetworks/bngtester:debian-latest clab deploy -t lab/bngtester.clab.yml --reconfigure` — same smoke test passes with Debian subscriber
6. **Destroy:** `clab destroy -t lab/bngtester.clab.yml --cleanup` — clean teardown

### What Is Not Tested

- PPPoE (out of scope, IPoE first)
- Multiple concurrent subscribers (out of scope per issue)
- QoS/traffic shaping (Rust collector scope)
- CI-automated deployment (requires self-hosted runner with containerlab)
- Robot Framework execution (#13)

## Not In Scope

- **PPPoE subscriber termination** — IPoE first, PPPoE in a follow-up issue
- **Multi-subscriber scale testing** — single subscriber validates the path; scale is a separate concern
- **QoS/traffic shaping validation** — reserved for Rust collector (#5)
- **Robot Framework integration** — #13 will add Robot tests that reference this topology
- **Rust collector integration** — #5 Phase E (bngtester-server binary) is not yet implemented
- **CI automation of the topology** — requires a self-hosted runner with containerlab and osvbng image access
- **IPv6 subscriber testing** — IPoE DHCPv4 only in this issue
