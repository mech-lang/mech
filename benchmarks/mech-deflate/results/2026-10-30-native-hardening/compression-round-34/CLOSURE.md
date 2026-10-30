# Work-meter inlining: retained 2.35% improvement

The only production change is `#[inline]` on the existing
`NativeWorkMeter::charge` method. Rust can fold its wrapper and error mapping
into callers. Prices, work checks, scopes, owners, error precedence and source
semantics are unchanged. No compiler admission or instruction allowance rises.

## Comparable results

Complete pinned eight-byte `abcdefgh` compression, unchanged evaluation-only
timer, pinned mimalloc and the same independently prepared worker design:

| Measurement | Before | After |
| --- | ---: | ---: |
| Primary median | 5.0355415 ms | 4.9173750 ms |
| Fresh common reference = 1x, 0.279 ms | 18.04853584x | 17.62500000x |
| Fixed older 0.300 ms reference ruler | 16.78513833x | 16.39125000x |

Elapsed reduction: **2.34664931%**; after/before **0.97653351**.
All four independently prepared worker pairs improve: 2.33%, 2.10%, 2.06%
and 2.97%. **57/64** descriptive blocks improve. Median block ratio 0.97525912,
geometric ratio 0.98006439. The blocks share four pairs, not 64 independent
experiments. Both evaluation-order strata improve: before/after median ratio
0.97484573 (27/32 blocks), after/before 0.97543739 (30/32 blocks).

All **3984** observations are retained, including **3968** primary calls.
The fresh reference has **20000** observations and measured the same 0.279 ms
median as the preceding round. Cross-session medians remain separate matched
experiments; the paired table above is the improvement claim. The older fixed
ruler is included to make changes in the denominator explicit.

Each reference and worker cohort was committed immediately before the next one.
Reference `33c225a6a`; pairs `9cf89721a`, `ca3ed60be`, `f909db10c`,
`f72b2f7e7`; summary `03acf6f9e`. There were no interrupted, discarded or
superseded primary cohorts in this round. All results, including slower blocks,
are retained. No builds, tests, audits, profiles or JIT preparation overlapped
primary evaluation. The peer worker waited outside the timer.

Every compression verifies the exact ten reference raw bytes, independent
inflation, input contents and aliases, output capacity 13 and final zero cleanup.
Logical work is **44443887150 in both variants**, for every observation.
The source, algorithm, tables, search limits, depth 300, nice length 258,
near-EOF limits, 262144-entry U16 tree, 1500513-word U32 cache, source allocation
sites and output clear/rebind boundaries are unchanged. No larger-input timing
or parity claim is made.

## Exact source and build

- PR stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
- Starting published head: `6123fdf46b749c5b5793591be7f9ed92f38269da`.
  Clean review: PR 973 issue comment `6526753449`.
- Production compiler: **`8c60a8cee62c01257b2c282dea95755a141307c8`**.
  Later commits contain evidence and documentation only.
- IR/work/ABI/bytecode remain **131/95/37/14**.
- Source SHA256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- Artifact SHA256 `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- Reference binary SHA256 `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.
- Pinned mimalloc 2.1.7 Object SHA256
  `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
- Baseline worker SHA256 `5119d358bf67c2601c7ba19152a4a232f2af7dd275278f6753aeeec9a90bd654`.
- Candidate worker SHA256 `09d6b7aeb11c66c4721d32e8f805646f3be424ace75b5d176184d5d7260dd20a`.
- Candidate ordinary harness SHA256
  `5bac7a2c9b63faa814d4ea9815dd490cf7d306e049c59220e71637a2a872a34f`.
- Unchanged worker-source SHA256
  `a0062c6170db0005d96eba239232cfdbdd41054945c2e4a574afa8acb8939575`.

Nightly 2026-03-03, aarch64 macOS; release opt 3, locked/offline, jobs 1,
incremental disabled, warnings as errors. GPU features:
`aot`, `canonical-native`, `canonical-native-program`,
`canonical-native-program-runtime`, `canonical-native-runtime`, `jit`.
All seven non-GPU linked library hashes are identical. Only the GPU crate and
its consuming binaries rebuilt; build duration was 76.25 seconds. Libraries
and executables were frozen before any evaluation. Exact commands, environments,
profiles, hashes and logs are in `qualification/` and the cohort archives.

Production build:

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review --features mech-runtime/compiler
```

