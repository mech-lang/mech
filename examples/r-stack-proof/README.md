# R1-R6 trust proof

This is the replacement for the withdrawn EKF demo. It makes a narrower claim
and proves it without a bespoke algorithm operation.

The complete algorithm is five lines of ordinary Mech in
[`trust-program.mec`](trust-program.mec):

```mech
~x := 1.0
next := x * 2.0 + 3.0
safe! := next < 1000.0
x = next
x
```

The compiled artifact must contain exactly `math/mul`, `math/add`,
`compare/lt`, and `core/assign`. The executable exits non-zero if an `ekf/*`
operation or any other hidden operation appears. Those four primitives are
native standard-library implementations; the recurrence is not implemented in
Rust.

Run it from the repository root:

```sh
./scripts/demo-r-stack.sh
```

The walkthrough prints and checks:

1. the complete Mech source and real parse structure;
2. compiler type-binding and per-instruction memory-planning evidence;
3. the immutable artifact, bytecode round trip, and exact operation allowlist;
4. the activated resident schedule, slots, integrity mode, and memory layout;
5. seven successful turns against the literal oracle
   `5, 13, 29, 61, 125, 253, 509`;
6. identical values and receipts from the source artifact and decoded bytecode;
7. rejection of the next candidate, `1021`, with epoch, hash, state, and output
   all unchanged;
8. syntax, type, missing-operation, truncated-bytecode, tampered-artifact, and
   one-byte-budget rejection paths.

Each protection is checked in-process. A false claim makes the demo exit with a
non-zero status instead of printing `ALL CLAIMED CHECKS PASSED`.

This proves the R-stack path and its protections. It does not claim an
independent implementation of EKF mathematics, and it explicitly discloses the
native primitive boundary.

## Inspect the complete internal structures

The concise views are backed by the full Rust structures. Dump any one of them:

```sh
./scripts/demo-r-stack.sh --dump parse
./scripts/demo-r-stack.sh --dump artifact
./scripts/demo-r-stack.sh --dump activation
./scripts/demo-r-stack.sh --dump memory
./scripts/demo-r-stack.sh --dump all
```

The memory dump uses `ProgramMemoryPlan::diagnostic_text`, the stable,
pointer-free representation used by the planner's audits and determinism
checks.
