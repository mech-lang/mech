# Pay eligible lexical cleanup together

Status: **implemented, locally qualified, measured and retained for review**.
The PR stays draft. Hosted CI is cancelled by user direction; no green-CI claim.

## Complete eight-byte result

Protected `abcdefgh` compression, pinned allocator, unchanged evaluation-only timer:

| Same-session measurement | Retained compiler | New compiler |
| --- | ---: | ---: |
| Primary median | 4.9301665 ms | 4.6873125 ms |
| Common measured reference = 1×, 0.287 ms | 17.17828049× | 16.33209930× |
| Fixed historical reference = 1×, 0.300 ms | 16.43388833× | 15.62437500× |

Elapsed reduction **4.92587826%**. All four independently prepared worker pairs
improve, by 5.096%, 5.007%, 4.378% and 4.916%. All 64 descriptive blocks improve;
the median block ratio is 0.95062197 and geometric ratio 0.95006874. Both order
strata improve: 32/32 blocks each, median ratios 0.95007505 and 0.95180535.
The blocks share four pairs; they are not 64 independent experiments.

All **3984 observations**, including **3968 primary calls**, are preserved.
The reference has 20000 observations. Each reference, worker-pair, control and
profile packet was committed immediately. No build, test, audit, preparation or
profile overlapped primary evaluation. Every observation checks the exact ten
raw bytes, independent decompression, input contents and aliases, capacity 13,
work **44443887150** in both variants and final cleanup zero. No timing packet
was interrupted, replaced or selectively omitted in this round.

Fresh reference measurements vary between sessions. These paired ratios and
the fixed historical ruler are deliberately separate. The earlier 17.33427257×
result was the provisional round-33 pair (4.9922705 / 0.288 ms), not the current
result. See [the normalization comparison](TIMING-COMPARISON.md). Raw receipts
and their original ratios have not been edited.

## Exact checkpoint and pins

- PR base: **eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b**.
- Published starting head: **bf64adb1e471213811470a9862c5a4c189a4c2e8**.
- Separate inherited CI repair: **921cd9194**.
- Original cleanup witness/refactoring: **1840ddaa2bb23b6eb3f1a50f565574b23015fdef**.
- Batch implementation, boundary tests and CI test inventory: **6ee0f54088f0fff805e0e144f10445cba41f704a**.
- Exact measured production source, after formatting and standard module roots:
  **9047e634e5964ad23ae19449bc425c2a3cf325b2**.
- IR/work/ABI/bytecode: **131/95/37/14**, unchanged logical prices and contracts.
- Source SHA256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- Bytecode SHA256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`, 4138627 bytes.
- Before worker SHA256: `71bace62e3b90fb7ef443183307f2311a4cf3cb181fe291fea15ebd03134b958`.
- After worker SHA256: `d94ea8611755f169f569c547dd58f9bd4d69d046aae041bb7d6d6015c9a4922b`.
- Final GPU library SHA256: `36702078dae5081c9f78ae1b862ed843001d288ab84099f5c39aac183cfc7d05`.
- Reference package: `671077457ff7ef3a3d44caf715a3fc595addf379`; compiler:
  `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`; executable SHA256:
  `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.
