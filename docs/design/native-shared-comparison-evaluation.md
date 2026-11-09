# R238: shared physical comparison flags — qualified, not adopted

Base: `f92415a31dbf0398ae641e66c419f435a3e3cb7c` (R237). Retained experimental compiler remains R237. Ordinary compiler stays R215 `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; IR/work/ABI/MBC215/107/39/14. No source algorithm or ordinary contract change.

## Mechanism and generated code

The real path argmin emitted the same physical integer comparison twice around a conditional select. An isolated two-file Cranelift0.131.3 AArch64 emitter patch retains flags only for an exact physical-register/width match across adjacent conditional selects that preserve both operands. Block boundaries, every other instruction, operand overwrite, nested pseudo expansion, unoptimized mode and pending user stack maps invalidate or disable reuse. No post-link edit, source name or compressor-specific rule is used.

All820 GPU source files remain byte-identical to R237. A distinct, unmodified backend rebuilt from the pinned dependency slice reproduces R237 Object and metadata byte-for-byte. The initially unpinned dependency build and invalid shared-target control are excluded and documented. C emits one comparison/two selections; the retained Rust min control emits two comparisons. The fresh reference PC capture confirms its actual corresponding cost loop is hot:2,805/5,349 Running/Timer-Fired PCs;729 samples land on its second select. These are sampled-time counts, not per-function retired instructions. Independent inflate appears in this diagnostic cohort outside the primary timer and is not compression work.

Only four path bodies change;132 numeric/map bodies and root text remain byte-identical. Each selected body loses one static comparison (486→485 instructions including padding), with unchanged loads/branches. Code alignment leaves its1952 B body size unchanged. Root text5647820 B and stack55168 B remain unchanged. The actual hot sequence is now cmp w8,w7; csel x15,x8,x7,lo; csel x5,x28,x5,lo.

## Decision and all25 complete calls

The two predeclared24-balanced-block cohorts are retained. The first cohort passes25 point gates but only23 block gates: empty1.06178047166 and one-stored1.05567103385 exceed1.05. Confirmation and pooled data pass25/25. The candidate is archived without replacing R237. Pooled equal-case geometric elapsed improves0.654960%; the11-large-case descriptive geometric change is-0.620860%. The new brief's prospective decision rule was not retroactively applied.

| Input | Candidate µs | Reference µs | R237 / reference | Candidate / reference | Block elapsed change | Instructions change |
|---|---:|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 9314.624 | 4372.666 | 2.143186× | 2.130193× | -0.6062% | -0.6594% |
| 33027-periodic-window | 2181.854 | 1262.000 | 1.737586× | 1.728886× | -0.5007% | -0.0025% |
| repeated-32769 | 2197.833 | 1346.958 | 1.642064× | 1.631701× | -0.6311% | -0.0025% |
| random-32769 | 3033.041 | 1069.876 | 2.848891× | 2.834948× | -0.4894% | +0.0001% |
| 16384-distribution-change | 2278.188 | 832.688 | 2.776802× | 2.735945× | -1.4713% | -0.0768% |
| eight-literals | 143.542 | 289.917 | 0.505603× | 0.495114× | -2.0558% | -0.0018% |
| reymont-prefix-16384 | 4498.917 | 2164.333 | 2.093717× | 2.078662× | -0.7190% | -0.6756% |
| empty | 0.083 | 0.042 | 1.976190× | 1.976190× | +2.0666% | +0.0000% |
| one-stored | 0.084 | 0.042 | 1.976190× | 2.000000× | +1.8241% | +0.0000% |
| seven-stored-prefix | 0.084 | 0.042 | 2.000000× | 2.000000× | -0.8283% | +0.0000% |
| eight-overlap | 144.167 | 290.208 | 0.502799× | 0.496770× | -0.3246% | -0.0049% |
| 320-overlap | 172.041 | 301.105 | 0.577389× | 0.571366× | -0.0903% | +0.0019% |
| eight-literals-prefix | 146.708 | 288.250 | 0.510047× | 0.508961× | -0.8679% | -0.0042% |
| 128-biased-dynamic | 146.917 | 289.750 | 0.521068× | 0.507047× | -1.9800% | -0.0044% |
| 32-high-entropy-stored | 152.792 | 290.125 | 0.535976× | 0.526642× | -1.7622% | -0.0021% |
| random-64 | 162.292 | 292.542 | 0.563594× | 0.554763× | -1.2638% | -0.0004% |
| random-256 | 196.250 | 299.562 | 0.661658× | 0.655122× | -0.4699% | -0.0010% |
| random-1024 | 230.916 | 309.792 | 0.755815× | 0.745390× | -0.9142% | -0.0005% |
| repeated-1024 | 218.666 | 329.833 | 0.672563× | 0.662960× | -1.7751% | +0.0002% |
| reymont-prefix-1024 | 307.375 | 336.041 | 0.923003× | 0.914695× | -0.9989% | -0.2678% |
| heldout-window-32767 | 9294.416 | 4366.208 | 2.142911× | 2.128716× | -0.6624% | -0.6607% |
| heldout-window-32768 | 9295.062 | 4363.854 | 2.145490× | 2.130012× | -0.7214% | -0.6607% |
| heldout-slides-65537 | 19648.687 | 8840.292 | 2.230293× | 2.222629× | -0.3436% | -0.6270% |
| heldout-random-65537 | 6150.562 | 1942.229 | 3.174778× | 3.166755× | -0.2527% | -0.0002% |
| heldout-mixed-131073 | 21197.105 | 8396.708 | 2.535262× | 2.524454× | -0.4263% | -0.2737% |

Instructions decrease on19 cases, tie on3 and increase on3; cycles decrease on23 and increase on2. Natural32 has127,905,201.5→127,061,759 instructions (−0.659428%) and0.606241% lower elapsed. Random32 instructions are effectively unchanged. The eight-byte elapsed reduction cannot be attributed to this comparison removal: its instruction median is effectively unchanged. No cohort percentages are multiplied.

All25 payload allocation counts, requested bytes, initialized copies, zeroed bytes and payload peaks match R237. Whole-call request/byte/reallocation/incremental-usable-peak medians match too. No memory or allocation win is claimed.

## Validation, interruptions and remaining coverage

703 maintained native tests pass with -D warnings and the unchanged aot,jit,canonical-native,canonical-native-runtime,canonical-native-program,canonical-native-program-runtime feature profile and same seven documented exclusions. All10,680 original/decoded native source controls pass;1,183 resident calls are fresh, while older path/matching resident rows remain hash-pinned reuse with34 unchanged default-work denials. All24 small original/decoded Object/metadata pairs agree. Five new backend controls plus eight existing emitter controls pass. Each of baseline, candidate and unmodified-control executes51,840 independent-oracle scalar checks (155,520 total); disabled Objects and unmodified-control optimized Objects match. Existing7 dispatch,4 low-word,6 read-reuse and wide-loop qualification are retained.

Complete primary observations:324,444; retained timings:324,144; separate counter calls:3,900. Fifty all-input smokes precede primary timing. All calls retain exact bytes, independent inflate, held aliases, expected capacities and zero final private ownership. No build/test/profile overlaps primary or counter cohorts. Wall/PC profiles complete25,351/105,933 correct calls; reference profile is a separate bounded diagnostic.

Environmental disk exhaustion interrupted confirmation while one-stored was partially recorded. All eight completed cases were kept; only the incomplete case resumed after a written plan, without inspecting gates or changing compiler/protocol. Its35,561 extra completed calls are preserved outside the324,444 complete cohort count; truncated transport is preserved too. A later PC capture hit the same disk limit: five completed profiles stayed unchanged, incomplete eight-byte data was preserved, and only that capture resumed. Exact stdout/receipt/partial data and setup-failures.json remain available. No compiler allowance was raised. Lossless older-log compression recovered3.37 GB. The first resumed wrapper used a wrong relative script path and failed before native work; the corrected absolute invocation passes.

Maintained layout/native-build-analysis checks, changed-backend formatting and dry-run patch application pass. The experimental owner_flow file/directory collision is inherited byte-identically; ordinary integration remains assigned separately. The previous head external review returned an unknown service error, not a clean review. Hosted CI is cancelled under standing instruction and PR973 stays draft. Full dependency/all-features rebuild, multi-MB inputs, the separate workload suite and ordinary complete-compressor timing remain unclaimed.

This standalone experiment still omits ordinary exact resource/admission/accounting/canonical-publication/recovery guarantees. Source algorithms, original pass limits, depth300,nice258, near-EOF rules,262144-U16 tree,1500513-U32 cache,305257-U64 path capacity, cap13 output clear/reuse, allocation sites, pinned allocator, aliases and full timing boundary are unchanged. R225 source bindings/R235 packet flags stay uncombined.

## Next bounded work

Follow the new brief: aggregate diagnostic census, collector private-view continuity, then direct path-reduction completion. Compare prospectively against R234 as main large-input control, keeping R237 as a separately reported retained variant. Predeclare all11 large inputs as primary, family reports, fixed confirmation, and all25 regression checks. Whole-phase append reserve and comparison-result reuse remain separate experiments; no speculative growth or algorithm change.

## Pins

- base: `f92415a31dbf0398ae641e66c419f435a3e3cb7c`
- retained_baseline: `R237`
- normal_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- compiler_source_manifest_sha256: `7df27c42ed87d7d5e41f3b11a408806c2a16019c928e2875da550fe0b07e842a`
- backend_source_manifest_sha256: `b330370226856d833eaac3e935c6fc87213263ba83e308f20b3c63aa86359fd6`
- compiler_patch_sha256: `884d22b6f8332f3577b66fdc1998058c6a646d652fd47b8f46cee63bbae2362f`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- object_sha256: `5fbfed46f7f9dfb3696f01547aaf81fc0cca76c1a19212ba089d4a7b88da3aee`
- parent_object_sha256: `fd60bd590984c843b521de7e68b2e2926027976f9a637554eb2b8e095eeecca9`
- worker_sha256: `47ff7f3c43b0bba70102f8665df2b6b7bf7c4d4b0b879c4af0260fad8fda20b3`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`
- compressor_source_unchanged: `True`
- all_820_GPU_files_byte_identical_to_R237: `True`
- R225_source_bindings_uncombined: `True`
- R235_input_packet_flags_uncombined: `True`

External evidence retains both complete cohorts, partial observations, the two-file backend patch/full changed-source closure, all source controls, profiles/assembly and numerical replay. Native binaries, bytecode, full metadata/descriptors and raw traces/stack samples are excluded. `analyze.py` replays results and heap summaries without compiling or executing native code.

## Public evidence

[Complete evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r238-shared-condition-2f37869d022d.tar.gz) (`sha256:2f37869d022d985cebf5b1e706c881117a51b9d6d0eaf8381ad55af3d4ae7c1a`, 7914785 bytes). Server digest, full download, all 2,370 hashes and byte-identical numerical replay verify.
