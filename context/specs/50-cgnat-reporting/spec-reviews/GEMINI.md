# Spec Review: CGNAT-Aware Reporting (GEMINI)

## Summary

The spec provides a solid foundation for making reports CGNAT-aware by introducing "dual addressing" (peer address vs. subscriber address). This is critical for multi-subscriber testing where multiple clients may share a single translated public IP.

## Findings

### 1. Information Leakage from Mandatory Local IP Reporting
- **Severity:** MEDIUM
- **Finding:** The proposal to "always send the client's local IP ... as a fallback" in the `HelloMsg` could leak internal network topology. While this is generally desirable in a BNG testing context (where the operator owns the network), it might be a privacy concern for users testing across public or untrusted networks.
- **Recommendation:**
    - Keep the "always send" behavior as the default to ensure "zero-config" CGNAT awareness.
    - Document this behavior clearly in the README/help text.
    - Consider adding a `--hide-local-ip` flag to the client to explicitly disable this fallback if privacy is a concern.

### 2. Reliable "No CGNAT" Detection Logic
- **Severity:** LOW
- **Finding:** The spec mentions showing simplified output when `peer == subscriber`. Since `peer` is a `SocketAddr` string (e.g., `1.2.3.4:5678`) and `subscriber_ip` is an `IpAddr` string (e.g., `10.0.0.1`), a direct string comparison will always fail due to the port number in the peer address.
- **Recommendation:**
    - The server-side logic must parse the peer's `SocketAddr` and the hello's `source_ip` (as an `IpAddr`) before comparison.
    - Comparison should be: `peer.ip() == subscriber_ip`.
    - Handle IPv4-mapped IPv6 addresses consistently if the server supports dual-stack.

### 3. Placement of `subscriber_ip` in Report Structs
- **Severity:** LOW
- **Finding:** The spec proposes adding `subscriber_ip` to both `TestConfig` and `ClientReport`.
- **Recommendation:**
    - **TestConfig:** This is the correct primary home for `subscriber_ip`. It represents a core property of the test session (the "real" identity of the client).
    - **ClientReport:** Adding it here is technically redundant since `ClientReport` contains a `TestReport`, which contains `TestConfig`. However, for JSON/combined report consumers, having it at the top level of `ClientReport` alongside `client_id` and `peer` is highly beneficial for usability. 
    - **Decision:** Proceed with adding it to both as proposed, but ensure the `TestReport` constructors in `server.rs` and `client.rs` are updated to populate it correctly from the `HelloMsg`.

### 4. Edge Case: Spoofed Source IP
- **Severity:** LOW
- **Finding:** If a user provides an explicit `--source-ip` that matches their local IP, but they are behind NAT, `peer.ip()` will differ from `subscriber_ip`, correctly triggering the "dual addressing" display. If a user "spoofs" a `--source-ip` on a direct connection, it will also look like NAT.
- **Recommendation:** This is acceptable behavior. The report accurately reflects what the client *claims* is its real IP vs. what the server *sees*. No change needed, but worth noting in implementation.

## Conclusion

The spec is well-aligned with the project's goals for multi-subscriber testing. The proposed changes to `HelloMsg` and report structures are surgical and effective.
