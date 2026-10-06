# Native inventory execution

The inventory source compiles through `CanonicalSourceFrontend` with an explicit standard-library catalog and signed 64-bit input schemas. The resulting program artifact passes bytecode encoding and decoding. One resident instance processes an initial zero-delta turn followed by nine input pairs. Source invariants govern input quantities, nonnegative stock and a capacity of 1,000,000.

The fixture uses generic addition, subtraction, comparison, assignment and integrity constraints. The Rust harness provides scalar value transport, public prepare/publish calls, copied-output inspection and an independent integer oracle. Its oracle computes `previous accepted stock + arrivals − demand` and compares the actual accepted `I64` exactly. Every rejected turn is checked for unchanged copied output, published epoch and state hash.

| Arrivals | Demand | Candidate | Outcome | Accepted stock |
| ---: | ---: | ---: | --- | ---: |
| 20 | 15 | 105 | accepted | 105 |
| 0 | 110 | -5 | rejected by stock invariant | 105 |
| 10 | 25 | 90 | accepted | 90 |
| 0 | 90 | 0 | accepted | 0 |
| 5 | 0 | 5 | accepted | 5 |
| -1 | 0 | 4 | rejected by arrivals invariant | 5 |
| 0 | -1 | 6 | rejected by demand invariant | 5 |
| 1,000,000 | 0 | 1,000,005 | rejected by capacity invariant | 5 |
| 10 | 3 | 12 | accepted | 12 |

The last accepted epoch is 6, including the initial publication. Four rejected turns preserve the previously accepted epoch, output and state hash. Source SHA-256 is `b20d99a51835470f52fc70f462acde751f0458fb8f406ce4f732b4ada63e7a3a`. The 5,608-byte artifact has SHA-256 `cdc3374448ac921e881453cc0fa4bd02b3d37d11e32ad2fb0014ca7b2aaeed98`.

`native-result.json` contains the command, configuration, exact results, source revision patch, file hashes and artifact identities. `source-files.json` records the relevant tracked SDK/machine sources and harness files; their hashes were checked before and after the final run. `native.log` preserves the exact output. `fixture.json` specifies the shared input/output oracle. Artifact and source bytes are retained as `inventory.mecb` and `inventory.mec`.

Run from the audit checkout:

```sh
CARGO_TARGET_DIR=/private/tmp/mech-v04-target-extraction/extracted \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
cargo +nightly-2026-03-03 run --offline --locked \
  --manifest-path audit/v04/evidence/inventory/native/Cargo.toml \
  -- audit/v04/evidence/inventory/inventory.mecb
```

The command uses the recorded nightly toolchain and locally cached dependencies. The selected crate feature closure is retained in `native/Cargo.toml` and its lockfile. The unoptimized development executable is 81,053,488 bytes; this measurement includes the compiler and audit harness. Release-product size and throughput remain unmeasured.

## Earlier candidates and limits

`inventory-interval-candidate.mec` models stock and inputs as signed integer intervals. Its arithmetic expression is rejected during semantic checking with `source-semantics/non-numeric-arithmetic-kind`; the interval fails the operation's `Number` constraint. The original diagnostic is in `../extraction/native-inventory-initial.log`. The accepted fixture expresses its constraints through ordinary signed arithmetic and four explicit source invariants.

The initial scalar comparison feature selection failed the repository's warnings-as-errors policy on seven unused comparison macros. The retained manifest enables dynamic matrix, row and column representations so that this selected comparison crate configuration compiles. The original build output is in `../extraction/native-inventory-invariant.log`. A later syntax candidate failed strict parsing; the final fixture uses explicit bracket type annotations and is retained byte-for-byte.

The native result establishes this fixture's source compilation, bytecode construction, generic resident CPU execution and transactional publication behavior for the recorded sequence. Browser execution is measured separately using the same fixture source and input oracle.
