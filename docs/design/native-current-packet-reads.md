# R265: checked internal argument reads, measured and rejected

**Keep R264 as the compiler baseline.** This candidate is **1.812837% slower**
in the all-81 paired geometric elapsed comparison. Both independent primary
cohorts are adverse: A +2.055305%,
B +1.570945%.
All original pooled and block-median 1.05 gates still pass 81/81. Passing a
regression ceiling does not justify adopting an otherwise slower compiler.
The candidate's source and passing correctness evidence remain reviewable in
this round's archive; it is not the retained compiler source.

## Headline metric and frontier

**Cases faster than Roc: 22/81 = 27.1605%; goal 100%.**
The rejected candidate is also 22/81 = 27.1605%, a
+0.0000 percentage-point movement. Count a case only when its
matched paired geometric elapsed ratio is strictly below 1.0; ties do not count.
Use the same fixed inventory of 81 complete compression cases for both columns.
This is a measured configuration count, not an input-byte-weighted score or a
statistical claim that every near-boundary row is definitively faster. The
historical R264 count is also 22/81. Future headline updates must report this
percentage, its count/denominator, percentage-point change, and the 100% goal.

| Case | Bytes | Retained R264 / Roc | Rejected R265 / Roc | Time change |
|---|---:|---:|---:|---:|
| prefix-repeated-32767 | 32767 | 1.026606x | 1.060901x | +3.3406% |
| prefix-repeated-32769 | 32769 | 1.030045x | 1.059532x | +2.8627% |
| prefix-repeated-32768 | 32768 | 1.031579x | 1.066543x | +3.3893% |
| prefix-repeated-33027 | 33027 | 1.044947x | 1.072964x | +2.6812% |
| prefix-periodic-32768 | 32768 | 1.059505x | 1.098785x | +3.7074% |
| prefix-periodic-32767 | 32767 | 1.065924x | 1.102288x | +3.4115% |
| prefix-periodic-32769 | 32769 | 1.071074x | 1.102238x | +2.9097% |
| prefix-periodic-33027 | 33027 | 1.071850x | 1.107804x | +3.3544% |

The repeated/periodic 32-KiB frontier becomes worse. Natural 1 MiB improves
slightly, but that isolated gain is retained as evidence alongside the contrary
frontier and random-input results, rather than being promoted as an overall win.
Chat updates need the headline and compact frontier; the full inventory remains
below to expose regressions.

## Exact checkpoint and scope

Published parent / evaluated baseline: `9676a9cbad760dfc44f9b69aee746e1ca9080e13`
(R264). PR base: `92f75db84e2523693648084bea16cd52ad60777a`.
The parent has a clean exact-head review:
https://github.com/mech-lang/mech/pull/973#issuecomment-6778223365 .
IR/work/ABI/MBC remain **215/107/39/14**. The candidate has 827 source files:
four existing files modified and two added. A no-fuzz reverse patch restores the
exact 825-file R264 source; the forward patch reconstructs all 827 candidate
files. The R264 backend, generated selector inputs and dependency libraries are
retained byte-for-byte. The included 200-file backend reconstruction is the
already published R264 patch, not a new backend change.

R235's earlier variant was also slower. The current outlined/composed compiler
and a current unused-load witness justified one measured retest. The previous
negative result is preserved in `native-input-packet-evaluation.md`; this round
does not reinterpret it as an improvement or use its old timings as current.

## Real operation and the six-file experiment

Every synchronous compiled numeric phase reads a compiler-owned argument packet.
The actual path-scan prologue still read descriptor lanes that were overwritten
without use. The experiment carries the existing compact caller's input-word
extent into its callee, reconstructs the expected extent from the prepared
numeric proof, and permits unused safe packet reads to be deleted.

- `compact/phase_function.rs` passes its already checked input-word count into
  the private local callee contract.
- `codegen/numeric_outlined.rs` carries that extent; `readonly_numeric.rs` passes
  it and the existing optimization options to the numeric boundary.