- Pinned mimalloc 2.1.7 Object SHA256:
  `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.

The baseline is the **restored** round-35 worker and its frozen libraries,
not that round's rejected cross-crate-inline experiment. All seven non-GPU
library hashes and selected production profiles agree. Full paths, toolchain,
commands, environments and file identities are in the archived JSON receipts.

Algorithm, tables, search limits, depth 300, nice length 258, near-EOF limits,
the actual 262144-entry U16 tree and 1500513-word U32 cache, source allocation
sites, output clear/rebind and timer boundaries are unchanged. The largest
complete compression input timed in this round is **eight bytes**. No larger
throughput or parity claim.

## Execution contract

Keep key and frozen-owner cleanup at their original sites. Pay the original
lexical scan first. Preview only the current arena/ledger/generations and the
existing independently derived frame-retirement recipe. Sum the original frame
prices and 32-unit arena prices with checked arithmetic. Only a group with more
than one original charge is eligible.

Pay that exact sum once only when the unchanged account and native ceiling
prove it fits, no call/observer scope is active and it is not a worker budget.
The unscoped account cannot change during this synchronous interval. Preview
rejection never becomes a source error: unsupported state, short allowance,
overflow, malformed slots, exhausted generations, foreign ledgers, scopes and
workers retain the original ordered entries and their exact spent prefix.

`PaidRetirements` is a non-cloneable mutable loan of the same meter with a private
remaining amount. It adds no work account, owner, heap allocation or refundable
fuel. Each slot releases the frame before the arena, in the original inventory
order. Both wrappers retain original validation and share the unchanged release
body; cheap receipt subtraction replaces repeated shared-ledger debits. Closing
requires exactly zero. Abandoning a receipt does not refund. Its mutable loan
prevents an ordinary helper from borrowing the owning invocation while open.

The existing retirement inventory and image rederivation remain authoritative.
There is no new source specialization, interpreter, opcode, pointer loan, owner
bank, program metadata, resource allowance or size cutoff. Published values,
input aliases, failure atomicity and recovery stay under the existing executor.
Persistent 1128 + invocation 1296 = **2424 bytes**, within the unchanged 2432-byte
base envelope; the maintained layout test passes.

## Meaningful regression evidence

**584 unique scoped maintained cases pass**:

| Maintained cohort | Executed cases |
| --- | ---: |
| GPU runtime | 311 |
| Work, memory transport and values | 31 |
| Existing component qualification filter | 233 |
| Parent/child composition integration | 8 |
| Actual runtime-only linked/AOT recovery | 1 |
| Total | 584 |

The unit root was built while the batch candidate was uncommitted atop
1840ddaa2; its receipt records every changed source hash. Production/test bodies
are retained in 6ee0f5408. Subsequent changes only sort a module declaration and
move two modules to standard directory roots; the complete diff is in history.
The final formatted unit root is rebuilt at 9047e634e and lists/executes the
guarded selection: **13 actual tests pass**, not thirteen additional unique
cases. Production components and linked runtime use 9047e634e.

The new boundary oracle independently uses the original ordered entries. For
original and decoded nested scalar/managed frames, every contributing charge
boundary and one-short denial compares exact error, spent prefix, frame words,
managed owners and arena state. It covers malformed slots, foreign ledgers,
nested/matching scopes, generation exhaustion, aliases and retry. Work-meter
tests cover exact funding, overdraw, incomplete receipt/no refund, overflow,
finite cap, scope fallback and parent-backed workers without renewed fuel.

The original Huffman parent/child source witness returns **[3,1,3,2]**, exactly
**91452918 work**, **seven reached batches per invocation**, repeated in both
original and decoded forms, with unchanged arguments and zero cleanup. The
red witness selects zero batches before implementation. Existing runtime and
component tests retain publication-preserving failure/retry and alias coverage.
No old component expected value or allowance is increased.

The component command executes 233 rather than the old harness's asserted 232:
all original 232 names pass, plus the already-existing image-authority rejection
test introduced in round 33. The inventory difference is explicitly preserved.

The actual generated linked project uses the unchanged Object and descriptor
and the maintained driver, linked to the new runtime. Its closure has only GPU
`canonical-native-program-runtime,canonical-native-runtime`, no parser or code
generator. Inputs 3 → 0 → 2 publish 4, reject the assertion before arithmetic
while preserving 4 and aliases, then publish 6, with cleanup and recovery.

All **12 scalar controls** pass the unchanged 1.05 timing ceiling; maximum ratio
**1.006226474**. Allocation requests/bytes do not increase; pure scalar counts
agree exactly. All **eight managed-frame controls** retain requests, bytes and
work, with zero 0/64-step allocation slopes. These are controls, not extra
complete-compression gains or an end-to-end allocation census.

## Machine code and fresh profile

Freshly generated compressor Object, metadata and descriptor are byte-identical
to the retained round-33/34 program: Object 13802344 bytes, SHA256
`d2c0185ac19d2279c477386cb9f51b080bb4ea159735bbae61101930bfe2cf13`;
metadata 2411445 bytes; descriptor JSON 59784245 bytes. JSON size is not a
descriptor heap-footprint measurement. Native program code/stack remain
13801616/108560 bytes. The optimization is in the Rust runtime.

The linked Rust worker's text shrinks **48187088 → 48185332 bytes** (1756).
The full cleanup function grows 335 → 616 static instructions because it now
contains a preview, selected loop and original fallback. The batch funding
function has 86 static instructions. Disassembly places one batch-funding call
before the selected per-slot loop; the selected loop and shared frame-release
body contain **no shared-ledger debit or scope lookup calls**. The original
charged frame/arena calls remain in the separate fallback loop. Static counts
are mechanism evidence, not a timing model.

The fresh profile contains **695 diagnostic compressions**, excluded from
primary, cleanup zero. It has **1339 disjoint exclusive weights**: frame actions
384, ordinary visits 213, other helpers 279, selected library calls 120 and
remaining 343. The previous recorded restored profile has 1354 weights; the
underlying compute debit has 61 exclusive weights before and 40 after. Samples
are unpaired diagnostic evidence, not exact fractions of elapsed time. Frame
actions, ordinary helper/control dispatch and accounting remain substantial.
Any next extension needs a bounded witness before changing their semantics.

## Commands, features and CI repair

Nightly **2026-03-03**, aarch64 macOS, locked/offline, jobs 1, incremental off,
warnings as errors. Production GPU opt 3 with the same six features:
`aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`.
The maintained unit/integration diagnostic roots use opt 1. Exact rustc extern
paths and feature definitions are archived, including the reduced operation
catalog. Principal commands:

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review --features mech-runtime/compiler
<GPU unit root> native_artifact::program::runtime:: --skip complete_unchanged_knucleotide_body_uses_the_owned_emitted_map --nocapture --test-threads=1
<GPU unit root> native_artifact::work:: native_artifact::memory:: native_artifact::values:: --nocapture --test-threads=1
<maintained component root> <unchanged component filter inventory in existing.json> --nocapture --test-threads=1
<maintained component root> numeric_composition_qualification --nocapture --test-threads=1
<final GPU unit root> composition_tests parent_continuation_is_single_use native_runtime_consolidation buffer_frames::tests::batching work::retirement::tests exhausted_retirement_generation --nocapture --test-threads=1
<generated linked project> cargo build --offline --locked --bin lexical-retry
<configured Python> -B -m unittest tests.ci.test_ci_full_contract tests.ci.test_ci_test_inventory
```

