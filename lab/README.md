# bngtester Lab Topology

Containerlab topology that deploys [osvbng](https://github.com/veesix-networks/osvbng) as a VPP-based BNG with a bngtester subscriber container for end-to-end IPoE validation.

## Architecture

```
subscriber (bngtester)  ──eth1──  bng1 (osvbng/VPP)  ──eth2──  server (FRR)
   QinQ VLAN 100.10         Access link            Core link     OSPF + iperf3
   DHCPv4 client            DHCP server            10.0.0.2/30
   10.255.0.x/16            10.255.0.1 (gw)
                            10.0.0.1/30
```

## Prerequisites

- [containerlab](https://containerlab.dev/install/) (v0.44+)
- Docker
- osvbng image: `docker pull veesixnetworks/osvbng:latest` (or build locally with `veesixnetworks/osvbng:local`)
- bngtester subscriber image: `docker pull veesixnetworks/bngtester:alpine-latest`

## Deploy

```bash
# From the repo root
sudo clab deploy -t lab/bngtester.clab.yml
```

## Validate

```bash
./lab/smoke-test.sh
```

The smoke test checks (in order): osvbng startup, QinQ interface creation, DHCP lease, OSPF adjacency, gateway ping, cross-BNG ping, and iperf3 throughput. Each stage has retry loops to handle startup ordering. Exit code 0 means all checks passed.

## Destroy

```bash
sudo clab destroy -t lab/bngtester.clab.yml --cleanup
```

## Image Override

Swap the subscriber image to test different distributions:

```bash
# Debian subscriber
BNGTESTER_IMAGE=veesixnetworks/bngtester:debian-latest sudo -E clab deploy -t lab/bngtester.clab.yml

# Ubuntu subscriber
BNGTESTER_IMAGE=veesixnetworks/bngtester:ubuntu-latest sudo -E clab deploy -t lab/bngtester.clab.yml

# Local osvbng build
OSVBNG_IMAGE=veesixnetworks/osvbng:local sudo -E clab deploy -t lab/bngtester.clab.yml
```

Use `sudo -E` to pass environment variables through to containerlab.

## Manual Inspection

```bash
# Check subscriber IP
docker exec clab-bngtester-subscriber ip -4 addr show eth1.100.10

# Check BNG subscriber sessions
docker exec clab-bngtester-bng1 curl -s http://localhost:8080/api/show/subscriber/sessions

# Check OSPF on server
docker exec clab-bngtester-server vtysh -c "show ip ospf neighbor"

# Run iperf3 manually
docker exec clab-bngtester-subscriber iperf3 -c 10.0.0.2 -t 10
```

## Troubleshooting

### Subscriber has no IP address

1. Check the subscriber container logs: `docker logs clab-bngtester-subscriber`
2. Verify the QinQ interface was created: `docker exec clab-bngtester-subscriber ip link`
3. Check osvbng logs for DHCP activity: `docker logs clab-bngtester-bng1 2>&1 | grep -i dhcp`
4. Verify osvbng sees the access interface: `docker exec clab-bngtester-bng1 vppctl -s /run/osvbng/cli.sock show interface`

### OSPF not converging

1. Check FRR status on server: `docker exec clab-bngtester-server /usr/lib/frr/frrinit.sh status`
2. Verify the core link is up: `docker exec clab-bngtester-server ip addr show eth1`
3. Check OSPF neighbor state: `docker exec clab-bngtester-server vtysh -c "show ip ospf neighbor"`

### QinQ MTU issues

QinQ adds 8 bytes of VLAN overhead per frame. Containerlab veth pairs default to 1500 MTU, giving an effective subscriber MTU of 1492 bytes. If large packets are being dropped or fragmented, either increase the host MTU or configure the subscriber to use a smaller MTU:

```bash
docker exec clab-bngtester-subscriber ip link set eth1.100.10 mtu 1492
```

### osvbng fails to start

osvbng requires hugepages. On the host:

```bash
echo 512 | sudo tee /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages
```

Check that the osvbng container has the required capabilities (SYS_ADMIN, NET_ADMIN, IPC_LOCK, SYS_NICE).

## IP Addressing

| Subnet | Purpose | Addresses |
|--------|---------|-----------|
| `10.0.0.0/30` | Core link (BNG - server) | bng1=10.0.0.1, server=10.0.0.2 |
| `10.254.0.1/32` | BNG loopback (OSPF router-id) | bng1 |
| `10.254.0.2/32` | Server loopback (OSPF router-id) | server |
| `10.255.0.0/16` | Subscriber DHCP pool | gateway=10.255.0.1 |

## Related Issues

- [#27](https://github.com/veesix-networks/bngtester/issues/27) — this topology
- [#13](https://github.com/veesix-networks/bngtester/issues/13) — Robot Framework tests (will use this topology)
- [#5](https://github.com/veesix-networks/bngtester/issues/5) — Rust collector (needs this topology for end-to-end testing)
