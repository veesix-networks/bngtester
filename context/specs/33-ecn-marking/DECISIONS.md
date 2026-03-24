# Decisions: 33-ecn-marking

## Accepted

### Tokio-safe recvmsg via readable() + try_io()
- **Source:** GEMINI (G1), CODEX (C2)
- **Severity:** HIGH
- **Resolution:** Never call blocking `libc::recvmsg` directly inside a tokio task. Use `UdpSocket::readable().await` + `try_io()` wrapping raw `recvmsg`. This preserves async cancellation — the receiver can still be cancelled via CancellationToken while waiting for readiness.

### IP_TOS cmsg data is c_int, not u8
- **Source:** GEMINI (G2)
- **Severity:** HIGH
- **Resolution:** The `recvmsg_tos()` wrapper parses the `IP_TOS` cmsg data as `libc::c_int` and casts to `u8`. Avoids alignment issues on different architectures.

### Track all 4 ECN states, not just CE
- **Source:** GEMINI (G3)
- **Severity:** MEDIUM
- **Resolution:** Receiver tracks Not-ECT, ECT(0), ECT(1), and CE counts. This detects ECN stripping (BNG re-marking ECT to Not-ECT = misconfiguration) in addition to CE marks. All four counters reported in metrics.

### Replace dscp_to_tos with build_tos
- **Source:** GEMINI (G5)
- **Severity:** LOW
- **Resolution:** Single `build_tos(dscp, ecn)` function replaces `dscp_to_tos()`. All callers updated to pass both DSCP and ECN mode.

### Single --ecn <MODE> flag instead of two flags
- **Source:** GEMINI (G6)
- **Severity:** LOW
- **Resolution:** Use `--ecn ect0` or `--ecn ect1` instead of `--ecn` and `--ecn-ect1`. Cleaner CLI, no need for mutual exclusion validation.

### IP_RECVTOS failure must fail-fast
- **Source:** CODEX (C1)
- **Severity:** HIGH
- **Resolution:** If `IP_RECVTOS` cannot be enabled on a receiver socket when ECN observation is requested, fail the receiver setup before the test starts. Missing cmsg on individual packets is counted as unknown — CE ratio excludes unknowns. Report fields omitted (not zero) when ECN is disabled or observation unavailable.

### File plan must include src/traffic/receiver.rs
- **Source:** CODEX (C3)
- **Severity:** HIGH
- **Resolution:** Added `src/traffic/receiver.rs` to file plan. Both receiver implementations (standalone and inline in server.rs) are updated with ECN support. No divergent receiver behaviors.

### Report fields omitted when ECN disabled, zero means observed-zero
- **Source:** CODEX (C4)
- **Severity:** MEDIUM
- **Resolution:** All ECN report fields use `skip_serializing_if = "Option::is_none"`. When ECN is off, fields are omitted entirely. When ECN is on, zero values mean "observed zero of that codepoint" not "not observed". This preserves backward compatibility and semantic correctness.

## Rejected

### Report received DSCP bits for re-marking detection
- **Source:** GEMINI (G4)
- **Severity:** MEDIUM
- **Rationale:** Out of scope for this issue. The `recvmsg` path gives us the full TOS byte, so received DSCP bits are technically available, but adding DSCP verification is a separate feature with its own reporting and assertion concerns. The same `recvmsg` infrastructure built here makes a follow-up trivial. Filed as a future follow-up.
