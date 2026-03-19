# Implementation Spec: Ubuntu Subscriber Image

## 1. Overview

Ubuntu 22.04 (Jammy) subscriber container image using isc-dhcp-client (dhclient) for DHCPv4/DHCPv6 and ppp for PPPoE. Reuses the shared entrypoint from issue #1 with no modifications — all dhclient compatibility work was completed in issue #3. This is the third subscriber image and the second to exercise the entrypoint's dhclient dispatch path.

## 2. Source Issue

[#4 — Ubuntu subscriber image](https://github.com/veesix-networks/bngtester/issues/4)

## 3. Current State

The shared entrypoint (`images/shared/entrypoint.sh`) is fully functional for both dhcpcd (Alpine) and dhclient (Debian) clients. The Alpine image (`images/alpine/Dockerfile`) and Debian image (`images/debian/Dockerfile`) are complete. The entrypoint's dhclient path — including `DHCP_TIMEOUT` config file generation, foreground mode, and cleanup — was implemented and validated during issue #3. No `images/ubuntu/` directory exists yet.

## 4. Design

### Architecture

The Ubuntu image follows the identical pattern established by Alpine and Debian:

```
images/
├── shared/
│   └── entrypoint.sh        # Already exists — no changes needed
├── alpine/
│   └── Dockerfile            # Already exists — no changes
├── debian/
│   └── Dockerfile            # Already exists — no changes
└── ubuntu/
    └── Dockerfile            # NEW — this spec
```

The build context remains `images/`, allowing the Dockerfile to `COPY shared/entrypoint.sh`.

### Package Mapping

| Capability | Alpine Package | Debian Package | Ubuntu Package |
|-----------|---------------|----------------|----------------|
| DHCP client | `dhcpcd` | `isc-dhcp-client` | `isc-dhcp-client` |
| PPPoE | `ppp` + `ppp-pppoe` | `ppp` | `ppp` |
| Ping | `iputils` | `iputils-ping` | `iputils-ping` |
| IP utilities | `iproute2` | `iproute2` | `iproute2` |
| Bandwidth testing | `iperf3` | `iperf3` | `iperf3` |
| HTTP client | — | `curl` | `curl` |

Notable observations:
- **Ubuntu uses the same packages as Debian.** Both are Debian-based and use `isc-dhcp-client` (dhclient). The entrypoint's dhclient dispatch path — already validated with the Debian image — works identically.
- **Ubuntu 22.04 base image is already minimal.** Unlike Debian, Ubuntu does not offer a `-slim` variant. The standard `ubuntu:22.04` image is ~77MB and does not include unnecessary development tools.
- **`ca-certificates` is required** for curl HTTPS support, same as Debian.
- **`netbase` is required.** The Ubuntu 22.04 base image also lacks `/etc/protocols` and `/etc/services`, same as `debian:bookworm-slim`.
- **ppp includes PPPoE plugin.** Same as Debian — `pppoe.so` is bundled in the `ppp` package.

### Key Design Decisions

**No entrypoint changes.** The shared entrypoint already handles dhclient fully — config file generation for `DHCP_TIMEOUT`, foreground mode with `-d`, lease release with `-r`, and cleanup. All of this was implemented in issue #3. The Ubuntu image only needs a Dockerfile.

**Ubuntu 22.04 (Jammy).** Current LTS release with support through April 2027 (standard) / April 2032 (ESM). Using `ubuntu:22.04` as the base image. Jammy was chosen over 24.04 (Noble) because 22.04 is the most widely deployed Ubuntu LTS in production environments — matching the issue's motivation of testing against real-world subscriber distributions.

**Single RUN layer for package installation.** All packages installed in one `apt-get install` command with `--no-install-recommends` and cache cleanup in the same layer.

## 5. Configuration

No new configuration. The Ubuntu image uses the exact same environment variables as the Alpine and Debian images, documented in the [issue #1 spec](../1-alpine-subscriber-image/IMPLEMENTATION_SPEC.md#5-configuration):

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
docker build -f images/ubuntu/Dockerfile images/ -t bngtester-ubuntu
docker run --rm --cap-add=NET_ADMIN \
  --network bng-subscribers \
  -e ACCESS_METHOD=dhcpv4 \
  -e ENCAP=qinq \
  -e SVLAN=100 \
  -e CVLAN=200 \
  bngtester-ubuntu
```

**PPPoE untagged:**
```sh
docker run --rm --cap-add=NET_ADMIN --cap-add=NET_RAW \
  --network bng-subscribers \
  -e ACCESS_METHOD=pppoe \
  -e PPPOE_USER=user@isp \
  -e PPPOE_PASSWORD=secret \
  bngtester-ubuntu
```

## 6. File Plan

| File | Action | Purpose |
|------|--------|---------|
| `images/ubuntu/Dockerfile` | Create | Ubuntu 22.04 subscriber image — installs isc-dhcp-client, ppp, test tools, copies shared entrypoint |

No modifications to existing files.

## 7. Implementation Order

### Phase A: Ubuntu Dockerfile

Create `images/ubuntu/Dockerfile`:

1. SPDX copyright header
2. `FROM ubuntu:22.04`
3. `RUN apt-get update && apt-get install -y --no-install-recommends isc-dhcp-client ppp iputils-ping iproute2 iperf3 curl ca-certificates netbase && rm -rf /var/lib/apt/lists/*`
4. `COPY shared/entrypoint.sh /entrypoint.sh`
5. `RUN chmod +x /entrypoint.sh`
6. `ENTRYPOINT ["/entrypoint.sh"]`

**Testable independently:** `docker build -f images/ubuntu/Dockerfile images/` must succeed. Image contains all expected binaries (`dhclient`, `pppd`, `ping`, `ip`, `iperf3`, `curl`).

This is a single-phase implementation — no entrypoint changes or multi-step dependencies.

## 8. Testing

### Build Validation

- `docker build -f images/ubuntu/Dockerfile images/` succeeds
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
| SIGTERM cleanup + lease release | VLAN interfaces removed; lease release via `dhclient -r` |
| Container stays alive after lease | Container persists after lease acquisition for post-lease testing |

### Behavioral Parity with Debian

Since Ubuntu and Debian both use `isc-dhcp-client`, behavior should be identical to the Debian image:

- Same lease file location (`/var/lib/dhcp/dhclient.leases`)
- Same foreground mode (`-d`)
- Same release mechanism (`dhclient -r`)
- Same timeout mechanism (config file with `timeout N;`)

Any difference would indicate a packaging or version discrepancy worth investigating.

## 9. Not In Scope

- **Entrypoint changes** — the shared entrypoint is complete; any changes require a separate issue
- **CI pipeline to publish the image** — separate issue
- **Collector client binary (`bng-client`)** — separate issue
- **Ubuntu 24.04 (Noble) image** — could be a future issue if testing against newer LTS is needed
- **curl in the Alpine image** — separate enhancement
