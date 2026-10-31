# R66: one owner through private numeric computation

Actual candidate compiler: `739c8b3bf4a11dcfaa87800e6cb171390afc2ff1`. Production change: `60c96f56e11e557a4807976c42f7ab674942aefa`. Parent compiler: `78209b2aab2175f1fad92ee4daae3d6b38669d36`. Accepted parent checkpoint: `8e549d40300d1bfcc8f9a2845987f6979a573c34`.

The complete unchanged eight-byte level-12 call improves **12.896272× → 12.704605× the fresh reference**, or **-1.486218%** pooled elapsed. All 4/4 independent pairs and 61/64 blocks improve. Exact raw output and independent inflation agree.

The checked replacement/append previously constructed an admitted immutable result, immediately consumed its buffer, then constructed another owner for the private phase. It now shares the original operation implementation and confines the first fresh result directly to that phase. One final publication owner survives the selected leaf, post-leaf metadata admission, append-preparation handoff, repeated native updates and final reconciliation. Aliases of the input still force the same checked copy route.

## Observed whole-call results

| Metric | Reference | Parent | Candidate | Candidate change |
|---|---:|---:|---:|---:|
| Primary elapsed, ms | 0.285000 | 3.6754375 | 3.6208125 | -1.486218% |
| Retired instructions | 1,364,372 | 41,076,087 | 40,590,096 | -1.183148% |
| CPU cycles | 924,069.5 | 11,728,479 | 11,416,448.5 | -2.660452% |
| Allocation requests | 95 | 8,105 | 7,097 | -12.436767% |
| Requested bytes | 9,021,923 | 98,227,372 | 98,146,732 | -0.082095% |
| Reallocations | 0 | 162 | 162 | +0.000000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 163,464,584 | +0.000000% |
| Incremental allocator peak | 9,257,328 | 20,482,072 | 20,482,072 | +0.000000% |
| RSS after call | 14,860,288 | 319,324,160 | 319,275,008 | -0.015393% |
| Physical footprint after call | 14,517,184 | 228,935,360 | 228,951,808 | +0.007185% |

| Multiple of the same reference metric | Parent | Candidate |
|---|---:|---:|
| Instructions | 30.106222× | 29.750021× |
| Cycles | 12.692204× | 12.354534× |
| Requests | 85.315789× | 74.705263× |
| Requested bytes | 10.887631× | 10.878693× |

Primary elapsed, CPU counters, heap counters and CPU-time samples are independent serial cohorts. Their gains are not multiplied. The fresh reference median is 0.285 ms; earlier rounds used other fresh reference medians. Across-round multiples are therefore not a matched regression test. The paired current elapsed result is the comparison. Requested bytes are allocation capacity requests, not initialized bytes copied. Post-call RSS/physical footprint includes retained compiler/catalog/image storage and is not peak RSS.

## Construction and lifetime evidence

The complete fresh censuses independently reconcile to heap counters. Exactly **1,008 requests / 80,640 bytes** disappear. There are **168** first-change transitions. The direct first-change leaf family changes **1,134 → 966** requests; the separate old owned-to-private reconstruction family changes **672 → 0**. The necessary final shape/data/root triple remains **504 requests**, rather than being pooled or hidden.

| Removed family | Requests | Requested bytes |
|---|---:|---:|
| runtime handoff shape-parameter clone | 168 | 1,344 |
| reconstructed final shape owner | 168 | 5,376 |
| intermediate canonical import registry lock | 168 | 10,752 |
| reconstructed final registration | 168 | 17,472 |
| reconstructed final root owner | 168 | 18,816 |
| reconstructed final data owner | 168 | 26,880 |

Every other size/kind family is unchanged. Large reservations remain: five 524,288-byte tree requests, one 6,002,052-byte cache request and 35 2,442,056-byte path requests. This patch removes metadata construction, not those remaining backing requests. No initialized-copy volume is inferred from requested capacities.

The new Core confinement entry requires a fresh unadmitted plain unsigned matrix with exclusive root and data owners, including no weak observer, no existing import and no physical payload ticket. It invalidates any pre-confinement fingerprint cache. The same complete conservative byte/node witness and current account registration are retained until publication; no authority from an earlier invocation is cached. Original selected-call charges, finite error precedence, copy/consume decisions and post-leaf metadata charges stay ordered. Finish is allocation-free and abort releases unpublished owners.