- `codegen/numeric_minimal.rs` reconstructs entry lanes plus writable owners and
  fixed-capacity lanes, checks the declared extent, and routes packet reads
  through the small private input type.
- New `numeric_input.rs` validates checked extent arithmetic, signed transport
  offsets and each read's ordinal. Only a verified compact packet can get the
  existing `NontrappingHomes` option's nontrapping read flag. Diagnostic entry
  packets retain ordinary trapping flags. New `numeric_input_tests.rs` supplies
  the four focused controls.

The packet is initialized in the caller's checked frame, the callee has local
linkage, and calls finish synchronously before those slots can be reused. Array
helpers do not receive the packet pointer. A private cold handoff returns before
unsupported work, so it does not leave the packet live across an external
continuation. These are internal transport facts, not permissions about source
array storage. Source-array reads, bounds/error order, source work, current
arguments, owners, aliases and cleanup stay intact. No readonly, alignment,
movable-load or alias-region assertion is added. Disabling the existing option
keeps packet reads trapping. There is no new policy cap, source optimization,
benchmark-specific selector, algorithm choice or ABI revision.

## Actual code and instruction consequences

The path phase's normal and vector Object bodies each shrink **2048 to 2000
bytes**, retaining **288-byte frames**. Each now deletes ten additional packet
loads in cleanup. Across all 341 root/numeric definitions the recorded dead-load
removal count rises 657 to 1377, a static increase of 720; this is not a dynamic
call or instruction count. Exactly 214 definitions shrink, 127 remain exact,
and none grows in code or frame. The root Object body remains exact. Both
matching Object bodies remain exact, and the linked matching and path entry
addresses remain the same 64-byte-aligned addresses. The linked root moves;
exact Object bytes do not promise identical linked relocations or placement.
The worker remains **2,084,296 bytes** including padding; total recorded root/
numeric code declines 1,359,288 to 1,356,368 bytes.

The shorter path routine also changes register allocation. In the inspected
loop, the old code retains a subtraction result in a register and copies it;
the candidate writes that result to a stack slot and reloads it. This is a real
surviving spill sequence in `path-15296-before.asm` and `path-15296-after.asm`,
not a per-function hardware instruction measurement. There are other allocation
and layout changes. A complete causal instruction ledger is not established,
and the slowdown is not attributed entirely to this one sequence.

The full compressor metadata is identical after excluding cleanup statistics,
code sizes and frames. Phase geometry, headers, input/output layouts, proof
sites, source errors and owner transfers are unchanged. The compact memory
families are equal on every measured input. Safety proof and static code-size
reduction therefore do not establish a runtime improvement.

## Independent hardware and memory measurements

| Case | Instructions before | Instructions candidate | Change | Cycles before | Cycles candidate | Change |
|---|---:|---:|---:|---:|---:|---:|
| eight-literals | 1,134,714.5 | 1,134,493.5 | -0.0195% | 499,024.5 | 477,566.5 | -4.3000% |
| prefix-repeated-32768 | 27,587,669.0 | 27,651,894.0 | +0.2328% | 4,430,255.5 | 4,517,759.5 | +1.9751% |
| prefix-periodic-32768 | 27,644,489.0 | 27,708,923.0 | +0.2331% | 4,247,395.5 | 4,376,692.0 | +3.0441% |
| prefix-natural-32768 | 104,774,225.0 | 104,169,896.0 | -0.5768% | 25,481,865.5 | 25,275,708.0 | -0.8090% |
| prefix-random-32768 | 31,018,442.0 | 31,116,165.5 | +0.3150% | 6,144,312.5 | 6,333,918.0 | +3.0859% |
| prefix-natural-1048576 | 3,793,920,735.0 | 3,770,178,013.0 | -0.6258% | 974,771,816.0 | 970,559,823.0 | -0.4321% |
| prefix-random-1048576 | 958,013,220.5 | 960,998,411.5 | +0.3116% | 198,476,855.5 | 205,236,070.0 | +3.4055% |
| prefix-periodic-1048576 | 863,429,983.0 | 865,496,817.0 | +0.2394% | 122,783,998.5 | 127,651,009.0 | +3.9639% |
| prefix-repeated-1048576 | 870,670,860.5 | 872,736,092.5 | +0.2372% | 129,736,425.5 | 134,985,351.5 | +4.0458% |

