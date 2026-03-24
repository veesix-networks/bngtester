# Spec Review: ECN Marking and Detection (#33)

Review of the implementation specification for adding ECN support to `bngtester`.

## Findings

### CRITICAL
None.

### HIGH

#### 1. Safe Async `recvmsg` Integration
The spec suggests replacing `recv_from` with a `recvmsg` wrapper but doesn't detail how this integrates with the Tokio reactor. 
- **Impact:** Calling a blocking `libc::recvmsg` directly inside a Tokio task can stall the executor.
- **Recommendation:** Use `udp_socket.async_io(Interest::READABLE, || { ... })` or `udp_socket.readable().await` followed by `udp_socket.try_io(...)`. This ensures the raw `libc` call is only made when the socket is actually ready and doesn't block other tasks.

#### 2. `IP_TOS` cmsg Data Type
On Linux, the `IP_TOS` ancillary data returned by `recvmsg` is typically an `int`, not a single `u8`.
- **Impact:** Reading only 1 byte from the cmsg data might lead to alignment issues or incorrect values depending on endianness if the kernel wrote an `int`.
- **Recommendation:** The `recvmsg` parser should treat the `IP_TOS` cmsg data as `libc::c_int` and then cast it to `u8` to extract the TOS byte.

### MEDIUM

#### 1. ECN Stripping Detection
The spec currently focuses on counting `CE` marks. However, a common BNG failure mode is stripping ECN bits entirely (re-marking `ECT(0/1)` to `Not-ECT`).
- **Impact:** Users won't know if the BNG is ECN-transparent or if it's stripping ECN.
- **Recommendation:** Track and report all four ECN states in the receiver results: `ecn_not_ect`, `ecn_ect0`, `ecn_ect1`, and `ecn_ce`. This allows the test runner to verify that `ECT` packets actually arrived as `ECT` (or `CE`).

#### 2. TOS Byte Verification (DSCP preservation)
Since `recvmsg` provides the full TOS byte, we have the opportunity to verify the DSCP bits as well.
- **Impact:** BNGs might incorrectly re-mark DSCP while handling ECN.
- **Recommendation:** The receiver should ideally report the received DSCP bits alongside the ECN bits to provide full visibility into TOS byte mutations.

### LOW

#### 1. `build_tos` Refactoring
The spec proposes a new `build_tos` function. 
- **Recommendation:** This should replace the existing `dscp_to_tos` in `src/dscp.rs`. All callers (UDP/TCP generators) should be updated to pass both DSCP and ECN modes.

#### 2. Mutually Exclusive CLI Flags
The spec mentions `--ecn` and `--ecn-ect1` are mutually exclusive.
- **Recommendation:** Use a `clap` argument group or a single `--ecn <MODE>` flag (e.g. `--ecn ect0`, `--ecn ect1`) to enforce this at the CLI level.

## Summary of Suggestions
1. **Tokio-safe `recvmsg`:** Use `udp_socket.async_io` for the receiver loop.
2. **Handle `int` in cmsg:** Parse `IP_TOS` cmsg as `int`.
3. **Full ECN tracking:** Record all 4 ECN states, not just CE.
4. **DSCP verification:** Report received DSCP bits since they are available in the same byte.
