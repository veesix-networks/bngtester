# Spec Critique: 27-containerlab-topology (Codex)

This critique focuses on end-to-end deployability, failure handling, routing validation, and whether the file plan is concrete enough for `clab deploy` to produce a working IPoE lab. I found no `CRITICAL` issues, but there are several `HIGH` gaps that should be resolved before implementation starts.

## HIGH

### 1. OSPF is not actually part of the acceptance path, so the 3-node design does not validate the routing goal it claims
- **Evidence:** The design says OSPF between `bng1` and `server` provides bidirectional routing (`IMPLEMENTATION_SPEC.md:77`, `IMPLEMENTATION_SPEC.md:94-97`), but the server also has a static route to `10.255.0.0/16` (`IMPLEMENTATION_SPEC.md:95`). The smoke test only pings `10.0.0.2` (`IMPLEMENTATION_SPEC.md:103-108`, `IMPLEMENTATION_SPEC.md:210-211`), which is the server's directly connected core-link address (`IMPLEMENTATION_SPEC.md:55`).
- **Impact:** The lab can pass even if OSPF never converges. `bng1` does not need OSPF to reach `10.0.0.2`, and the server can still return traffic to the subscriber pool via the static route. That means the third node is not meaningfully exercising dynamic routing, despite dynamic routing being a stated design goal.
- **Recommendation:** Make OSPF observable and required. Good options are: validate adjacency state plus a route to the server loopback `10.254.0.2`, remove the backup static route from the success path, or add a real far-side subnet/host behind the server so forwarding depends on OSPF rather than a connected address.

### 2. The `osvbng.yaml` contract is too underspecified to guarantee that VPP can actually terminate subscribers
- **Evidence:** The spec reduces the osvbng config to high-level bullets (`IMPLEMENTATION_SPEC.md:79-88`) and a single file in the plan (`IMPLEMENTATION_SPEC.md:160`), but the rest of the document assumes much more: VLAN-matched IPoE subscriber groups (`IMPLEMENTATION_SPEC.md:64-66`, `IMPLEMENTATION_SPEC.md:74-75`), a DHCP pool and gateway at `10.255.0.1` (`IMPLEMENTATION_SPEC.md:58`, `IMPLEMENTATION_SPEC.md:75-76`), local auth with `allow_all` (`IMPLEMENTATION_SPEC.md:85`), OSPF on the core side (`IMPLEMENTATION_SPEC.md:86`), and a working subscriber-sessions API on `localhost:8080` (`IMPLEMENTATION_SPEC.md:212`).
- **Impact:** An implementer can produce a syntactically valid `osvbng.yaml` that still boots without a working subscriber path because required sections are not enumerated. The spec currently leaves too much unstated around interface binding, subscriber-group matching, DHCP parameters, routing context, and API enablement.
- **Recommendation:** Expand the spec to name the required configuration sections from the source template explicitly. At minimum, define the expected access/core interface mapping, subscriber group/VLAN match, IPoE enablement, DHCP pool/router settings, loopback/router-id, OSPF area/interface settings, local auth behavior, and whether the HTTP API used in the manual check must be enabled/configured by this issue.

### 3. Startup and failure behavior is not specified tightly enough for a reliable smoke test or future automation
- **Evidence:** The smoke test says only "wait for osvbng healthy" using a log marker and "retry loop with timeout" (`IMPLEMENTATION_SPEC.md:103-110`), and Phase B only requires exit 0/non-zero (`IMPLEMENTATION_SPEC.md:189-191`). There are no concrete deadlines or failure outputs for the cases the issue explicitly depends on: slow osvbng startup, DHCP timeout, or OSPF non-convergence. Meanwhile, the subscriber entrypoint already has concrete timeout behavior for interface readiness and DHCP (`images/shared/entrypoint.sh:16-17`, `images/shared/entrypoint.sh:126-153`, `images/shared/entrypoint.sh:213-234`).
- **Impact:** Phase 5 will be forced to invent timeout values and diagnostics ad hoc. On slow hosts the smoke test may fail too early; on broken labs it may wait too long or report only a generic failure. That makes the topology harder to operate manually and much harder to reuse from Robot later.
- **Recommendation:** Define concrete stage deadlines and required diagnostics. For example: fail if osvbng is not ready within `N` seconds and print `docker logs`; fail if the subscriber exits or lacks a lease after `N` seconds and dump interface/link state plus subscriber logs; fail if OSPF is not established by `N` seconds and dump neighbor/route state from both `bng1` and `server`.

## MEDIUM

### 4. The file plan does not provision the far-side service that the architecture advertises
- **Evidence:** The server is described as an `iperf3` and ping endpoint (`IMPLEMENTATION_SPEC.md:33-34`, `IMPLEMENTATION_SPEC.md:49`), and the smoke test includes an optional throughput check (`IMPLEMENTATION_SPEC.md:108`). But the plan uses the stock `frrouting/frr:v8.4.1` image (`IMPLEMENTATION_SPEC.md:49`) and only adds `daemons`, `frr.conf`, and `entrypoint.sh` under `lab/config/server/` (`IMPLEMENTATION_SPEC.md:161-165`).
- **Impact:** There is no repo-controlled mechanism to ensure `iperf3` exists on the server node. Even if the throughput step stays optional, the architecture is promising a capability that the file plan does not actually create.
- **Recommendation:** Either add a custom server image or explicit package-install/bootstrap step for `iperf3`, or remove `iperf3` from this issue's role/acceptance and keep the lab focused on DHCP, routing, and reachability.

### 5. The "Current State" section is out of sync with the checked-out branch about the Rust collector
- **Evidence:** The spec says issue `#5` already defines `bngtester-server` and `bngtester-client` binaries and that the server is a stub (`IMPLEMENTATION_SPEC.md:15`). In the current branch, I could not verify that claim from the repo itself: there is no tracked `Cargo.toml` or `src/` tree, only untracked `Cargo.lock` and `target/`.
- **Impact:** This weakens the spec's grounding in current repo state and sends future reviewers toward local files that are not actually present on the branch they were told to inspect.
- **Recommendation:** Reword this section to say the Rust collector is planned in `#5` unless the Rust project is added to this branch, or cite the exact external branch/source that contains those binaries if that is what the Phase 1 draft relied on.