Repeated and periodic 32 KiB execute about 0.23% more retired instructions,
despite removing static reads. Their cycle measurements are adverse too. Natural
1 MiB saves instructions/cycles while random 1 MiB worsens. All observations
remain in the archive. CPU, elapsed and heap observations are separate cohorts;
no CPU-sample share is multiplied by retired instructions or by another cohort.

Payload allocation requests, requested bytes, initialized copies, zeroing,
private payload peaks and final nonreturned private bytes remain equal on all
81 cases. Final nonreturned private payload and owner records are zero. This is
not an allocation, streaming or whole-worker RSS improvement. Whole-worker
physical-memory observations remain in the raw counter summaries; they include
harness/library state and are not the private payload peak.

## All 81 fresh matched complete-call timings

Two fresh primary cohorts use all six actor orders repeated four times, for
48 balanced blocks per case. Ratios are formed inside each block before
geometric aggregation. Every valid timing and counter observation is retained;
none is repeated, discarded or substituted. Both existing 1.05 gates pass 81/81.
These equally weighted configuration results are not a representative workload mix.

Both sides run fixed level 12 with runtime inputs, the pinned allocator and Rust
clock, equal 200-ms warmups and borrowed output ownership. Full compression,
scratch construction, output transfer and private cleanup remain inside the
timer. Preparation, validation and old external output lifetime remain outside
on both sides. Fresh reference denominators do not rewrite historical tables.
Stored sub-microsecond cases remain close to clock resolution.

Negative change means faster. The candidate is rejected; the left column is
still the retained compiler. Every input appears exactly once below.

### Original 25

| Case | Bytes | Retained R264 / Roc | Rejected R265 / Roc | Time change |
|---|---:|---:|---:|---:|
| empty | 0 | 2.278578x | 2.114437x | -7.2037% |
| one-stored | 1 | 2.084757x | 1.999889x | -4.0709% |
| seven-stored-prefix | 7 | 2.268882x | 2.237653x | -1.3764% |
| eight-literals | 8 | 0.422626x | 0.421959x | -0.1578% |
| eight-overlap | 8 | 0.430021x | 0.426019x | -0.9306% |
| 320-overlap | 320 | 0.454822x | 0.453900x | -0.2028% |
| eight-literals-prefix | 8 | 0.420882x | 0.424132x | +0.7723% |
| 128-biased-dynamic | 128 | 0.420532x | 0.421151x | +0.1473% |
| 32-high-entropy-stored | 32 | 0.431855x | 0.429608x | -0.5203% |
| 16384-distribution-change | 16384 | 1.877883x | 1.942828x | +3.4584% |
| 33027-periodic-window | 33027 | 1.072380x | 1.111121x | +3.6126% |
| random-64 | 64 | 0.439339x | 0.439841x | +0.1142% |
| random-256 | 256 | 0.482723x | 0.495641x | +2.6759% |
| random-1024 | 1024 | 0.542055x | 0.547502x | +1.0049% |
| repeated-1024 | 1024 | 0.521775x | 0.524057x | +0.4374% |
| random-32769 | 32769 | 1.790754x | 1.848972x | +3.2510% |
| repeated-32769 | 32769 | 1.030750x | 1.060323x | +2.8690% |
| reymont-prefix-1024 | 1024 | 0.682376x | 0.688014x | +0.8263% |
| reymont-prefix-16384 | 16384 | 1.750247x | 1.737457x | -0.7307% |
| reymont-prefix-32769 | 32769 | 1.820157x | 1.809380x | -0.5921% |
| heldout-window-32767 | 32767 | 1.821101x | 1.811354x | -0.5352% |
| heldout-window-32768 | 32768 | 1.825221x | 1.813524x | -0.6408% |
| heldout-slides-65537 | 65537 | 1.886663x | 1.877714x | -0.4743% |
| heldout-random-65537 | 65537 | 1.957299x | 2.030202x | +3.7247% |
| heldout-mixed-131073 | 131073 | 1.865431x | 1.909082x | +2.3400% |

