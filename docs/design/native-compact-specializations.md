# Complete compact-native specializations (R207)

Existing numeric phases and packed byte reads now execute through the complete compact diagnostic. This removes substantial work, but **the diagnostic is not adopted**: six original production gates still fail and ordinary contracts remain omitted. Production stays **R199, IR/work/ABI/MBC214/107/39/14**. All 20 complete streams agree with the pinned reference, including the 33,027-byte case. Parity and the full brief remain unfinished.

## Exact checkpoint and compiler mechanism

Compiler base `da4a2d2819803335480ac6490947cefccd95eb7c`; retained production `7748204bd12e57a20369ce7970f31f8d15c3901f`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode SHA-256 `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. The exact27-file candidate patch is archived externally, not substituted for the ordinary entry. Existing draft #973 only.

The R204 closed-Plan emitter previously lost the retained numeric/word-read specializations. This candidate reuses the existing closed-tail proofs, numeric operations, reduction/window lowering and packed-read derivation. It connects four synchronous native phases through typed private-array boundaries, with 28 input words and 12 output words each. Only current writable arrays/owners are exported. Actual current exclusivity gates writable entry; a live borrowed owner uses the existing checked raw path. Ordinary first change can create unique current storage, allowing a later visit to enter the phase.

Append growth happens at the reached source operation. It uses the existing growth calculation, actual allocation and initialized copy, transfers the new owner, and releases the previous owner. No eager capacity substitution or zero-filled path matrix is introduced. Twenty-seven static word-pack sites share the existing little-endian2/4/8-byte machine emitter. Current-window and vector-geometry guards retain the original checked operation chain when they decline. Source algorithms, tables, search/EOF limits, allocation sites and real capacities are unchanged.

The timed/default Object has no phase/pack observers. A separate observed image establishes dynamic selection without contributing timings or instruction counts. Its natural32 execution selects 308,196 packs; periodic33 selects 2,135,935; repeated32 selects 2,121,659. All have zero pack fallbacks. Natural32 executes the two relevant phase functions 1/9 times; repeated and periodic execute phase 15296 twice. Static sites and dynamic visits are distinct.

## Complete-call elapsed

Fresh current retained, previous diagnostic and new diagnostic run in twelve balanced permutation blocks per case. There are 8,496 retained timing observations /8616 complete Mech calls and 5,120 separate pinned-reference compressions. The previous diagnostic is R204, not a production baseline. Preparation, clear/prefix, import/rebind, validation, output slots and drop are excluded identically; actual compression construction, allocation, copying, output transfer and private cleanup remain measured. The allocator is unchanged mimalloc 2.1.7.

| Input | Reference ms | Retained / Roc | R204 diagnostic / Roc | R207 diagnostic / Roc | Diagnostic elapsed change |
|---|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 4.363000 | 3.359605× | 14.451672× | 7.053131× | -51.195% |
| 33027-periodic-window | 1.254000 | 3.554625× | 24.794258× | 12.563796× | -49.328% |
| repeated-32769 | 1.337000 | 3.055192× | 23.096531× | 11.709860× | -49.300% |
| random-32769 | 1.055000 | 4.840186× | 23.908156× | 19.053061× | -20.307% |
| 16384-distribution-change | 0.790000 | 6.395622× | 20.790427× | 14.887447× | -28.393% |
| eight-literals | 0.290000 | 4.489941× | 2.169183× | 2.193750× | +1.133% |
| reymont-prefix-16384 | 2.128000 | 4.137913× | 14.977492× | 6.915962× | -53.824% |
| empty | 0.000000 | — | — | — | -16.400% |
| one-stored | 0.000000 | — | — | — | -10.072% |
| seven-stored-prefix | 0.000000 | — | — | — | -10.072% |
| eight-overlap | 0.290000 | 4.344181× | 2.137357× | 2.153521× | +0.756% |
| 320-overlap | 0.298000 | 4.188688× | 2.977000× | 2.478188× | -16.756% |
| eight-literals-prefix | 0.285000 | 4.561770× | 2.226168× | 2.233114× | +0.312% |
| 128-biased-dynamic | 0.285000 | 3.833116× | 2.154165× | 2.088816× | -3.034% |
| 32-high-entropy-stored | 0.288000 | 4.105615× | 2.468823× | 2.446833× | -0.891% |
| random-64 | 0.297000 | 4.066498× | 2.609778× | 2.567130× | -1.634% |
| random-256 | 0.299000 | 4.178651× | 3.512472× | 3.323998× | -5.366% |
| random-1024 | 0.303000 | 3.888751× | 4.411167× | 3.880155× | -12.038% |
| repeated-1024 | 0.323000 | 4.085655× | 5.060178× | 3.258384× | -35.607% |
| reymont-prefix-1024 | 0.330000 | 4.635291× | 6.130998× | 4.240782× | -30.830% |

**Seventeen diagnostic medians fall and three rise; geometric elapsed changes −22.164358%.** All 20 unchanged 1.05 gates pass against R204; only 14/20 pass against retained production. Empty, one and seven-byte reference elapsed rounds to zero in the existing clock harness, so no elapsed multiple is computed. No favorable unchanged rerun replaces these observations.

Natural32 complete-call medians are retained 14.657958ms, previous 63.052646ms, new 30.772813ms. All twelve matched blocks improve versus R204. Eight literals is contrary: 0.629063→0.636188ms, +1.132557%; eleven blocks worsen and one improves. Its separate instructions also rise 5.1912%. Those observations remain in the packet.

## Instructions, cycles and memory

CPU/heap cohorts are separate from primary elapsed: 2,400 complete Mech calls, 18 retained observations per side/case/mode. Fresh reference CPU/heap cohorts reuse the exact 40 R206 executables and verified fourteen-file pinned source package, with 1,280 counter observations /10240 verified compressions across all 20 inputs. Same-input reference counts are used below; raw window counts include probe overhead without subtraction. CPU-probe allocation zeros are uninstrumented, not zero-allocation evidence.

| Input | R204 instructions | R207 instructions | Instruction change | Retained instructions / Roc | R207 instructions / Roc | Cycle change vs R204 |
|---|---:|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 649,613,828.5 | 354,741,962.0 | -45.392% | 4.375800× | 7.455559× | -51.187% |
| 33027-periodic-window | 591,795,470.5 | 266,104,241.5 | -55.034% | 3.762776× | 13.812983× | -49.436% |
| repeated-32769 | 588,070,356.5 | 267,199,886.0 | -54.563% | 3.495823× | 13.973643× | -49.251% |
| random-32769 | 278,314,975.5 | 223,416,496.0 | -19.725% | 6.042392× | 19.009671× | -20.277% |
| 16384-distribution-change | 181,442,689.0 | 133,085,507.0 | -26.651% | 6.904856× | 14.205638× | -27.822% |
| eight-literals | 4,999,133.0 | 5,258,648.0 | +5.191% | 9.111421× | 3.854263× | +0.260% |
| reymont-prefix-16384 | 324,423,075.5 | 167,733,899.0 | -48.298% | 4.789221× | 6.758247× | -53.923% |

Twelve instruction medians fall/eight rise; fourteen cycle medians fall/six rise. Instruction, cycle, elapsed and sample cohorts are not multiplied or pooled. Exact all 20 counters, reference denominators, process RSS/physical-footprint observations and raw measurements remain in `results.json` and the per-case receipts.

All 20 diagnostic allocation-request, requested-byte, reallocation and incremental usable-peak medians are exactly equal before/after. This candidate removes native computation, not allocation.

| Input | Retained requests / requested B / incremental usable peak B | Diagnostic requests / requested B / incremental usable peak B | Reference requests / requested B / incremental usable peak B |
|---|---:|---:|---:|
| reymont-prefix-32769 | 6,449 / 24,793,116 / 18,722,528 | 9,224 / 12,057,256 / 10,013,032 | 276 / 9,115,076 / 9,257,328 |
| 33027-periodic-window | 3,679 / 15,798,298 / 15,314,928 | 932 / 10,262,803 / 9,972,104 | 83 / 9,012,161 / 9,257,328 |
| repeated-32769 | 3,135 / 15,712,250 / 15,313,264 | 354 / 10,123,399 / 9,972,104 | 83 / 9,012,134 / 9,257,328 |
| eight-literals | 3,354 / 17,643,969 / 15,279,752 | 498 / 9,091,628 / 9,193,416 | 95 / 9,021,923 / 9,257,328 |

Heap metrics above include all instrumented allocator requests, not just the concrete payload helper. Diagnostic `initialized_bytes_copied` counts tracked COW-helper copying only, not every memcpy/memmove or initialization. Requested capacity, initialized copies, live payload, usable heap, process RSS and lifetime physical peaks are different quantities. Preserved input aliases do not establish streaming or zero input copying. Physical output backing reuse remains unestablished.

## Current actual binary and exclusive profiles

R204 generated code is 8,315,704B with 78,160B root stack. R207 root is 8,897,684B /79696B, plus four 2,208B /304B phase functions: **8906516B total generated text and 80,000B root-plus-max-phase static stack envelope**. Production metadata already includes helpers: 8,152,436B /16480B static envelope. These are static native metadata, not peak process stack/RSS. The diagnostic code-size and stack growth are adverse costs retained with its timing gains. Original generation takes 199.901847s outside timing; decoded generation 204.584017s.

Object SHA-256 `c5bbbc5618577f3c5de9f89c03de0ce6f4cedc0cc7d5415ceae627598d56cd04`; timed worker `7dff5ab09cfd039122e0c599b1c7112af2de6fcc408a321fff45471eb5e3e2a3`. Original and decoded Objects and metadata are byte-identical. Four fresh stack profiles execute 8,669 verified complete calls; four exact-PC profiles execute 20,790. All use this exact default worker, not the observed image.

| Input | Native exclusive sample weights | Host below native | Other worker/system |
|---|---:|---:|---:|
| 33027-periodic-window | 99.213% | 0.569% | 0.219% |
| eight-literals | 79.301% | 12.707% | 7.991% |
| repeated-32769 | 98.819% | 0.656% | 0.525% |
| reymont-prefix-32769 | 98.733% | 1.005% | 0.262% |

These are disjoint timer-stack weights, not retired-instruction shares, callback counts or removable fractions. The whitelist contains the root and actual four generated phase symbols; allocation helpers remain host. The diagnostic removes generic operation dispatch but still calls concrete allocation/growth/free helpers. An exclusive production host-operation-family ledger and complete initialized-copy census remain unfinished; this diagnostic profile does not substitute for them.

The actual large-input hot root assembly repeatedly reloads array length/geometry, records source error origins into stack slots, and increments then decrements an array owner around a pure copy. The source correlation checks all 41,039 Plan origins against the current image before mapping sites2031/2032 and2823/2829; no historical address map is applied to changed code. Existing compact phase 15296 takes about 16.2% of natural stack weights; most remaining diagnostic work is in the much larger raw root.

The exact current reference executable contains64-bit matching loads, register-held address arithmetic and XOR/rbit/clz mismatch extraction. A 14-line current loop excerpt and 35 static candidate contexts are archived, not a dynamic execution count or a proved source-function attribution. Separately compiled optimized C and Rust Objects place checked-error origins outside their successful per-item loop and emit vector uniform fills. Both actual Objects are linked and exercised by 10 checked-sum and 48 fill controls. These analogous patterns are mechanism evidence, not a C/Rust compressor comparison. The initial eight-byte instruction-delta/tree-fill hypothesis is unestablished: its fresh PC profile is dispersed, and the reference tree is zero-filled.

## Qualification and exact limits

- **All 20 complete streams**,40 calls per original/decoded form, match exact reference bytes and independent inflate. Another40 observer-only calls establish dynamic eligibility. Input and prepared-output aliases, exact capacities, final allocation transfer and zero residual private payload/owner records pass.
- **48 original/decoded focused phase cases** exercise capacity1→2→4→8 growth, current nested reads, live original aliases, zero-trip, assertion-before-division, late assertion after append, late overflow, unchanged prior output, cleanup and success→failure→retry. The live alias first declines; after the actual first COW append, the new unique owner enters fast code. Values and expected capacities are unchanged.
- **1724 real matching controls** pass: 1,566 resident value agreements,142 unchanged default 16Mi resident-cap denials,16 ordered source-error controls, all mismatch positions/EOF/range checks, aliases, unchanged prior output, cleanup and retry. The before fixture has six packed sites; the after standalone fixture has zero and deliberately validates fallback. Its current eligibility requires an existing numeric region. Full compressor dynamic positive coverage is measured separately. No cap is increased to hide the denials.
- **40 original/decoded generic builder cases** use the real 305,257-U64 path and 13-U8 output capacities, constructors, reserve/growth/append/clear/sublist/indexed replacement/append-LE, aliases, errors, recovery and final output transfer. Four additional U8 shared-rejection/sole-transfer controls are exercised by the focused consumers.
- Seven independently generated original/decoded image pairs are byte-identical. Candidate library and linked Rust consumers use opt3, nightly 2026-03-03, Cranelift 0.131.3, aot/jit/canonical-native/program/runtime features and **warnings-as-errors**. Frozen dependency and allocator hashes plus exact commands are archived. The isolated R203 sparse-SSA frontend is unchanged; no production dependency change is made.

No fresh maintained production native suite is claimed in this docs-only round. The production compiler remains byte-identical to R199 outside retained semantic tests; R206's 677 native tests/all 260 actual CI selections,15 architecture/catalog checks and132 workflow tests are historical receipts. The new diagnostic linked tests above are actual R207 executions. Full dependency/all-features rebuilding, the separate ten-workload suite and multi-MB timing remain skipped. Ordinary exact-head hosted CI is cancelled under the standing instruction, review automation remains enabled, and #973 stays draft.

Introduced development failures are preserved and resolved: unused/private-interface compile warnings; the shared emitter's successful-body general-call refusal when real cold growth was first added; a missing import; incorrect two-argument fixture syntax; unused consumer transfer control; an overstrict alias-selection expectation; a nonzero pack-count expectation for a standalone fixture with no eligible numeric region, one import-format difference, and its verification-script argument assumption. The formatter-only source is rebuilt and the full emitted Object is compared with the exact measured Object; the pre-format source is separately retained. No warning suppression, expected-value change, benchmark-source change or allowance increase resolves them. `setup-failures.json` records exact receipts and fixes. No unresolved stream/value failure remains on the exercised controls.

The same seven inherited native exclusions remain:

```text
batched::aot::tests::aot_rejected_updates_preserve_backing_buffers_before_dispatch
batched::aot::tests::concurrent_aot_builds_publish_complete_isolated_libraries
batched::aot::tests::isa_features_shared_flags_and_abi_change_cache_identity
batched::axis_tests::gpu_matrix_solve_rejects_singular_inputs_without_publishing
batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable
batched::jit::tests::jit_rejected_updates_preserve_backing_buffers_before_dispatch
native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map
```

## Decision and remaining brief

The complete diagnostic is retained as an experiment with its exact source and contrary evidence. It still omits ordinary exact resource accounting, precise canonical error-prefix/recovery, current canonical ownership/publication and complete external admission/import. Its local failures preserve the tested previous raw output, but that is not restoration of every canonical recovery contract. This is neither ordinary NoLimits nor an established cost-of-contract floor. No ordinary guarantee is weakened in production.

R205 direct reduction completion and R206 exit-based actual accounting both have completed measured outcomes and remain unadopted. R207 now carries existing qualified phase/packed operations through the full diagnostic, but scalar-result leaf bridges and further live-state/fact reuse remain missing. The exact diagnostic root is still much slower than retained production on six cases. Do not compare omitted guarantees against production and attribute the difference solely to contract cost.

The next bounded implementation should preserve a demonstrated existing optimization in this raw root: last-use owner transfer at the hot pure array copy, or cold error-origin materialization instead of successful-path stack writes. Begin with a minimal source witness, exact live-alias/failure controls and current-source proof, then rerun the same all 20 complete-call gates/counters/profiles. Existing production private-frame last-use transfer has already landed; this is its missing diagnostic counterpart, not a newly discovered production technique. Separately measured restoration of the four ordinary contract layers follows once the computation baseline is useful.

## Review packet and replay

[Download the exact compiler patch, original/decoded controls, all 20 matched measurements, current binary findings and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r207-compact-specializations-4c556b933168.tar.gz) (2,813,174 bytes; 813 files). SHA-256 `4c556b933168dfc7d50473cbfc9734436877ef01e55ead8e1e8ccb4b228caba9`. The server digests, every archived file and fresh release download verify; `heap-summary.py` and `analyze.py` reproduce outputs byte-for-byte. The measured source and formatter-only equivalent source are both retained; the full emitted Object and metadata are byte-identical.

From an extracted packet, `python3 heap-summary.py` and `python3 analyze.py` reconstruct numerical outputs without compiling or running compression. Native reproduction requires the pinned toolchain/dependencies recorded in build specs; raw native Objects, binaries, bytecode, executable descriptors and trace stores are excluded. `candidate-source.patch` applies to the exact compiler base, and `compiler-source-pins.json` verifies all 27 changed files.
