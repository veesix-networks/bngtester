# Implementation Spec: Debian Subscriber Image

## 1. Overview

Debian 12 (Bookworm) subscriber container image using isc-dhcp-client (dhclient) for DHCPv4/DHCPv6 and ppp for PPPoE. Reuses the shared entrypoint from issue #1 with minimal targeted fixes for dhclient compatibility (`DHCP_TIMEOUT` support and lifecycle consistency). This is the first image to exercise the entrypoint's dhclient dispatch path with a real Debian package set.

## 2. Source Issue

[#3 — Debian subscriber image](https://github.com/veesix-networks/bngtester/issues/3)

## 3. Current State

The shared entrypoint (`images/shared/entrypoint.sh`) and Alpine image (`images/alpine/Dockerfile`) are complete from issue #1. The entrypoint supports dhclient via `command -v` auto-detection and has dispatch functions for both clients. However, the dhclient path has two gaps: `DHCP_TIMEOUT` is not passed to dhclient (it lacks a CLI flag — requires a config file), and container lifecycle behavior differs between dhcpcd and dhclient. No `images/debian/` directory exists yet.

## 4. Design

### Architecture

The Debian image follows the same pattern established by the Alpine image:

```
images/
├── shared/
│   └── entrypoint.sh        # Already exists — unchanged
├── alpine/
│   └── Dockerfile            # Already exists — unchanged
└── debian/
    └── Dockerfile            # NEW — this spec
```

The build context remains `images/`, allowing the Dockerfile to `COPY shared/entrypoint.sh`.

### Package Mapping

The issue specifies Debian packages for the same capabilities Alpine provides:

| Capability | Alpine Package | Debian Package |
|-----------|---------------|----------------|
| DHCP client | `dhcpcd` (dhcpcd) | `isc-dhcp-client` (dhclient) |
| PPPoE | `ppp` + `ppp-pppoe` | `ppp` |
| Ping | `iputils` | `iputils-ping` |
| IP utilities | `iproute2` | `iproute2` |
| Bandwidth testing | `iperf3` | `iperf3` |
| HTTP client | — | `curl` |

Notable differences:
- **DHCP client is dhclient, not dhcpcd.** This is the key behavioral difference. The shared entrypoint detects this via `command -v dhclient` and dispatches to `dhclient -4 -v -1 -d` (DHCPv4) or `dhclient -6 -v -1 -d` (DHCPv6).
- **curl is explicitly requested** in the issue's acceptance criteria. Alpine's image does not include curl — Debian adds it. `ca-certificates` is also required for HTTPS support.
- **`netbase` is required on bookworm-slim.** The slim image lacks `/etc/protocols` and `/etc/services`, which some networking tools depend on for protocol/service name resolution.
- **ppp-pppoe is not a separate package on Debian.** The `ppp` package includes the PPPoE plugin (`/usr/lib/pppd/*/pppoe.so`).

### Key Design Decisions

**Minimal entrypoint fixes for dhclient compatibility.** The shared entrypoint's dhclient dispatch path has two gaps exposed by this image. Rather than blocking on a separate issue #1 amendment, targeted fixes are in scope:

1. **`DHCP_TIMEOUT` for dhclient.** dhclient has no CLI flag for timeout — it reads `timeout N;` from `dhclient.conf`. The entrypoint will generate a minimal config file (`/tmp/dhclient-bngtester.conf`) with the timeout value and pass it via `dhclient -cf`. This is 2 lines per dispatch function.
2. **Container lifecycle consistency.** `dhclient -1 -d` stays in foreground managing renewals (container stays alive). `dhcpcd -B` may exit after lease acquisition (container exits). During implementation, verify dhcpcd's behavior with `-B` on Alpine 3.21 — if it exits after lease, change to `-f` (foreground) so both clients keep the container alive for post-lease testing.

**Debian 12 (Bookworm).** Current stable release. Using `debian:bookworm-slim` as the base to minimize image size while keeping a full apt ecosystem.

**Single RUN layer for package installation.** All packages installed in one `apt-get install` command with `--no-install-recommends` and cache cleanup in the same layer to keep the image small. Includes `ca-certificates` (for `curl` HTTPS) and `netbase` (for `/etc/protocols` and `/etc/services` missing in bookworm-slim).

**pppoe.so plugin path.** On Debian, the PPPoE plugin is at `/usr/lib/pppd/<version>/pppoe.so`. The entrypoint uses `plugin pppoe.so` (without a full path) — pppd resolves this relative to its plugin directory automatically, so no path differences matter.

## 5. Configuration

No new configuration. The Debian image uses the exact same environment variables as the Alpine image, documented in the [issue #1 spec](../1-alpine-subscriber-image/IMPLEMENTATION_SPEC.md#5-configuration):

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ACCESS_METHOD` | No | `dhcpv4` | `dhcpv4`, `dhcpv6`, `pppoe` |
| `ENCAP` | No | `untagged` | `untagged`, `single`, `qinq` |
| `PHYSICAL_IFACE` | No | `eth0` | Physical interface inside container |
| `SVLAN` | If `qinq` | — | Outer S-VLAN ID |
| `CVLAN` | If `single`/`qinq` | — | C-VLAN ID |
| `IFACE_WAIT_TIMEOUT` | No | `30` | Seconds to wait for interface |
| `DHCP_TIMEOUT` | No | `60` | Seconds for DHCP lease acquisition |
| `PPPOE_USER` | If `pppoe` | — | PPPoE username |
| `PPPOE_PASSWORD` | If `pppoe` | — | PPPoE password |
| `PPPOE_SERVICE` | No | — | PPPoE service name |

### Example Usage

**IPoE DHCPv4 with QinQ:**
```sh
docker build -f images/debian/Dockerfile images/ -t bngtester-debian
docker run --rm --cap-add=NET_ADMIN \
  --network bng-subscribers \
  -e ACCESS_METHOD=dhcpv4 \
  -e ENCAP=qinq \
  -e SVLAN=100 \
  -e CVLAN=200 \
  bngtester-debian
```

**PPPoE untagged:**
```sh
docker run --rm --cap-add=NET_ADMIN --cap-add=NET_RAW \
  --network bng-subscribers \
  -e ACCESS_METHOD=pppoe \
  -e PPPOE_USER=user@isp \
  -e PPPOE_PASSWORD=secret \
  bngtester-debian
```

## 6. File Plan

| File | Action | Purpose |
|------|--------|---------|
| `images/debian/Dockerfile` | Create | Debian 12 subscriber image — installs isc-dhcp-client, ppp, test tools, copies shared entrypoint |
| `images/shared/entrypoint.sh` | Modify | Add dhclient.conf generation for DHCP_TIMEOUT; verify/fix dhcpcd foreground behavior |

## 7. Implementation Order

### Phase A: Entrypoint Fixes

Modify `images/shared/entrypoint.sh`:

1. **DHCP_TIMEOUT for dhclient:** In `start_dhcpv4()` and `start_dhcpv6()`, before the dhclient launch, generate a minimal config:
   ```sh
   printf 'timeout %s;\n' "$DHCP_TIMEOUT" > /tmp/dhclient-bngtester.conf
   dhclient -4 -v -1 -d -cf /tmp/dhclient-bngtester.conf "$TARGET_IFACE" &
   ```
2. **Lifecycle consistency:** Verify whether `dhcpcd -B` on Alpine 3.21 stays alive or exits after lease. If it exits, change `-B` to `-f` in both `start_dhcpv4()` and `start_dhcpv6()`.

**Testable independently:** Rebuild Alpine image, verify dhcpcd still works with the flag change. Verify `DHCP_TIMEOUT` is honored by dhclient.

### Phase B: Debian Dockerfile

Create `images/debian/Dockerfile`:

1. `FROM debian:bookworm-slim`
2. `RUN apt-get update && apt-get install -y --no-install-recommends isc-dhcp-client ppp iputils-ping iproute2 iperf3 curl ca-certificates netbase && rm -rf /var/lib/apt/lists/*`
3. `COPY shared/entrypoint.sh /entrypoint.sh`
4. `RUN chmod +x /entrypoint.sh`
5. `ENTRYPOINT ["/entrypoint.sh"]`

SPDX copyright header at the top.

**Testable independently:** `docker build -f images/debian/Dockerfile images/` must succeed. Image contains all expected binaries (`dhclient`, `pppd`, `ping`, `ip`, `iperf3`, `curl`).

## 8. Testing

### Build Validation

- `docker build -f images/debian/Dockerfile images/` succeeds
- Image contains expected binaries: `dhclient`, `pppd`, `ping`, `ip`, `iperf3`, `curl`
- `curl https://...` works (validates `ca-certificates` is present)
- Entrypoint is executable and set correctly (`docker inspect` confirms entrypoint is `/entrypoint.sh`)
- Dockerfile uses `COPY shared/entrypoint.sh` (not a forked copy)
- SPDX copyright header present
- PPPoE plugin present: `pppoe.so` exists under `/usr/lib/pppd/`

### DHCP Client Detection

- Container starts and the entrypoint detects `dhclient` (not `dhcpcd`)
- Log output shows `bngtester: Detected DHCP client: dhclient`

### Functional Tests (require NET_ADMIN capability)

| Test | What It Validates |
|------|-------------------|
| Untagged + DHCPv4 | dhclient obtains lease via default path |
| Single-tagged VLAN + DHCPv4 | VLAN sub-interface created, dhclient obtains lease through it |
| QinQ + DHCPv4 | S-VLAN + C-VLAN created, dhclient obtains lease through QinQ |
| Missing CVLAN with `ENCAP=single` | Entrypoint exits with error |
| PPPoE launch | pppd starts with correct flags |
| DHCP_TIMEOUT honored | Non-default DHCP_TIMEOUT value appears in generated dhclient.conf |
| SIGTERM cleanup + lease release | VLAN interfaces removed; verify lease release via address-state check (`ip addr show` confirms no address) or DHCP server log showing DHCPRELEASE |
| Container stays alive after lease | Container persists after lease acquisition (both Alpine and Debian) for post-lease testing |

### Behavioral Differences to Watch

dhclient behaves differently from dhcpcd in several ways:

- **Lease file location:** dhclient writes to `/var/lib/dhcp/dhclient.leases` (vs dhcpcd's `/var/lib/dhcpcd/`). Not an entrypoint concern — both clients manage their own lease files.
- **Foreground mode:** dhclient uses `-d` for foreground mode. dhcpcd uses `-B` (or `-f` if changed for lifecycle consistency). Both should keep the container alive after lease.
- **Release mechanism:** dhclient uses `dhclient -r` (vs dhcpcd's `dhcpcd -k`). The entrypoint's cleanup dispatches correctly. Verify release actually occurs via address-state or server logs.
- **Timeout mechanism:** dhclient reads `timeout N;` from config file (generated by entrypoint). dhcpcd uses `-t` CLI flag. Both honor `DHCP_TIMEOUT`.

## 9. Not In Scope

- **Ubuntu image** — separate issue, will also use the shared entrypoint
- **Unrelated entrypoint changes** — only dhclient-specific fixes (DHCP_TIMEOUT, lifecycle consistency) are in scope. Other entrypoint changes require a separate issue.
- **CI pipeline to publish the image** — separate issue
- **Collector client binary (`bng-client`)** — separate issue
- **curl in the Alpine image** — the issue requests curl for Debian; adding it to Alpine would be a separate enhancement
