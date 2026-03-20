#!/bin/sh
# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Shared subscriber entrypoint — works on any Linux distro.
# Auto-detects DHCP client (dhcpcd or dhclient). PPPoE uses pppd (same everywhere).
# Long-term, bng-client (Rust binary) will replace this script entirely.

set -eu

# --- Defaults ---
ACCESS_METHOD="${ACCESS_METHOD:-dhcpv4}"
ENCAP="${ENCAP:-untagged}"
PHYSICAL_IFACE="${PHYSICAL_IFACE:-eth0}"
IFACE_WAIT_TIMEOUT="${IFACE_WAIT_TIMEOUT:-30}"
DHCP_TIMEOUT="${DHCP_TIMEOUT:-60}"
SVLAN="${SVLAN:-}"
CVLAN="${CVLAN:-}"
PPPOE_USER="${PPPOE_USER:-}"
PPPOE_PASSWORD="${PPPOE_PASSWORD:-}"
PPPOE_SERVICE="${PPPOE_SERVICE:-}"
MGMT_IFACE="${MGMT_IFACE:-}"

# --- State ---
CLIENT_PID=""
TARGET_IFACE=""
DHCP_CLIENT=""

# --- Logging ---
log()   { echo "bngtester: $*"; }
error() { echo "bngtester: ERROR: $*" >&2; }
die()   { error "$@"; exit 1; }

# --- Cleanup (idempotent, registered on EXIT) ---
cleanup() {
    # Stop client process
    if [ -n "$CLIENT_PID" ] && kill -0 "$CLIENT_PID" 2>/dev/null; then
        case "$DHCP_CLIENT" in
            dhcpcd)  dhcpcd -k "$TARGET_IFACE" 2>/dev/null || true ;;
            dhclient) dhclient -r "$TARGET_IFACE" 2>/dev/null || true ;;
        esac
        kill "$CLIENT_PID" 2>/dev/null || true
        wait "$CLIENT_PID" 2>/dev/null || true
    fi
    CLIENT_PID=""

    # Remove generated dhclient config
    rm -f /tmp/dhclient-bngtester.conf

    # Remove VLAN interfaces in reverse creation order
    case "$ENCAP" in
        qinq)
            [ -d "/sys/class/net/${PHYSICAL_IFACE}.${SVLAN}.${CVLAN}" ] && \
                ip link del "${PHYSICAL_IFACE}.${SVLAN}.${CVLAN}" 2>/dev/null || true
            [ -d "/sys/class/net/${PHYSICAL_IFACE}.${SVLAN}" ] && \
                ip link del "${PHYSICAL_IFACE}.${SVLAN}" 2>/dev/null || true
            ;;
        single)
            [ -d "/sys/class/net/${PHYSICAL_IFACE}.${CVLAN}" ] && \
                ip link del "${PHYSICAL_IFACE}.${CVLAN}" 2>/dev/null || true
            ;;
    esac
}

trap cleanup EXIT
trap 'exit 143' TERM
trap 'exit 130' INT

# --- Validation ---
validate_vlan_id() {
    _vid="$1"
    _name="$2"
    case "$_vid" in
        ''|*[!0-9]*) die "$_name must be a number (got '$_vid')" ;;
    esac
    if [ "$_vid" -lt 1 ] || [ "$_vid" -gt 4094 ]; then
        die "$_name must be between 1 and 4094 (got $_vid)"
    fi
}

validate_iface_name_length() {
    case "$ENCAP" in
        single) _target="${PHYSICAL_IFACE}.${CVLAN}" ;;
        qinq)   _target="${PHYSICAL_IFACE}.${SVLAN}.${CVLAN}" ;;
        *)      return 0 ;;
    esac
    _len=$(printf '%s' "$_target" | wc -c)
    if [ "$_len" -gt 15 ]; then
        die "Derived interface name '$_target' is ${_len} bytes (max 15)"
    fi
}