The separate CI repair in **921cd9194** addresses review comment 4426176445.
The published round-33 inventory shell command violated an existing step's
Cargo-only contract. Clean base passes the focused contract and the published
parent fails it. The unchanged guarded list/count/run moves to a separate native
step; both existing source profiles and warnings-as-errors remain. **132 Python
workflow/inventory tests pass**, and the original five-test Rust selection is
actually listed and executed. The batch implementation separately extends the
guard to 13 and actually executes all thirteen. See [CI repair](ci-inventory-repair/REPORT.md).

All **12 architecture checks pass**: resident routing, source catalog, host
catalog, build layout, Rust module layout, factory safety, warning policy,
planning quarantine, CI inventory, unsafe boundaries, formatting and diff.
No check or policy is weakened. Hosted CI is cancelled immediately on publication
as requested; local receipts do not substitute for a green hosted run.

## Failures and coverage limits

- **Inherited published-stack CI failure:** introduced in round 33 at 2de2b627d;
  fixed separately in 921cd9194 with parent/base evidence and actual consumers.
- **Introduced work-in-progress errors:** initial helper module path and an arena
  extraction matched the wrong 32-unit charge. The compiler rejected both; the
  original `put` is unchanged and the correct retirement body is shared. Raw
  failed builds are retained, followed by passing qualification.
- **Introduced witness setup error:** retaining a previous published output while
  requiring the no-previous-output work total adds the existing coexistence fee
  (91480762 rather than 91452918). Clear it outside the witness invocation for
  identical entry conditions. Existing retained-publication recovery tests still
  pass; no old expectation is altered.
- **Introduced formatting/module-layout failures:** sorted the module declaration
  and used standard directory roots; rebuilt final production and the guarded
  unit selection. No audit exemption.
- **Harness inventory mismatch:** Rust passed all 233 component cases; an outer
  assertion expected 232. Name comparison proves all old 232 plus the existing
  image rejection case. Corrected only the inventory assertion. An initial name
  extraction regex was also corrected; raw diagnostics remain.
- **Environmental Python failure:** system Python lacks `yaml`; the existing
  configured environment executes all 132 tests successfully.
- Read-only path discovery and a documentation patch-context attempt also failed;
  neither changed production source or supplied a timing observation.
- **Inherited reduced-catalog exclusion:** the whole-knucleotide stats case has
  the already-reproduced missing-module failure; its same exclusion is retained.
  Selected not/xor is outside this catalog. Full GPU/Core/Engine feature suites,
  unrelated integration targets and larger-input timing are not claimed here.

No new semantic, qualification or protected-control failure remains unresolved.
The previous rejected round-35 experiment stays removed and archived. Parity is
not established. This packet closes the current change for review; it does not
authorize an unqualified wider native execution path.