### Controlled repeated prefixes

| Bytes | Retained R264 / Roc | Rejected R265 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.519200x | 0.527969x | +1.6889% |
| 4096 | 0.735122x | 0.752122x | +2.3126% |
| 8192 | 0.845071x | 0.868043x | +2.7184% |
| 16384 | 0.922428x | 0.959772x | +4.0485% |
| 32767 | 1.026606x | 1.060901x | +3.3406% |
| 32768 | 1.031579x | 1.066543x | +3.3893% |
| 32769 | 1.030045x | 1.059532x | +2.8627% |
| 33027 | 1.044947x | 1.072964x | +2.6812% |
| 65535 | 1.101266x | 1.143026x | +3.7920% |
| 65536 | 1.103478x | 1.143748x | +3.6493% |
| 65537 | 1.107378x | 1.142896x | +3.2074% |
| 131072 | 1.149185x | 1.193814x | +3.8835% |
| 262144 | 1.167973x | 1.217591x | +4.2482% |
| 1048576 | 1.179781x | 1.229960x | +4.2532% |

### Controlled periodic prefixes

| Bytes | Retained R264 / Roc | Rejected R265 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.530161x | 0.539851x | +1.8278% |
| 4096 | 0.753099x | 0.775980x | +3.0382% |
| 8192 | 0.849135x | 0.880450x | +3.6878% |
| 16384 | 0.957353x | 0.985497x | +2.9397% |
| 32767 | 1.065924x | 1.102288x | +3.4115% |
| 32768 | 1.059505x | 1.098785x | +3.7074% |
| 32769 | 1.071074x | 1.102238x | +2.9097% |
| 33027 | 1.071850x | 1.107804x | +3.3544% |
| 65535 | 1.139179x | 1.183263x | +3.8698% |
| 65536 | 1.140089x | 1.183546x | +3.8118% |
| 65537 | 1.140821x | 1.256693x | +10.1569% |
| 131072 | 1.172978x | 1.266514x | +7.9742% |
| 262144 | 1.207562x | 1.256157x | +4.0242% |
| 1048576 | 1.224765x | 1.272176x | +3.8710% |

### Controlled natural prefixes

| Bytes | Retained R264 / Roc | Rejected R265 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.686682x | 0.689584x | +0.4226% |
| 4096 | 1.309183x | 1.297454x | -0.8959% |
| 8192 | 1.578200x | 1.576005x | -0.1391% |
| 16384 | 1.756860x | 1.742974x | -0.7904% |
| 32767 | 1.822088x | 1.811686x | -0.5709% |
| 32768 | 1.819434x | 1.810608x | -0.4851% |
| 32769 | 1.817935x | 1.807774x | -0.5589% |
| 33027 | 1.822089x | 1.810370x | -0.6432% |
| 65535 | 1.867145x | 1.854999x | -0.6505% |
| 65536 | 1.868307x | 1.858621x | -0.5184% |
| 65537 | 1.870457x | 1.858891x | -0.6183% |
| 131072 | 1.873374x | 1.864455x | -0.4761% |
| 262144 | 1.882429x | 1.875510x | -0.3675% |
| 1048576 | 1.887367x | 1.878046x | -0.4938% |

### Controlled random prefixes

| Bytes | Retained R264 / Roc | Rejected R265 / Roc | Time change |
|---:|---:|---:|---:|
| 1024 | 0.549011x | 0.546980x | -0.3700% |
| 4096 | 0.938838x | 0.944004x | +0.5503% |
| 8192 | 1.169539x | 1.207990x | +3.2877% |
| 16384 | 1.531528x | 1.585695x | +3.5368% |
| 32767 | 1.799051x | 1.866710x | +3.7608% |
| 32768 | 1.804711x | 1.874511x | +3.8676% |
| 32769 | 1.786744x | 1.852807x | +3.6974% |
| 33027 | 1.796294x | 1.857652x | +3.4158% |
| 65535 | 1.978092x | 2.049247x | +3.5971% |
| 65536 | 1.986871x | 2.055103x | +3.4342% |
| 65537 | 1.973105x | 2.045080x | +3.6478% |
| 131072 | 2.159830x | 2.222093x | +2.8828% |
| 262144 | 2.110564x | 2.188037x | +3.6707% |
| 1048576 | 2.155342x | 2.255872x | +4.6642% |