validate() {
    case "$ACCESS_METHOD" in
        dhcpv4|dhcpv6|pppoe) ;;
        *) die "Invalid ACCESS_METHOD '$ACCESS_METHOD'. Must be: dhcpv4, dhcpv6, pppoe" ;;
    esac

    case "$ENCAP" in
        untagged|single|qinq) ;;
        *) die "Invalid ENCAP '$ENCAP'. Must be: untagged, single, qinq" ;;
    esac

    case "$ENCAP" in
        single|qinq)
            [ -z "$CVLAN" ] && die "CVLAN is required when ENCAP=$ENCAP"
            validate_vlan_id "$CVLAN" "CVLAN"
            ;;
    esac

    if [ "$ENCAP" = "qinq" ]; then
        [ -z "$SVLAN" ] && die "SVLAN is required when ENCAP=qinq"
        validate_vlan_id "$SVLAN" "SVLAN"
    fi

    if [ "$ACCESS_METHOD" = "pppoe" ]; then
        [ -z "$PPPOE_USER" ] && die "PPPOE_USER is required when ACCESS_METHOD=pppoe"
        [ -z "$PPPOE_PASSWORD" ] && die "PPPOE_PASSWORD is required when ACCESS_METHOD=pppoe"
    fi

    validate_iface_name_length
}

# --- Interface Wait ---
wait_for_interface() {
    log "Waiting for interface $PHYSICAL_IFACE (timeout: ${IFACE_WAIT_TIMEOUT}s)..."
    _elapsed=0
    while [ ! -d "/sys/class/net/$PHYSICAL_IFACE" ]; do
        if [ "$_elapsed" -ge "$IFACE_WAIT_TIMEOUT" ]; then
            die "Interface $PHYSICAL_IFACE did not appear within ${IFACE_WAIT_TIMEOUT}s"
        fi
        sleep 1
        _elapsed=$((_elapsed + 1))
    done

    log "Interface $PHYSICAL_IFACE found, bringing up..."
    ip link set "$PHYSICAL_IFACE" up || die "Failed to bring up $PHYSICAL_IFACE"

    log "Waiting for $PHYSICAL_IFACE link state..."
    _elapsed=0
    while true; do
        _state=$(cat "/sys/class/net/$PHYSICAL_IFACE/operstate" 2>/dev/null || echo "down")
        case "$_state" in
            up|unknown) break ;;
        esac
        if [ "$_elapsed" -ge "$IFACE_WAIT_TIMEOUT" ]; then
            die "Interface $PHYSICAL_IFACE operstate is '$_state' after ${IFACE_WAIT_TIMEOUT}s (expected: up or unknown)"
        fi
        sleep 1
        _elapsed=$((_elapsed + 1))
    done
    log "Interface $PHYSICAL_IFACE is ready (operstate: $_state)"
}

# --- VLAN Configuration ---
configure_vlans() {
    case "$ENCAP" in
        untagged)
            TARGET_IFACE="$PHYSICAL_IFACE"
            ;;
        single)
            TARGET_IFACE="${PHYSICAL_IFACE}.${CVLAN}"
            log "Creating VLAN interface $TARGET_IFACE (C-VLAN $CVLAN)..."
            ip link add link "$PHYSICAL_IFACE" name "$TARGET_IFACE" \
                type vlan id "$CVLAN" || \
                die "VLAN creation failed. Check 8021q kernel module and NET_ADMIN capability."
            ip link set "$TARGET_IFACE" up
            ;;
        qinq)
            _svlan_iface="${PHYSICAL_IFACE}.${SVLAN}"
            TARGET_IFACE="${_svlan_iface}.${CVLAN}"

            log "Creating S-VLAN interface $_svlan_iface (S-VLAN $SVLAN, 802.1ad)..."
            ip link add link "$PHYSICAL_IFACE" name "$_svlan_iface" \
                type vlan id "$SVLAN" protocol 802.1ad || \
                die "S-VLAN creation failed. Check 8021q/8021ad kernel module and NET_ADMIN capability."
            ip link set "$_svlan_iface" up

            log "Creating C-VLAN interface $TARGET_IFACE (C-VLAN $CVLAN)..."
            ip link add link "$_svlan_iface" name "$TARGET_IFACE" \
                type vlan id "$CVLAN" || \
                die "C-VLAN creation failed."
            ip link set "$TARGET_IFACE" up
            ;;
    esac
    log "Target interface: $TARGET_IFACE"
}

