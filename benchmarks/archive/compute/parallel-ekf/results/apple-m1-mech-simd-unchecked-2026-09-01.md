# Mech SIMD-JIT checked and unchecked rerun

Apple M1 arm64, 10,000 independent EKFs, 20 resident turns, one worker,
seven fresh samples after warm-up. Compilation, input construction, startup,
warm-up, and final readback are outside the timed region.

| Mech path | Contract | Median M EKF-turns/s |
| --- | --- | ---: |
| Cranelift SIMD-JIT | checked | 33.327 |
| Cranelift SIMD-JIT | checked-fast | 38.655 |
| Cranelift SIMD-JIT | unchecked | 40.234 |
| Cranelift SIMD-JIT | unchecked-fast | 42.497 |

The ordinary unchecked row uses the same compiled source artifact with
integrity constraints removed before JIT preparation. The unchecked-fast row
also permits the limited algebraic zero/one-term simplifications already used
by the unchecked WGSL generator. It is retained as an opt-in numeric policy;
the checked headline remains the strict row.

The fast policy is not a blanket `-ffast-math` switch. It only removes terms
whose operands are compile-time zero or one, so it must not be described as
preserving all IEEE exceptional-value behavior. Futhark's bounded control is
similar in one important respect: its scalarized source removes the known
zero matrix terms and its compiler vectorizes the resulting map. It does not
use Mech's `checked-fast` API name.
