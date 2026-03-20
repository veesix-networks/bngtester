#!/bin/bash
# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Smoke test for the bngtester containerlab topology.
# Run after: clab deploy -t lab/bngtester.clab.yml
# Usage: ./lab/smoke-test.sh [lab-name]

set -euo pipefail

LAB_NAME="${1:-bngtester}"
BNG="clab-${LAB_NAME}-bng1"
SERVER="clab-${LAB_NAME}-server"
SUBSCRIBER="clab-${LAB_NAME}-subscriber"
QINQ_IFACE="eth1.100.10"

PASS=0
FAIL=0

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

# --- Stage 1: osvbng healthy ---
echo "[1/7] Waiting for osvbng to start..."
OK=0
for i in $(seq 1 12); do
    if docker logs "$BNG" 2>&1 | grep -q "osvbng started successfully"; then
        OK=1
        break
    fi
    sleep 10
done
if [ "$OK" -eq 1 ]; then
    pass "osvbng started successfully"
else
    fail "osvbng did not start within 120s"
    echo "--- bng1 logs ---"
    docker logs "$BNG" 2>&1 | tail -50
    echo "---"
    echo ""
    echo "Results: $PASS passed, $FAIL failed"
    exit 1
fi

# --- Stage 2: QinQ interface exists ---
echo "[2/7] Checking subscriber QinQ interface..."
OK=0
for i in $(seq 1 12); do
    if docker exec "$SUBSCRIBER" ip link show "$QINQ_IFACE" > /dev/null 2>&1; then
        OK=1
        break
    fi
    sleep 5
done
if [ "$OK" -eq 1 ]; then
    pass "QinQ interface $QINQ_IFACE exists"
else
    fail "QinQ interface $QINQ_IFACE not found within 60s"
    echo "--- subscriber ip link ---"
    docker exec "$SUBSCRIBER" ip link 2>&1 || true
    echo "--- subscriber logs ---"
    docker logs "$SUBSCRIBER" 2>&1 | tail -30
    echo "---"
    echo ""
    echo "Results: $PASS passed, $FAIL failed"
    exit 1
fi

# --- Stage 3: Subscriber has IPv4 ---
echo "[3/7] Waiting for subscriber DHCP lease..."
OK=0
for i in $(seq 1 18); do
    ADDR=$(docker exec "$SUBSCRIBER" ip -4 addr show "$QINQ_IFACE" 2>/dev/null || true)
    if echo "$ADDR" | grep -q "inet " && ! echo "$ADDR" | grep -q "169.254"; then
        OK=1
        break
    fi
    sleep 5
done
if [ "$OK" -eq 1 ]; then
    pass "Subscriber has IPv4 address on $QINQ_IFACE"
else
    fail "Subscriber did not get DHCP lease within 90s"
    echo "--- subscriber ip addr ---"
    docker exec "$SUBSCRIBER" ip addr 2>&1 || true
    echo "--- subscriber ip route ---"
    docker exec "$SUBSCRIBER" ip route 2>&1 || true
    echo "--- subscriber logs ---"
    docker logs "$SUBSCRIBER" 2>&1 | tail -30
    echo "---"
    echo ""
    echo "Results: $PASS passed, $FAIL failed"
    exit 1
fi

# --- Stage 4: OSPF adjacency ---
echo "[4/7] Checking OSPF adjacency..."
OK=0
for i in $(seq 1 12); do
    OSPF=$(docker exec "$SERVER" vtysh -c "show ip ospf neighbor" 2>/dev/null || true)
    if echo "$OSPF" | grep -q "Full"; then
        OK=1
        break
    fi
    sleep 5
done
if [ "$OK" -eq 1 ]; then
    pass "OSPF adjacency established (Full)"
else
    fail "OSPF adjacency not established within 60s"
    echo "--- server ospf neighbor ---"
    docker exec "$SERVER" vtysh -c "show ip ospf neighbor" 2>&1 || true
    echo "--- server ip route ---"
    docker exec "$SERVER" ip route 2>&1 || true
    echo "---"
    echo ""
    echo "Results: $PASS passed, $FAIL failed"
    exit 1
fi

# --- Stage 5: Ping gateway ---
echo "[5/7] Pinging gateway from subscriber..."
if docker exec "$SUBSCRIBER" ping -c 3 -W 2 10.255.0.1 > /dev/null 2>&1; then
    pass "Subscriber can ping gateway 10.255.0.1"
else
    fail "Subscriber cannot ping gateway 10.255.0.1"
    echo "--- subscriber routes ---"
    docker exec "$SUBSCRIBER" ip route 2>&1 || true
    echo "---"
    echo ""
    echo "Results: $PASS passed, $FAIL failed"
    exit 1
fi

# --- Stage 6: Ping server through BNG ---
echo "[6/7] Pinging server through BNG..."
OK=0
for i in $(seq 1 6); do
    if docker exec "$SUBSCRIBER" ping -c 3 -W 2 10.0.0.2 > /dev/null 2>&1; then
        OK=1
        break
    fi
    sleep 5
done
if [ "$OK" -eq 1 ]; then
    pass "Subscriber can ping server 10.0.0.2 through BNG"
else
    fail "Subscriber cannot reach server 10.0.0.2 within 30s"
    echo "--- subscriber routes ---"
    docker exec "$SUBSCRIBER" ip route 2>&1 || true
    echo "--- server routes ---"
    docker exec "$SERVER" ip route 2>&1 || true
    echo "---"
    echo ""
    echo "Results: $PASS passed, $FAIL failed"
    exit 1
fi

# --- Stage 7: iperf3 (informational) ---
echo "[7/7] Running iperf3 throughput test (informational)..."
if docker exec "$SUBSCRIBER" iperf3 -c 10.0.0.2 -t 5 2>&1; then
    pass "iperf3 throughput test completed"
else
    echo "  SKIP: iperf3 test failed (non-fatal)"
fi

echo ""
echo "Results: $PASS passed, $FAIL failed"
exit 0