# --- DHCP Client Detection ---
detect_dhcp_client() {
    if command -v dhcpcd >/dev/null 2>&1; then
        DHCP_CLIENT="dhcpcd"
    elif command -v dhclient >/dev/null 2>&1; then
        DHCP_CLIENT="dhclient"
    else
        die "No DHCP client found (need dhcpcd or dhclient)"
    fi
    log "Detected DHCP client: $DHCP_CLIENT"
}

# --- dhclient config generation ---
generate_dhclient_conf() {
    if [ -f /etc/dhcp/dhclient.conf ]; then
        cp /etc/dhcp/dhclient.conf /tmp/dhclient-bngtester.conf
    else
        : > /tmp/dhclient-bngtester.conf
    fi
    printf 'timeout %s;\n' "$DHCP_TIMEOUT" >> /tmp/dhclient-bngtester.conf
}

# --- Dispatch Functions ---
start_dhcpv4() {
    log "Starting DHCPv4 on $TARGET_IFACE (timeout: ${DHCP_TIMEOUT}s)..."
    case "$DHCP_CLIENT" in
        dhcpcd)  dhcpcd -4 -B -t "$DHCP_TIMEOUT" "$TARGET_IFACE" & ;;
        dhclient)
            generate_dhclient_conf
            dhclient -4 -v -1 -d -cf /tmp/dhclient-bngtester.conf "$TARGET_IFACE" & ;;
    esac
    CLIENT_PID=$!
    wait "$CLIENT_PID"
}

start_dhcpv6() {
    log "Starting DHCPv6 on $TARGET_IFACE (timeout: ${DHCP_TIMEOUT}s)..."
    case "$DHCP_CLIENT" in
        dhcpcd)  dhcpcd -6 -B -t "$DHCP_TIMEOUT" "$TARGET_IFACE" & ;;
        dhclient)
            generate_dhclient_conf
            dhclient -6 -v -1 -d -cf /tmp/dhclient-bngtester.conf "$TARGET_IFACE" & ;;
    esac
    CLIENT_PID=$!
    wait "$CLIENT_PID"
}

start_pppoe() {
    log "Starting PPPoE on $TARGET_IFACE..."
    set -- pppd plugin pppoe.so "$TARGET_IFACE" \
        user "$PPPOE_USER" \
        password "$PPPOE_PASSWORD" \
        nodetach \
        noauth \
        defaultroute \
        usepeerdns \
        persist \
        maxfail 0
    if [ -n "$PPPOE_SERVICE" ]; then
        set -- "$@" servicename "$PPPOE_SERVICE"
    fi
    exec "$@"
}

# --- Management Interface ---
remove_mgmt_default_route() {
    if [ -z "$MGMT_IFACE" ]; then
        return
    fi
    log "Removing default route via management interface $MGMT_IFACE..."
    if ip route del default dev "$MGMT_IFACE" 2>/dev/null; then
        log "Default route via $MGMT_IFACE removed"
    else
        log "No default route via $MGMT_IFACE found (may already be absent)"
    fi
}

# --- Main ---
log "Config: ACCESS_METHOD=$ACCESS_METHOD ENCAP=$ENCAP PHYSICAL_IFACE=$PHYSICAL_IFACE"

validate
wait_for_interface
configure_vlans
remove_mgmt_default_route

case "$ACCESS_METHOD" in
    dhcpv4|dhcpv6) detect_dhcp_client ;;
esac

case "$ACCESS_METHOD" in
    dhcpv4) start_dhcpv4 ;;
    dhcpv6) start_dhcpv6 ;;
    pppoe)  start_pppoe ;;
esac