## Profiles and next bounded step

Eight fresh candidate captures use the exact measured image: eight literals,
repeated/periodic/natural/random 32 KiB and natural/random/periodic 1 MiB. All
four timing/counter cohorts finish before the first candidate capture begins.
No build, test or profile overlaps primary or counter collection. The serialized
isolation receipt is checked in replay. R264's eight baseline and two reference
captures are reused with exact image hashes and explicit provenance; zero new
baseline/reference captures are claimed. Current PCs are resolved only against
their own image. Root source attribution remains unknown.

| Candidate case | Running samples | Matching phase | Path phase |
|---|---:|---:|---:|
| eight-literals | 5371 | 27 (0.50%) | 0 (0.00%) |
| prefix-natural-1048576 | 5336 | 1872 (35.08%) | 3094 (57.98%) |
| prefix-natural-32768 | 5508 | 1758 (31.92%) | 3140 (57.01%) |
| prefix-periodic-1048576 | 5362 | 3348 (62.44%) | 1387 (25.87%) |
| prefix-periodic-32768 | 5356 | 3068 (57.28%) | 1294 (24.16%) |
| prefix-random-1048576 | 5528 | 3120 (56.44%) | 1160 (20.98%) |
| prefix-random-32768 | 5517 | 2477 (44.90%) | 1274 (23.09%) |
| prefix-repeated-32768 | 5501 | 3038 (55.23%) | 1275 (23.18%) |

These CPU-time samples include harness and validation. They are not removable
runtime budgets or an instruction-cost partition. Matching remains the largest
border-input component; the path phase is substantial. Before another packet
read experiment, the next bounded diagnostic should map the changed path-loop
live ranges and spill sequence and count its dynamic visits, then determine
whether safe dead-lane elimination can avoid worsening live-value allocation.
Keep that distinct from globally changing live-load scheduling. The acceptance
must be matched elapsed and instructions across all 81, led by the frontier—not
static loads removed. No new semantic compiler change is started in this round.

## Qualification, errors and exact commands

- **724 native tests pass, zero fail; seven unchanged inherited exclusions.**
  Four focused input tests also pass and execute within the full suite. They
  cover extent mismatch, empty packets, overflow, last-word/off-by-one reads,
  existing transport-offset boundaries, conservative flag/option refusals,
  safe dead-load removal with live source loads retained, and fresh mutable
  arguments across 40 native calls per test execution.
- **7384 fresh compiled native source calls and 759 fresh resident calls pass.**
  The native total is 1116 checked-read calls +480 private cold-handoff calls
  +5788 matching controls. Resident totals are 279 +480; matching reuses frozen
  R230 rows and claims zero fresh resident calls. The same/different/write-alias
  fixtures cover current arguments and two-dimensional views, unselected paths,
  bounds/assertion/overflow precedence, mutable aliases, prior output,
  failure cleanup and recovery. Cold controls retain selected visits and all
  66 rejections per form. All before/after and original/decoded rows agree.
- **324 final full smokes**, 162 in each artifact form, cover all 81 exact raw
  streams, independent inflate, input/borrowed-output aliases and zero final
  private ownership. All **17** small/full Object and metadata pairs match.
- Tests use opt-level 1 and the performance compiler library opt-level 3, with
  warnings-as-errors and the same six features:
  `aot,jit,canonical-native,canonical-native-runtime,canonical-native-program,canonical-native-program-runtime`.
  Changed-module formatting and three tracked architecture/catalog checks pass.
  No expected result, source allowance, budget policy or timing gate changes.

