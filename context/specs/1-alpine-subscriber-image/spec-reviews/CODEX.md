# Spec Critique: 1-alpine-subscriber-image (Codex)

This critique focuses on failure modes, lifecycle behavior, interface edge cases, and downstream/CI contracts. I found no `CRITICAL` issues, but there are several `HIGH` gaps that should be resolved before implementation starts.

## HIGH

### 1. The shared-entrypoint goal conflicts with the current Alpine-only contract
- **Evidence:** `IMPLEMENTATION_SPEC.md:5`, `IMPLEMENTATION_SPEC.md:22`, `IMPLEMENTATION_SPEC.md:27`, and `IMPLEMENTATION_SPEC.md:202` say this issue establishes a shared entrypoint that Debian and Ubuntu will reuse. But `IMPLEMENTATION_SPEC.md:55-56` assumes `dhcpcd` today and future dispatch logic later, while `IMPLEMENTATION_SPEC.md:206` explicitly pushes DHCP client abstraction out of scope.
- **Impact:** The first implementation has no defined cross-image contract for DHCP launch, lease release, PPPoE plugin handling, or image-specific defaults. That means the "shared" script will either hardcode Alpine behavior or future images will need to fork it, which defeats the architectural goal this issue is supposed to establish.
- **Recommendation:** Either define the shared abstraction now (for example, per-image commands/defaults for DHCPv4, DHCPv6, PPPoE, and lease release behavior) or narrow this issue to an Alpine-specific entrypoint and move the shared-script contract to a later spec.

### 2. Access-acquisition failure semantics are unspecified, so container health is unobservable
- **Evidence:** The flow only says `dhcpcd -4 -B`, `dhcpcd -6 -B`, or foreground `pppd`, then "wait / hold container open" with cleanup on signal (`IMPLEMENTATION_SPEC.md:41-46`, `IMPLEMENTATION_SPEC.md:57-69`, `IMPLEMENTATION_SPEC.md:149-151`). The tests only check that the container starts and the client launches (`IMPLEMENTATION_SPEC.md:185-193`). There is no defined lease/session deadline, retry policy, or exit contract.
- **Impact:** The spec does not say what should happen when DHCP never gets a lease, when `dhcpcd -B` daemonizes and keeps retrying, or when PPP authentication/LCP fails. For DHCP specifically, daemonizing the client also breaks the stated "hold container open until child exits" model unless PID tracking is part of the contract. CI and orchestration will not be able to distinguish "subscriber is up" from "subscriber is stuck waiting" or "subscriber failed and should be restarted."
- **Recommendation:** Define a readiness and failure contract per access method: what counts as success, how long acquisition may take, whether failures exit non-zero or retry forever, whether PPPoE uses persistence, and how the entrypoint monitors a daemonized DHCP client.

### 3. The runtime network-attachment model is missing, which blocks reproducible implementation and CI
- **Evidence:** The spec says only that the container needs `NET_ADMIN`/`NET_RAW` and defaults to `PHYSICAL_IFACE=eth0` (`IMPLEMENTATION_SPEC.md:59`, `IMPLEMENTATION_SPEC.md:100`). The examples use a plain `docker run` with no network topology setup (`IMPLEMENTATION_SPEC.md:112-128`), and Phase B says a basic `docker run` with no extra network config should work (`IMPLEMENTATION_SPEC.md:165`).
- **Impact:** In a normal Docker bridge network, `eth0` already exists and is runtime-managed. Re-running DHCP or PPPoE over it is not a stable subscriber model, and VLAN behavior on that interface is not a meaningful acceptance test. Without a declared attachment model such as `--network none` plus an injected veth/macvlan, or another explicit topology, downstream images and a future CI pipeline have no deterministic way to launch this container correctly.
- **Recommendation:** Add an explicit runtime contract for how the subscriber interface reaches the container and update examples/tests to match it. If full network topology setup is intentionally out of scope, remove the misleading "basic docker run" acceptance and replace it with a deterministic local test harness contract.

## MEDIUM

### 4. Interface readiness only checks for presence, not for usable link state
- **Evidence:** Step 2 waits by polling `/sys/class/net/$PHYSICAL_IFACE` with a timeout (`IMPLEMENTATION_SPEC.md:35`, `IMPLEMENTATION_SPEC.md:147`), then the flow immediately creates VLANs and launches DHCP/PPPoE (`IMPLEMENTATION_SPEC.md:40-44`, `IMPLEMENTATION_SPEC.md:148-149`).
- **Impact:** Interface presence does not mean carrier is up, the lower interface is usable, or the newly created VLAN interface is operational. DHCP and PPPoE startup can race against link readiness, which creates flaky behavior and makes failures hard to classify.
- **Recommendation:** Define whether the script waits for carrier/`operstate` on the physical and target interfaces, or explicitly state that the clients must absorb link flaps and how long they are allowed to retry before the container is considered failed.

### 5. Interface-name validation is incomplete, and the prescribed naming scheme breaks on legitimate long names
- **Evidence:** Derived interface names are `${PHYSICAL_IFACE}.${CVLAN}` and `${PHYSICAL_IFACE}.${SVLAN}.${CVLAN}` (`IMPLEMENTATION_SPEC.md:38`, `IMPLEMENTATION_SPEC.md:78-90`), while `PHYSICAL_IFACE` is user-controlled config with no validation requirements beyond existence (`IMPLEMENTATION_SPEC.md:100`, `IMPLEMENTATION_SPEC.md:146`).
- **Impact:** Linux interface names are limited to 15 bytes. Valid parent names can exceed that limit once `.SVLAN.CVLAN` is appended, and unvalidated characters in `PHYSICAL_IFACE` can break `/sys/class/net/$PHYSICAL_IFACE` checks or `ip link add` invocations. The spec currently pushes those failures to runtime with no defined error handling.
- **Recommendation:** Add validation for interface-name characters and length, and either reject unsupported derived names early or define a deterministic short-name scheme for created VLAN devices.

### 6. Cleanup is specified for signals only, not for partial setup failures
- **Evidence:** The cleanup contract only mentions SIGTERM and SIGINT (`IMPLEMENTATION_SPEC.md:63-69`). The implementation plan does not require rollback if setup fails after one or more resources have already been created (`IMPLEMENTATION_SPEC.md:148-151`).
- **Impact:** Cases like "S-VLAN created but C-VLAN creation failed" or "VLAN created but DHCP/PPPoE launch failed" have no explicit unwind path. That leaves failure behavior inconsistent and can leak interfaces or helper processes in host-network or reused-netns test environments.
- **Recommendation:** Require one idempotent cleanup path that runs on every exit after resources are created, deletes interfaces in reverse order, terminates any started client processes, and is covered by negative tests for partial-failure scenarios.

### 7. VLAN-capability and kernel-support failure paths are not part of the spec
- **Evidence:** The spec assumes `ip link add` succeeds for 802.1Q and 802.1ad (`IMPLEMENTATION_SPEC.md:76-89`), and the functional test list does not include unsupported VLAN operations or missing capability/module cases (`IMPLEMENTATION_SPEC.md:183-194`).
- **Impact:** On hosts without usable 8021q/8021ad support, or where the container lacks the expected capabilities, VLAN creation will fail with low-signal `ip` errors. The spec does not say whether that should produce a specific fatal message, whether cleanup must still run, or how CI should classify the result.
- **Recommendation:** Add an explicit failure requirement for `ip link add` errors, including operator-facing diagnostics that point at missing VLAN support/capabilities, and add a negative test for unsupported VLAN creation.
