# Implementation Spec: Add Management Interface Awareness

## Overview

Add a `MGMT_IFACE` environment variable to the shared entrypoint. When set, the entrypoint deletes the default route via that interface before starting the access method, preserving the connected route for management access.

## Source Issue

[#22 — Add management interface awareness](https://github.com/veesix-networks/bngtester/issues/22)

## Current State

The shared entrypoint (`images/shared/entrypoint.sh`) has no awareness of management interfaces. When a container orchestrator (e.g., containerlab) assigns a management interface with a default route, the management default route (metric 0) wins over the DHCP-learned route (higher metric), causing subscriber traffic to exit via management instead of through the BNG.

## Design

Single new env var `MGMT_IFACE` (default: unset). When set:

1. After `configure_vlans` and before starting the access method, run `ip route del default dev "$MGMT_IFACE"`.
2. This removes only the default route — the connected route for the management subnet is preserved automatically (it's a directly connected route, not a default).
3. The DHCP-learned or PPPoE-learned default route then becomes the only default, routing subscriber traffic through the BNG.
4. No-op when unset — existing behaviour unchanged.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `MGMT_IFACE` | _(unset)_ | Management interface name (e.g., `eth0`). When set, its default route is removed before starting the access method. |

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `images/shared/entrypoint.sh` | Modify | Add `MGMT_IFACE` variable and default route removal |

## Implementation Order

1. Add `MGMT_IFACE` env var to defaults section
2. Add `remove_mgmt_default_route()` function
3. Call it in main flow after `configure_vlans`, before access method dispatch

## Testing

- [ ] `MGMT_IFACE` unset — no change in behaviour
- [ ] `MGMT_IFACE=eth0` — default route via eth0 deleted, connected route preserved
- [ ] Works with dhcpv4, dhcpv6, pppoe access methods
- [ ] Works with untagged, single, qinq encap types

## Not In Scope

- bng-client Rust binary
- API server on subscriber containers
- Dockerfile or package list changes
- IPv6 management interface handling
