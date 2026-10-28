# Uniform-construction checkpoint

The real tree/cache initializer now executes 76 control visits instead of
12338685. Its full zeroed buffers agree for original and decoded artifacts. The
six-position collector still rejects the unchanged work limit; there is no
complete Mech compressor or compression time.

## Revisions and scope

- Starting compiler: `fd790ca0fce74b8eb8563cc4442d3a0041672c70`.
- Preserved optimized baseline: `6eac350f3209d8ddcec2217dcf3ba20d0279c27f`.
- Separate linked-test/empty-catalog repair: `c7096ff33984d80d3ea5a0a57bb8440a0ed7f1ea`.
- Constructor and its qualification: `f0d209a6f482266790cdb14dd8c0a0662c33beb1`.
- Branch: `codex/native-uniform-word-construction`.
- Pinned algorithm/source: `671077457ff7ef3a3d44caf715a3fc595addf379`.
- Native typed IR 107, program ABI 35, work policy 82; artifact revision 14.

The [design](../../uniform-words/README.md) describes the exact same-plan proof,
typed construction, resource admission, ordinary fallback and image restoration.
The [provenance record](provenance.json) hashes the compiler changes, test binary
and unchanged compressor source fixtures. The tree, cache, search limits,
allocation sites, source assertions and timing boundaries are unchanged. Search
and skip remain separate; skip has no cache parameter or recording path.

## Validation

All commands use `nightly-2026-03-03`, locked offline dependencies and one Cargo
build job. The main profile is `mech-gpu`, no defaults,
`canonical-native-program`. Generated linked consumers use only
`canonical-native-program-runtime`; the harness checks their actual Rust build
closure for absence of the parser and code generator. Their final build uses
`RUSTFLAGS=-D warnings`, and their auxiliary driver also denies warnings.

| Receipt | Result |
| --- | --- |
| [Broad component qualification](uniform-component-and-linked-qualification.json) | 119 existing + 5 new component tests passed; new linked test failed in the inherited helper before execution |
| [Final focused and linked qualification](uniform-linked-and-focused-final.json) | 8 passed: the same 5 constructor tests and 3 actual linked consumers; 0 failed/ignored |
| [Committed real-buffer witness](uniform-final-real-buffer-witness.json) | Initializer passed for original/decoded; six-position integration test still failed |
| [Architecture and static contracts](uniform-static-contracts.json) | 12 checks passed, including formatting, warning policy, module layout, value/artifact/type-memory boundaries, routing, quarantine and unsafe audit |

The broad run took 486 seconds to build and 291.28 seconds to execute. Its
failure was then reproduced with the old scalar assertion test. The corrected
final run took 26.44 seconds to build and 98.16 seconds to execute all eight
tests, with three independent linked builds in parallel. It includes an explicit
fact-selection check on both artifact forms before the constructor-bearing
Object is emitted. Individual commands, profiles, counts and build output are
in the linked JSON/log receipts; `check-static.py` contains the static commands.

The new tests cover four word widths, zero and nonzero words, changing words
and counts, repeated calls, resident/native agreement on small cases, effectful
body rejection, range overflow before output rejection, copy/output failure
precedence, prior aliases, cleanup and recovery. The linked sequence runs valid,
assertion-invalid and valid inputs against the same Object. It checks assertion
failure before division, unchanged publication/aliases and final cleanup.

## Measured evidence

The committed-head witness ran with the **unoptimized Rust test host** and
Cranelift `speed`. Initialization plus returning both full buffers took
90.625 ms for original and 89.635 ms for decoded artifacts. They requested
44680/44679 allocations and 43885729/43885665 cumulative bytes. Both used
1298546729 work units and 76 visits. Source/JIT compilation and whole-buffer
checking are outside those measurements.

The same six-position witness rejected after **13.315 seconds of execution**,
requesting 661549 allocations and 412755498 cumulative bytes. These counts are
neither peak nor live memory. Retained managed memory returned to its original
19379 bytes, inputs retained their aliases and no output was published.
Completed prefix positions remain unknown. The decoded six-position case was
not reached after the original case failed.

The first precise denial is `sequence/length`, origin node 0, block 36, local 11,
with a 966656-byte binding-metadata allowance:

```text
Kernel(ResourceLimit {
  phase: Admission, dimension: ComputeWork,
  required: 10005126192, limit: 10000000000
})
```

The real 262144-entry U16 tree, 1500513-word U32 cache, depth 300, nice length
258, 256 MiB managed allowance, 128 MiB temporary allowance and all work caps
remain unchanged. The integration test and its complete expected arrays were
not weakened to accept this failure.

The earlier optimized, pre-commit constructor probes took 16.825/15.842 ms
([receipt](uniform-real-buffer-qualification.json)). They precede the final
proof-function extraction and linked-harness repair. A balanced optimized rerun
of the committed head has **not** run. Do not mix those samples with the debug
host observations or present them as a full compression result. The original
[optimized baseline](../2026-10-26-fd790ca-reality-check/REPORT.md) remains intact.
Its exact executable was sampled for the included initialization profile.

## Failure disposition and remaining work

- Introduced compile errors in the initial candidate (`CodePlan` qualification,
  budget-error type and test error variant): corrected; subsequent checks/tests
  passed. Their failed command receipts are retained.
- Introduced repeated-call oracle mistake: the artifact selected only the final
  source output. The test now explicitly returns both arrays in a record and
  verifies both, including fresh arguments.
- Inherited linked helper mismatch: the shell delegates to the maintained
  launcher, but the helper searched for its former inline invocation. The
  [parent evidence](linked-harness-parent-evidence.json) and old-test failure are
  retained. Commit `c7096ff33` uses the public same-image binding API, keeps all
  build pins, and passes all three linked cases.
- Empty generated catalog's unnecessary mutable builder: exposed by denying
  warnings in the restored driver. The same commit generates an immutable
  builder when no factory installation exists; no warning policy was relaxed.
- Six-position work denial: the unresolved integration blocker already existed
  at the baseline. Removing initialization overhead exposes the later length
  admission above. The current original/decoded zero-position state agrees;
  no complete six-position result or compressor result is claimed.
- Narrow checked addition `word + 1u32` produced `MissingSemanticWitness` while
  writing a test. Its source is preserved in
  `repeated-call-missing-witness.mec`; it is not fixed or classified against a
  clean executable parent here. The real subsequent requirement is the U32
  end-of-block frequency increment at `CompressOptimal.roc:566–567`. Minimize
  that operation and establish its acceptance test before expanding compiler
  scope. The constructor test instead supplies independently changing words.

No GitHub CI was triggered or awaited. The six independent feature-profile
checks and 21 native-owner unit tests from the previous checkpoint were not
rerun here; actual runtime-only linked consumers were built and executed.
No release timing of this committed head, full-file compression, independent
decompression or reymont run is available.

Permanent compressor functions are unchanged: matchfinder helpers, separate
binary-tree search/skip and the consecutive-position prefix collector execute.
The next bounded step is to identify the real argument/binding/ownership cost
behind the precise length-call denial, add its minimal witness and repeat this
same collector test without increasing allowances. Full observations/block
decisions/carryover, retained-capacity U64/U8 buffers, optimal parsing, block
writing, full Huffman integration and `compress_into` remain to be connected.
