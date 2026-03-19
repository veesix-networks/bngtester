# Implementation Spec: Alpine Subscriber Image with Shared Entrypoint

## 1. Overview

Alpine Linux subscriber container image with a shared entrypoint script that configures VLAN encapsulation and obtains addressing via IPoE (DHCPv4/DHCPv6) or PPPoE. This is the first subscriber image and establishes the shared entrypoint that all future images (Debian, Ubuntu, etc.) will reuse.

## 2. Source Issue

[#1 — Alpine subscriber image with shared entrypoint](https://github.com/veesix-networks/bngtester/issues/1)

## 3. Current State

No subscriber images exist. The `images/` directory does not exist yet. The repository contains only the AI workflow (`context/`), issue templates (`.github/ISSUE_TEMPLATE/`), and project documentation.

## 4. Design

### Architecture

```
images/
├── shared/
│   └── entrypoint.sh    # Shared across all subscriber images
└── alpine/
    └── Dockerfile        # Alpine-specific image definition
```

The build context for each image is `images/`, allowing Dockerfiles to `COPY` from `shared/`. Each image directory contains only its Dockerfile — the entrypoint is shared.

### Entrypoint Flow

```
entrypoint.sh
  │
  ├── 1. Register cleanup trap (trap cleanup EXIT)
  ├── 2. Validate environment variables
  │     ├── ACCESS_METHOD ∈ {dhcpv4, dhcpv6, pppoe}
  │     ├── ENCAP ∈ {untagged, single, qinq}
  │     ├── CVLAN required if ENCAP=single|qinq, must be 1-4094
  │     ├── SVLAN required if ENCAP=qinq, must be 1-4094
  │     ├── PPPOE_USER + PPPOE_PASSWORD required if ACCESS_METHOD=pppoe
  │     └── Derived interface name must be ≤ 15 bytes
  ├── 3. Wait for physical interface
  │     ├── Poll /sys/class/net/$PHYSICAL_IFACE (with IFACE_WAIT_TIMEOUT)
  │     ├── ip link set $PHYSICAL_IFACE up
  │     └── Poll operstate = up|unknown (with timeout)
  ├── 4. Configure encapsulation
  │     ├── untagged  → use physical interface directly
  │     ├── single    → ip link add ... (check exit code, diagnostic on failure)
  │     └── qinq      → create S-VLAN, then C-VLAN (check each, diagnostic on failure)
  ├── 5. Bring up the target interface
  ├── 6. Auto-detect DHCP client (command -v dhcpcd || command -v dhclient)
  └── 7. Obtain addressing (via dispatch functions)
        ├── dhcpv4    → start_dhcpv4: dhcpcd or dhclient (auto-detected); wait $PID
        ├── dhcpv6    → start_dhcpv6: dhcpcd or dhclient (auto-detected); wait $PID
        └── pppoe     → start_pppoe: exec pppd ... (replaces shell, same on all distros)
```

### Key Design Decisions

**POSIX sh, not bash.** The entrypoint uses `#!/bin/sh` for Alpine compatibility (busybox ash). Debian and Ubuntu also support POSIX sh, so this works everywhere without requiring bash as a dependency.

**Build context is `images/`, not `images/alpine/`.** This allows `COPY shared/entrypoint.sh` to work from any image's Dockerfile. Build command: `docker build -f images/alpine/Dockerfile images/`.

**DHCP client auto-detection.** The entrypoint detects the available DHCP client at runtime (`command -v dhcpcd` / `command -v dhclient`) and dispatches accordingly. Both `dhcpcd` and `dhclient` implementations are built into the dispatch functions (`start_dhcpv4`, `start_dhcpv6`, `stop_client`). This means the same entrypoint works unmodified on Alpine (dhcpcd) and Debian/Ubuntu (dhclient) — no per-image overrides or configuration needed. PPPoE uses `pppd` which is identical across distros.

**Long-term: `bng-client` replaces the entrypoint.** The planned Rust binary (`bng-client`) will handle VLAN setup, client launch, health reporting, and signal handling in a single compiled binary. When it lands, it replaces the shell entrypoint entirely. The current entrypoint is the minimum viable approach to get working subscriber containers before the Rust tooling exists.

**pppd uses `exec` to become PID 1.** For PPPoE, the entrypoint uses `exec pppd` so pppd replaces the shell and receives signals directly. The `nodetach` flag keeps pppd in the foreground. The container exits when pppd exits.

**No `--privileged` assumption in the entrypoint.** The script uses standard `ip link` commands. The container must be run with `--cap-add=NET_ADMIN` (and `--cap-add=NET_RAW` for PPPoE) but does not require full `--privileged` mode. This is a runtime concern, not an entrypoint concern.

### Signal Handling and Cleanup

A single idempotent `cleanup()` function is registered via `trap cleanup EXIT`. This runs on every exit path — signals (SIGTERM, SIGINT), errors, and normal exit. The function:

1. Releases DHCP leases via `stop_client()` (auto-detected: `dhcpcd -k` or `dhclient -r`) or kills the PPP daemon
2. Removes VLAN sub-interfaces in reverse creation order (C-VLAN first, then S-VLAN)
3. Is safe to call multiple times (checks interface/process existence before acting)

For DHCP methods, the entrypoint runs the client in the background and uses `wait $PID`. Signals interrupt `wait`, the trap fires, cleanup runs. For PPPoE, `exec pppd` replaces the shell — pppd handles signals directly and the cleanup trap does not apply (pppd manages its own teardown).

### Failure and Readiness Contract

Each access method defines explicit success/failure semantics:

| Access Method | Success Condition | Timeout | Failure Behavior |
|---------------|-------------------|---------|-----------------|
| `dhcpv4` | Lease obtained (DHCP client exits 0) | `DHCP_TIMEOUT` (default: 60s) via `dhcpcd -t` or `dhclient -timeout` | Client exits non-zero → container exits non-zero |
| `dhcpv6` | Lease/prefix obtained (DHCP client exits 0) | `DHCP_TIMEOUT` (default: 60s) via `dhcpcd -t` or `dhclient -timeout` | Client exits non-zero → container exits non-zero |
| `pppoe` | LCP + auth + IPCP complete (pppd stays running) | pppd's built-in LCP timeout | Auth failure or LCP timeout → pppd exits non-zero → container exits non-zero. Uses `persist` + `maxfail 0` for retry on transient failures. |

**Exit code contract:** The container's exit code is the client's exit code. Zero = session ended normally. Non-zero = acquisition failed or session dropped. CI and orchestration can rely on this to distinguish success from failure.

### Runtime Network Model

Subscriber containers require a dedicated network interface into the BNG-facing network. The default Docker bridge (`eth0`) is **not** suitable for subscriber testing because it is runtime-managed and does not represent a real subscriber attachment.

Supported attachment models:

1. **`--network none` + injected veth/macvlan** — orchestrator creates the interface after container start. Set `PHYSICAL_IFACE` to the injected interface name.
2. **Dedicated Docker/podman network** — a bridge or macvlan network attached to the BNG-facing segment. The container's interface in that network becomes `PHYSICAL_IFACE`.
3. **Host network passthrough** — `--network host` with `PHYSICAL_IFACE` set to a host interface. Useful for bare-metal testing.

The entrypoint does not create or manage the network attachment — it assumes `PHYSICAL_IFACE` will appear within `IFACE_WAIT_TIMEOUT` seconds. Network topology setup is the orchestrator's responsibility.

### VLAN Configuration Detail

**Untagged:**
No VLAN configuration. The target interface is the physical interface itself.

**Single-tagged (802.1Q):**
```sh
ip link add link $PHYSICAL_IFACE name ${PHYSICAL_IFACE}.${CVLAN} type vlan id $CVLAN || {
    echo "ERROR: VLAN creation failed. Check 8021q kernel module and NET_ADMIN capability." >&2
    exit 1
}
ip link set ${PHYSICAL_IFACE}.${CVLAN} up
```
Target interface: `${PHYSICAL_IFACE}.${CVLAN}`

**QinQ (802.1ad):**
```sh
ip link add link $PHYSICAL_IFACE name ${PHYSICAL_IFACE}.${SVLAN} type vlan id $SVLAN protocol 802.1ad || {
    echo "ERROR: S-VLAN creation failed. Check 8021q/8021ad kernel module and NET_ADMIN capability." >&2
    exit 1
}
ip link set ${PHYSICAL_IFACE}.${SVLAN} up
ip link add link ${PHYSICAL_IFACE}.${SVLAN} name ${PHYSICAL_IFACE}.${SVLAN}.${CVLAN} type vlan id $CVLAN || {
    echo "ERROR: C-VLAN creation failed." >&2
    exit 1
}
ip link set ${PHYSICAL_IFACE}.${SVLAN}.${CVLAN} up
```
Target interface: `${PHYSICAL_IFACE}.${SVLAN}.${CVLAN}`

On any `ip link add` failure, the `cleanup` trap (registered on EXIT) removes any interfaces already created before the container exits.

## 5. Configuration

All configuration is via environment variables. No config files.

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ACCESS_METHOD` | No | `dhcpv4` | Access method: `dhcpv4`, `dhcpv6`, `pppoe` |
| `ENCAP` | No | `untagged` | Encapsulation: `untagged`, `single`, `qinq` |
| `PHYSICAL_IFACE` | No | `eth0` | Physical interface name inside the container |
| `SVLAN` | If `qinq` | — | Outer S-VLAN ID (QinQ only) |
| `CVLAN` | If `single` or `qinq` | — | C-VLAN ID (single-tagged or QinQ inner) |
| `IFACE_WAIT_TIMEOUT` | No | `30` | Seconds to wait for physical interface to appear |
| `DHCP_TIMEOUT` | No | `60` | Seconds for DHCP lease acquisition (passed to `dhcpcd -t`) |
| `PPPOE_USER` | If `pppoe` | — | PPPoE username |
| `PPPOE_PASSWORD` | If `pppoe` | — | PPPoE password |
| `PPPOE_SERVICE` | No | — | PPPoE service name (optional, for filtering) |

### Example Usage

**IPoE DHCPv4 with single-tagged VLAN (dedicated network):**
```sh
docker run --rm --cap-add=NET_ADMIN \
  --network bng-subscribers \
  -e ACCESS_METHOD=dhcpv4 \
  -e ENCAP=single \
  -e CVLAN=100 \
  bngtester-alpine
```

**PPPoE over QinQ (dedicated network):**
```sh
docker run --rm --cap-add=NET_ADMIN --cap-add=NET_RAW \
  --network bng-subscribers \
  -e ACCESS_METHOD=pppoe \
  -e ENCAP=qinq \
  -e SVLAN=100 \
  -e CVLAN=200 \
  -e PPPOE_USER=user@isp \
  -e PPPOE_PASSWORD=secret \
  bngtester-alpine
```

In both examples, `bng-subscribers` is a Docker network (macvlan or bridge) attached to the BNG-facing segment. See "Runtime Network Model" in Design for supported attachment models.

## 6. File Plan

| File | Action | Purpose |
|------|--------|---------|
| `images/shared/entrypoint.sh` | Create | Shared entrypoint script — VLAN config, addressing, signal handling |
| `images/alpine/Dockerfile` | Create | Alpine image — installs packages, copies shared entrypoint |

Both files get SPDX copyright headers.

## 7. Implementation Order

### Phase A: Shared Entrypoint Script

Create `images/shared/entrypoint.sh` with:

1. `cleanup()` function registered via `trap cleanup EXIT` — idempotent, runs on every exit path
2. Environment variable validation:
   - ACCESS_METHOD must be one of: `dhcpv4`, `dhcpv6`, `pppoe`
   - ENCAP must be one of: `untagged`, `single`, `qinq`
   - CVLAN required and in 1-4094 range if ENCAP is `single` or `qinq`
   - SVLAN required and in 1-4094 range if ENCAP is `qinq`
   - PPPOE_USER and PPPOE_PASSWORD required if ACCESS_METHOD is `pppoe`
   - Derived interface name must be ≤ 15 bytes
3. Physical interface wait: poll `/sys/class/net/$PHYSICAL_IFACE` with timeout, then `ip link set up`, then poll operstate for `up` or `unknown`
4. VLAN interface creation with error checking on each `ip link add` (diagnostic message on failure pointing at 8021q module / NET_ADMIN)
5. Auto-detect DHCP client: set `DHCP_CLIENT=dhcpcd` or `DHCP_CLIENT=dhclient` based on `command -v`. Exit with error if neither is found and ACCESS_METHOD is dhcpv4/dhcpv6.
6. Dispatch functions with both client implementations:
   - `start_dhcpv4()`:
     - dhcpcd: `dhcpcd -4 -f -t $DHCP_TIMEOUT $TARGET_IFACE`
     - dhclient: `dhclient -4 -v -timeout $DHCP_TIMEOUT $TARGET_IFACE`
   - `start_dhcpv6()`:
     - dhcpcd: `dhcpcd -6 -f -t $DHCP_TIMEOUT $TARGET_IFACE`
     - dhclient: `dhclient -6 -v -timeout $DHCP_TIMEOUT $TARGET_IFACE`
   - `start_pppoe()`: `exec pppd plugin pppoe.so $TARGET_IFACE user "$PPPOE_USER" password "$PPPOE_PASSWORD" nodetach noauth defaultroute usepeerdns persist maxfail 0` (same on all distros)
   - `stop_client()`:
     - dhcpcd: `dhcpcd -k $TARGET_IFACE`
     - dhclient: `dhclient -r $TARGET_IFACE`
     - pppd: `kill $PID`
7. For DHCP methods: run client in background, `wait $PID`, propagate exit code

**Testable independently:** `shellcheck` in POSIX mode, plus functional tests in any container with `iproute2`.

### Phase B: Alpine Dockerfile

Create `images/alpine/Dockerfile`:

1. `FROM alpine:3.21`
2. `RUN apk add --no-cache dhcpcd ppp ppp-pppoe iputils iproute2 iperf3`
3. `COPY shared/entrypoint.sh /entrypoint.sh`
4. `RUN chmod +x /entrypoint.sh`
5. `ENTRYPOINT ["/entrypoint.sh"]`

**Testable independently:** `docker build -f images/alpine/Dockerfile images/` must succeed. Image contains all expected binaries.

## 8. Testing

### Build Validation

- `docker build -f images/alpine/Dockerfile images/` succeeds
- Image contains expected binaries: `dhcpcd`, `pppd`, `ping`, `ip`, `iperf3`
- Entrypoint is executable and set correctly

### Static Analysis

- `shellcheck images/shared/entrypoint.sh` passes (POSIX sh mode)

### Functional Tests (require NET_ADMIN capability)

These tests validate the entrypoint logic. They require a container runtime with `--cap-add=NET_ADMIN`.

| Test | What It Validates |
|------|-------------------|
| Untagged + DHCPv4 | Default path — container starts, waits for interface, runs dhcpcd |
| Single-tagged VLAN | VLAN sub-interface is created with correct ID |
| QinQ | Both S-VLAN and C-VLAN interfaces are created, correct protocol (802.1ad) |
| Missing CVLAN with `ENCAP=single` | Entrypoint exits with error, non-zero exit code |
| Missing SVLAN with `ENCAP=qinq` | Entrypoint exits with error, non-zero exit code |
| Interface wait timeout | Entrypoint exits with error after timeout when interface doesn't appear |
| SIGTERM cleanup | VLAN interfaces are removed on container stop |
| PPPoE launch | pppd starts with nodetach, noauth, defaultroute, usepeerdns flags |
| Invalid ACCESS_METHOD | Entrypoint exits with error |
| Invalid ENCAP | Entrypoint exits with error |
| VLAN ID out of range | SVLAN=0 or CVLAN=5000 exits with error |
| Interface name too long | Long PHYSICAL_IFACE + VLAN IDs exceeding 15 bytes exits with error |
| Missing PPPOE_USER with `ACCESS_METHOD=pppoe` | Entrypoint exits with error |
| VLAN creation without NET_ADMIN | Diagnostic error message pointing at capability/module |
| Partial VLAN failure (QinQ) | S-VLAN created but C-VLAN fails → S-VLAN cleaned up on exit |
| DHCP timeout | dhcpcd exits non-zero after DHCP_TIMEOUT → container exits non-zero |

### End-to-End (out of scope for this issue)

Full BNG integration testing (subscriber connects through BNG, gets addressing, passes traffic) requires the server component and test orchestration, which are separate issues.

## 9. Not In Scope

- **Debian and Ubuntu images** — separate issues, will reuse `images/shared/entrypoint.sh`
- **CI pipeline to publish the image** — separate issue for GHCR publishing
- **Collector client binary (`bng-client`)** — the Rust binary that runs inside containers for traffic/metrics; separate issue
- **Multi-arch builds** — ARM/x86 builds are a future enhancement
- **`bng-client` Rust binary** — the planned binary that will eventually replace the shell entrypoint with compiled VLAN setup, client management, health reporting, and signal handling. Separate issue.
