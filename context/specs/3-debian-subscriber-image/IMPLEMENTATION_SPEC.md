# Implementation Spec: Debian Subscriber Image

## 1. Overview

Debian 12 (Bookworm) subscriber container image using isc-dhcp-client (dhclient) for DHCPv4/DHCPv6 and ppp for PPPoE. Reuses the shared entrypoint from issue #1 with zero modifications. This validates that the entrypoint's DHCP client auto-detection works correctly with a different DHCP implementation than Alpine's dhcpcd.

## 2. Source Issue

[#3 — Debian subscriber image](https://github.com/veesix-networks/bngtester/issues/3)

## 3. Current State

The shared entrypoint (`images/shared/entrypoint.sh`) and Alpine image (`images/alpine/Dockerfile`) are complete from issue #1. The entrypoint already supports dhclient via `command -v` auto-detection — the Debian image only needs to install the right packages. No `images/debian/` directory exists yet.

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
- **curl is explicitly requested** in the issue's acceptance criteria. Alpine's image does not include curl — Debian adds it.
- **ppp-pppoe is not a separate package on Debian.** The `ppp` package includes the PPPoE plugin (`/usr/lib/pppd/*/pppoe.so`).

### Key Design Decisions

**No entrypoint changes.** The shared entrypoint already has full dhclient support built in. The auto-detection path (`command -v dhclient`) and all dispatch functions (`start_dhcpv4`, `start_dhcpv6`, cleanup release via `dhclient -r`) are already implemented and tested in issue #1's design. This image is purely a Dockerfile.

**Debian 12 (Bookworm).** Current stable release. Using `debian:bookworm-slim` as the base to minimize image size while keeping a full apt ecosystem.

**Single RUN layer for package installation.** All packages installed in one `apt-get install` command with `--no-install-recommends` and cache cleanup in the same layer to keep the image small.

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

One file. The shared entrypoint is not modified.

## 7. Implementation Order

### Phase A: Debian Dockerfile

Create `images/debian/Dockerfile`:

1. `FROM debian:bookworm-slim`
2. `RUN apt-get update && apt-get install -y --no-install-recommends isc-dhcp-client ppp iputils-ping iproute2 iperf3 curl && rm -rf /var/lib/apt/lists/*`
3. `COPY shared/entrypoint.sh /entrypoint.sh`
4. `RUN chmod +x /entrypoint.sh`
5. `ENTRYPOINT ["/entrypoint.sh"]`

SPDX copyright header at the top.

**Testable independently:** `docker build -f images/debian/Dockerfile images/` must succeed. Image contains all expected binaries (`dhclient`, `pppd`, `ping`, `ip`, `iperf3`, `curl`).

## 8. Testing

### Build Validation

- `docker build -f images/debian/Dockerfile images/` succeeds
- Image contains expected binaries: `dhclient`, `pppd`, `ping`, `ip`, `iperf3`, `curl`
- Entrypoint is executable and set correctly
- SPDX copyright header present

### DHCP Client Detection

- Container starts and the entrypoint detects `dhclient` (not `dhcpcd`)
- Log output shows `bngtester: Detected DHCP client: dhclient`

### Functional Tests (require NET_ADMIN capability)

These tests are the same as issue #1, validating the entrypoint works correctly with the Debian package set:

| Test | What It Validates |
|------|-------------------|
| Untagged + DHCPv4 | dhclient obtains lease via default path |
| Single-tagged VLAN + DHCPv4 | VLAN sub-interface created, dhclient obtains lease through it |
| QinQ + DHCPv4 | S-VLAN + C-VLAN created, dhclient obtains lease through QinQ |
| Missing CVLAN with `ENCAP=single` | Entrypoint exits with error |
| PPPoE launch | pppd starts with correct flags |
| SIGTERM cleanup | VLAN interfaces removed, dhclient lease released on stop |

### Behavioral Differences to Watch

dhclient behaves differently from dhcpcd in several ways that the entrypoint already accounts for:

- **Lease file location:** dhclient writes to `/var/lib/dhcp/dhclient.leases` (vs dhcpcd's `/var/lib/dhcpcd/`). Not an entrypoint concern — both clients manage their own lease files.
- **Foreground mode:** dhclient uses `-d` for foreground mode (vs dhcpcd's `-B` for background-suppress). The entrypoint already passes the correct flags per client.
- **Release mechanism:** dhclient uses `dhclient -r` (vs dhcpcd's `dhcpcd -k`). The entrypoint's cleanup function already dispatches correctly.

## 9. Not In Scope

- **Ubuntu image** — separate issue, will also use the shared entrypoint
- **Entrypoint modifications** — if the entrypoint needs changes for Debian, that is an amendment to issue #1
- **CI pipeline to publish the image** — separate issue
- **Collector client binary (`bng-client`)** — separate issue
- **curl in the Alpine image** — the issue requests curl for Debian; adding it to Alpine would be a separate enhancement
