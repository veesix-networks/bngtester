# Spec Review: DSCP/TOS Marking on Outgoing Packets (#32)

Review of the implementation specification for adding DSCP marking support to `bngtester`.

## Findings

### CRITICAL
None.

### HIGH

#### 1. TCP SYN Marking Missing in Design
The spec proposes calling `setsockopt(IP_TOS)` on the socket. For TCP, using `TcpStream::connect` (as the current code does) means the socket is created and connected in one call. Calling `setsockopt` *after* `connect` results in the SYN packet being sent with the default TOS (0).
- **Impact:** BNGs may use the SYN packet's TOS for session classification or expect all packets in a flow to have consistent marking. Missing the SYN can lead to inconsistent QoS treatment.
- **Recommendation:** Use a "create socket -> set TOS -> connect" flow. This requires using the `socket2` crate or raw `libc` to create the socket and set the option before initiating the connection.

#### 2. Server Sending DSCP in Bidirectional Modes
The spec states the server doesn't need to set DSCP for latency mode but will need it for bidirectional/RRUL modes.
- **Impact:** The `src/traffic/generator.rs` and `src/traffic/tcp.rs` modules must be updated to support DSCP marking regardless of whether they are running on the client or server. The server must correctly map the `StreamDscpConfig` from the `HelloMsg` to its local stream generators.
- **Recommendation:** Ensure the generator implementation is common and always applies the configured DSCP if provided, whether initiated by client or server.

### MEDIUM

#### 1. Verifying Received DSCP (Inbound)
The spec focuses entirely on *marking* outgoing packets. It does not mention *verifying* the DSCP of received packets.
- **Impact:** In many BNG testing scenarios, it is critical to verify that the BNG has not cleared the DSCP marking or re-marked it incorrectly (e.g., stripping EF marking to BE).
- **Recommendation:** Consider adding support for `IP_RECVTOS` (and `IPV6_RECVTCLASS` in the future) on the receiver side. This allows the server (and client in bidirectional modes) to record and report the DSCP values actually received. This could be a follow-up issue or an enhancement to the current spec.

#### 2. ECN Bits Interaction
The spec correctly notes that `IP_TOS` overwrites the entire TOS byte, including ECN bits.
- **Impact:** While ECN is out of scope (Issue #33), the current design will zero out ECN bits. 
- **Recommendation:** When Issue #33 is implemented, the `set_tos` logic should be updated to read the current TOS byte via `getsockopt`, preserve the ECN bits, and only modify the DSCP bits. For now, a comment in the code acknowledging this would be helpful.

### LOW

#### 1. `socket2` Dependency
The spec mentions using `socket2` but it is not currently in `Cargo.toml`.
- **Recommendation:** Add `socket2` to `Cargo.toml`. It provides a much cleaner and safer API than raw `libc` for the "create -> set option -> connect" pattern required for TCP SYN marking.

#### 2. DSCP Codepoint Validation
The spec doesn't explicitly define how to handle out-of-range values (>63) or invalid names.
- **Recommendation:** The CLI and parser should strictly validate that numeric values are 0-63 and names match the standard PHB list. Invalid inputs should result in an immediate error and process exit with a helpful message.

#### 3. IPv6 Future-Proofing
While IPv6 is not in scope, the internal helper for setting DSCP should be designed to eventually support `IPV6_TCLASS`.
- **Recommendation:** Name the internal helper something generic like `set_traffic_class` or `set_tos_tclass` to accommodate both protocols later.

## Summary of Suggestions
1. **Refactor TCP Connection:** Change TCP stream creation to allow setting TOS before `connect()`.
2. **Add `socket2`:** Include `socket2` in dependencies for easier socket configuration.
3. **Clarify Server Mapping:** Explicitly state how the server maps `HelloMsg` stream DSCP config to its local generators.
4. **Consider Inbound Verification:** (Optional) Add `IP_RECVTOS` support to report received DSCP.