```sh
python3 run-spec.py candidate-gpu candidate-producer candidate-tests
python3 qualify-packet-source.py
python3 qualify-cold.py
python3 qualify-matching.py
python3 run-spec.py mech-worker decoded-worker after-cpu after-memory
python3 run-measurements.py
python3 profile-border-pcs.py after
python3 replay.py
```

Exact commands, environment flags, link inputs, hashes and outcomes are in each
command receipt. The packet-control first attempt failed because an old R235
library had been cleaned up. That environmental staging failure is preserved
in the two `initial-stale-library-*` receipts and corrected to use the pinned
R264 runtime library before qualification. No measurement used a stale executable
or failed command. A read-only symbol inventory initially expected an unretained
vector alias; it was corrected to report absent linked symbols explicitly.
No introduced correctness failure remains in tested profiles. The measured
performance regression is introduced by the experimental candidate and is the
reason to reject it. The seven native exclusions remain inherited and assigned.

The unchanged maintained 680-test result is inherited from R261. Full hosted CI,
a fresh full linked/AOT integration suite, the full 6.6-MB input and a full
disabled-pipeline emission are not claimed. The compact timing entry retains its
established additional ordinary managed admission/accounting/publication/recovery
omissions. No claim that those ordinary guarantees were restored is made.

## Review contract and fidelity

Review against **R264 at `9676a9cbad760dfc44f9b69aee746e1ca9080e13`**:

1. Confirm the rejected six-file patch is the measured source closure, while
   R264 remains the retained source packet. Reverse/forward reconstruction must
   give 825/827 exact files with no fuzz and no source-program change.
2. Check the internal packet span, initialized storage, local synchronous
   lifetime and private cold-return argument; reject readonly/alignment/movable
   or source-array permissions not established by that proof. Validate negative
   extent/option tests and current arguments, aliases, error order, cleanup and retry.
3. Recompute the strict faster-than-reference metric on the unchanged 81-case
   inventory (22/81 =27.160494%, goal100%) and the 48 matched blocks per case.
   The candidate must remain rejected despite passing the 1.05 ceilings; preserve
   natural-text wins as well as frontier/random regressions and adverse counters.
4. Verify candidate/baseline/reference hashes, matched level/input/ownership/
   warmup/timer boundaries, original/decoded images, cohort/profile isolation,
   inherited coverage labels, and public packet replay. The spill sequence is
   a witness, not a complete causal decomposition of the slowdown.

Benchmark source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
MBC14 SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Preserve the complete level-12 algorithm and tables, depth300, nice258, original
EOF rules, 262144 U16 tree, 1500513 U32 cache, 305257 U64 path capacity, output
capacity13 and clear/reuse, aliases, allocation sites, pinned allocator and
complete-call timing boundary. Original/decoded outputs remain exact.

The archive contains the evaluated 827-file compiler, its reversible six-file
patch, the unchanged R264 200-file backend reconstruction, small fixture metadata,
commands, all observations, counters and aggregate profile PCs. Native binaries/
libraries/Objects, bytecode, full compressor metadata/IR/CFG/maps and raw trace/
XML/stacks are excluded. Five offline reports replay without compiling or
compressing. PR973 remains the only active optimization PR and stays draft.
Parity is open: the retained headline is27.160494%, and the target is100%.

## Public evidence and replay

[Rejected compiler patch, all81 measurements, commands and validation](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r265-current-packet-reads-d54abac422bb.tar.gz).
SHA-256: `d54abac422bbceeb97ad0bb02f3d61ddf086f893a4465b0c1c42f67178ce1696`.
Fresh public retrieval verifies 2,358 file hashes, exact 827/825 compiler and 200-file
backend reconstruction, and five deterministic reports. The rejected candidate
is retained as evidence; R264 remains the active compiler source packet.

```sh
python3 benchmarks/mech-deflate/experiment.py replay-round \
  --index benchmarks/mech-deflate/experiments.json --round 265 \
  --output /tmp/mech-r265-review
```