One real allocation/registration was removed, so its former allocation-failure ordinal is not synthesized. Logical admission and publication/retry behavior are checked independently. No work-policy or bytecode revision changes.

[Materialization inventory and design](https://github.com/mech-lang/mech/blob/739c8b3bf4a11dcfaa87800e6cb171390afc2ff1/docs/design/native-private-numeric-lifetime.md). The inventory covers this path, including its remaining canonical region exit. It does not claim every compressor materialization has been eliminated or justify a promise of 95 allocations.

## Real-capacity compiled controls

These are separate diagnostic programs using ordinary maintained compilation and evaluate_into. Each uses the same artifact for changing step arguments. Table/cache inputs are admitted outside the counter, retain caller aliases and remain all-zero after every changed result. Path/output use source with-capacity/append. The added real-size controls use the existing NoLimits mode; existing finite scalar/component controls and the primary workload are unchanged.

| Buffer | Logical length / capacity | Reached updates | Parent requests/call | Candidate requests/call |
|---|---:|---|---:|---:|
| U16 tree | length 262144 | 2, 65, 4097 | 241 | 235 |
| U32 cache | length 1500513 | 2, 65, 4097 | 243 | 237 |
| U64 path | capacity 305257 | 2, 129, 266 | 260 | 254 |
| U8 output | capacity 13 | 2, 7, 13 | 262 | 256 |

All 24 original/decoded real-capacity cases pass on both compiler graphs; all request and requested-byte slopes are exactly zero as visits increase within fixed capacity. The 12 smaller private-loop cases pass as well. The parent already had allocation-free repeated updates: the new saving is a removed constant entry conversion, not an invented new zero-slope claim. Further private-region handoffs and owner-slot workspace reuse remain separate work.

## Machine code and profiling

The freshly emitted compressor Object/image/metadata are byte-identical: generated text **11,800,572 bytes**, entry stack **107,344 bytes**, IR/ABI **132/37**. The actual linked host text grows **+12,164 bytes**, 60,823,288 → 60,835,452. Worker file size changes -27,184 bytes. The text increase is retained as a tradeoff, not concealed by total file shrinkage.

Actual ARM64 excerpts contain the new confinement and private leaf entries. The former post-leaf into_private_updates/appends and PrivateUnsignedMatrix::prepare entries are no longer linked. Ordinary canonical admission and input consuming-transfer routines remain for their real consumers. Static function instruction counts do not establish dynamic execution counts or a maximum call-stack depth. Whole-call retired instructions fall independently by 485,991.

Fresh samples include 1,321 parent, 1,329 candidate and 1,525 reference exclusive weights. Candidate frame-action ancestry is about 27.16%, TLS lookup 3.91%, tail cleanup 3.54% and memory move/copy 2.56%. Ancestry groups overlap. These are sampling leads, not instruction partitions; the copy and canonical-construction sample shares increase despite the net elapsed and instruction improvement. No per-function instruction ledger is claimed.

## Pairs and contrary observations

| Pair | Parent ms | Candidate ms | Candidate/parent |
|---|---:|---:|---:|
| 0 | 3.6769165 | 3.6200630 | 0.98453772× |
| 1 | 3.7021665 | 3.6294790 | 0.98036623× |
| 2 | 3.6719165 | 3.6218335 | 0.98636053× |
| 3 | 3.6657085 | 3.6137915 | 0.98583712× |

Separate CPU-cohort elapsed: 3.6610000 → 3.5621875 ms (-2.699058%), excluded from primary. Primary has four independent pairs, 64 descriptive blocks and 3,968 timed observations. Three blocks are adverse. Scalar controls include adverse cases up to 1.01104422× within the unchanged 1.05 ceiling; no universal scalar speedup is claimed. Allocator peak is unchanged. Post-call physical footprint rises 16,448 bytes. Sampling groups retain their contrary directions.

## Qualification and provenance

**passed**: 71 required receipt checks pass. Actual executed batches include Core 367; runtime 329; components 236; connected 8; linked 12; work/memory/value 32; preparation 2; parent/candidate projection 12 and shape 6; scalar controls 12; frame controls 8; private-loop cases 12 plus real-capacity cases 24; GPU CI 35; workflow Python 132; architecture 10; and the strict 14-owner inventory with 21 owner/string tests. Batches overlap and are not added as unique tests.

The candidate rebuilds all twelve selected Core-dependent project crates coherently with exact retained release flags/features and warnings-as-errors. External dependencies remain frozen. Core tests use the retained current-source unit profile; GPU unit tests use retained opt1; release libraries use opt3. Parent Engine controls reuse its exact pinned test executable and execute afresh with historical build provenance. Candidate Engine/GPU tests are newly compiled. No minimal-profile or all-features build claim is made.

The CI inventory change is separately reviewable in `452b78037`: 11 →14, preserving the actual list/count/run guard. The old Bash/awk guard fails against the new real 14-test inventory; the corrected guard passes and executes all 14 unsigned plus seven string tests.

The unchanged workload retains source/MBC/allocator pins, tables, buffers, depth 300, nice 258, original source allocation sites, output capacity 13 clear/reuse, input aliases and evaluate_into timing. Exact raw bytes `[75,76,74,78,73,77,75,207,0,0]` independently inflate to `abcdefgh`. Logical work **60,260,219,686**, comparison work **47,612** and final managed bytes/nodes **zero** agree.

```json
{
  "source_sha256": "331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9",
  "bytecode_sha256": "a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7",
  "allocator_sha256": "38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874",
  "logical_work": 60260219686,
  "comparison_work": 47612,
  "input_bytes": 8,
  "expected_output": [
    75,
    76,
    74,
    78,
    73,
    77,
    75,
    207,
    0,
    0
  ],
  "output_capacity": 13
}
```

Failed attempts and dispositions:
- introduced harness setup: Created required empty parent directories; rerun initialization and dependent stages with checked sequencing. Preserved the empty Core directory created by the first rejected attempt as core-focused-uninitialized before retry.
- introduced harness setup: Copied the manifest-verified regular receipt to host.json; compiler binary and flags unchanged. Future build helper writes both names. Preserved the empty after directory from the first rejected setup as after-uninitialized; retry guard initially rejected that existing directory before execution.
- inherited unsupported native operation in an added fixture: The initial initializer diagnosis was incorrect and is withdrawn. Only widening U16 arithmetic to U64 and checking the cast back makes the real-size witness pass, matching pinned source rebase-node at lines 76–78. Minimal U16-add source and both failures are retained. No native vocabulary expansion is required by the actual compressor; this inherited operation remains unsupported and separately assigned. Large diagnostic controls use the existing NoLimits mode; existing finite controls and benchmark settings are unchanged.

The provisional initializer diagnosis was wrong and is corrected explicitly. Direct native U16 addition remains unsupported in both compilers. The real compressor already uses widened U64 arithmetic and checked narrowing; the added U16 witness now uses that same pattern and passes. The minimal rejected source and raw failures remain in the findings packet. This inherited operation is assigned separately and does not block the current compressor. No introduced production failure remains.

Skipped coverage:
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local selected tests with their actual retained feature configurations and architecture checks are reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization measures the complete pinned eight-byte level-12 call only.

Decision: **retained**. Four of four primary pairs and 61 of 64 blocks improve; pooled elapsed falls 1.486218%. Independent retired instructions fall by 485991 (1.183148%). Exact construction delta is 1008 requests/80640 bytes, including a removed owner/registration round trip and shape clone. All required regressions and real-capacity original/decoded no-slope controls pass without widening existing allowances. Retain the 12164-byte host-text increase, unchanged allocator peak and adverse OS-footprint/sample directions. No allocation or runtime parity claim.

## Replay and next boundary

Run `python3 reconstruct.py /new/empty/output/path` from this packet. Every archived version is verified before overlay; seven numerical outputs are reconstructed byte-for-byte from retained evidence without binaries, compiler execution or external scratch inputs.

Next: reclassify the current remaining construction chains by prepared-program, invocation, private-computation and observable ownership lifetimes. Immutable closed-schema binding/plan geometry and sequential owner-slot workspace are the next bounded targets. Keep current values, account authority, finite limits and funding dynamic. The first-change intermediate removed here is completed work, not a future proposal.