## Binary and profile evidence

The retained baseline frame-retirement helper calls the 86-instruction work
wrapper. Candidate retirement calls the maintained library debit directly.
That particular helper grows 124 to 155 instructions because wrapper checks and
error paths are folded into it; the complete linked worker's Rust `__text`
instead falls **48197444 to 48187088 bytes**, a net **10356-byte reduction**.
These are static instruction counts, not executed counts. The wrapper still
exists for callers where the optimizer elects not to inline it.

Freshly generated compressor Object, metadata and canonical descriptor are
**byte-identical** to the previous compiler's output. Object is 13802344 bytes,
code 13801616 bytes and stack 108560 bytes. Object SHA256
`d2c0185ac19d2279c477386cb9f51b080bb4ea159735bbae61101930bfe2cf13`;
descriptor SHA256
`7b6cfa95f6735e77b12842bbd2e9aa94388d6f04b29c8417b30e01f0cb21520e`.
The improvement is in the maintained Rust runtime called by that Object.

The separate fresh profile contains **695** complete diagnostic compressions,
all excluded from primary timing, with exact results and zero cleanup. Raw
profile committed immediately in `735cec2a0`. Exclusive sampling weights:

| Category | Prior profile | Fresh profile |
| --- | ---: | ---: |
| Frame actions | 348 | 332 |
| Ordinary visits | 246 | 237 |
| Other invocation helpers | 270 | 287 |
| Selected library calls | 121 | 141 |
| Remaining | 366 | 357 |
| Total | 1351 | 1354 |

The targeted work wrapper falls from **27 to 2 exclusive samples**. This is
sampling attribution, not proof of zero cost or a paired elapsed-time estimate.
The underlying `charge_compute_with_limit` remains prominent (68 exclusive
samples), along with TLS lookup (54), frame retirement (54), helper dispatch
(52), visits (43) and the shared debit (39). Those account and retirement paths
are the next bounded investigation; this round does not change their contracts.
The pinned reference disassembly is retained as a comparison of generated
operations, without treating its large function boundary as equivalent to a
small Mech runtime helper.

## Qualification

**321 maintained tests pass** in the actual GPU unit root, opt 1, same selected
features, warnings as errors:

```text
gpu-unit-tests native_artifact::program::runtime:: --skip complete_unchanged_knucleotide_body_uses_the_owned_emitted_map --nocapture --test-threads=1
# 308 passed, 0 failed; 131 filtered out
gpu-unit-tests native_artifact::work:: native_artifact::memory:: --nocapture --test-threads=1
# 13 passed, 0 failed; 426 filtered out
```

Existing cases cover scalar/shared-account accumulation, finite and one-short
denials, overflow and error precedence, foreign and nested scopes, parent-backed
workers, current frames/continuations, failures, unchanged publication, aliases,
cleanup and retry. No test or expected value was changed for this hint.

All **12 protected scalar controls** pass the unchanged 1.05 timing ceiling;
maximum ratio **1.016038735**. Allocation requests and bytes do not increase;
pure scalar counts/bytes are identical. Both scalar loop forms retain zero
allocation slopes at 0/64 steps. All **eight managed-frame controls** preserve
requests, bytes, work and zero slopes. Raw controls were committed immediately
in `d961a7f7d` and `476b74c47`.

Warnings-as-errors remain enabled. Formatting and diff checks pass, with their
exact results recorded in the final-check receipt. Routing, catalog, ownership,
codec and safety-boundary definitions are unchanged; no new unsafe code exists.
The previous round's 574-case qualification and architecture receipts remain
retained. They are not claimed as freshly rerun in this round.

## Limitations and disposition

No build, maintained test or protected timing/allocation gate failed in this
round. The same inherited reduced-catalog whole-knucleotide statistics exclusion
is retained. Broad Core/Engine/full-feature and linked/AOT suites, the 232-case
component cohort and selected not/xor coverage were not rerun for an inline hint.
Those exclusions are not evidence of their success in this configuration.

Hosted CI is cancelled by user direction after publication; no green-CI claim.
The PR stays draft pending green CI and clean review. Parity remains unachieved.
The next experiment must be motivated by the fresh account/retirement profile,
preserve prices and failure order, and pass the same matched timing and controls.
