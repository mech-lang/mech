# R255: reuse a current native integer comparison

**Retain the experimental backend change for its instruction reduction.** Text 1 MiB retires 31.8 million fewer instructions (-0.6974%), while matched elapsed is essentially flat (+0.0809%). Do not present this as a substantial text speedup. Every unchanged 1.05 timing gate passes in both confirmations; all 81 exact outputs and independent inflation checks pass. Larger-input parity remains incomplete.

Git parent `66b787f3370de40909ab0022ffe8da32d558f945`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Ordinary R215 remains `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC 215/107/39/14. All 822 Mech compiler files and the complete compressor source/bytecode stay exact R253. The only compiler source changes are three paths in pinned Cranelift 0.131.3: one experimental Cargo feature, the AArch64 emitter and a small tested comparison tracker. No LLVM backend is adopted.

## Actual missed work and the bounded fix

R254 located a hot scalar cost-reduction loop with the same comparison before both cost and winning-index selection. Fresh C/Rust assembly witnesses share that comparison. Replacing umin with two selects sharing the source predicate produces identical old backend code, so a source-IR spelling change cannot remove it. The standalone baseline demonstrates the duplicate at both opt none and speed.

With experimental-cmp-reuse enabled, the backend omits a duplicate flag-only SUBS comparison after register allocation only when the same physical input registers and width remain current, a conditional selection intervenes, and no input was overwritten. Nonoverwriting moves and the exact U32-to-U64 move retain the proof. Different width/order/registers, data-producing arithmetic and every other instruction are barriers. Each basic block starts fresh. Unknown multi-instruction expansions suppress tracking for their entire body, including internal labels. No operation moves; memory access, ownership, logical aliases, funding, errors and return values retain their original implementation. Original code/resource estimates stay conservative.

Final native probes cover 4,608 direct choice calls and 54,208 hazard calls across enabled/disabled backends and opt none/speed: raw high bits, unsigned/signed choices, different widths/order, overwritten values, a flag-clobbering call and branch joins. Eight focused tracker tests pass. These establish correctness of this bounded transformation, not a whole-program timing prediction.

## Whole-program identity and placement controls

The backend is rebuilt with the feature on and off, using frozen dependencies and the earlier sparse-SSA frontend improvement. The disabled full compressor restores the entire R253 Object and metadata byte for byte. Final source annotations restore the entire measured enabled Object and metadata. All seven small original/decoded source-image pairs are exact.

Exactly four of 136 original routines change: headers 12419, 15296, 26895 and 31086. Each scalar reduction loses one comparison; padding keeps each reported routine at 2,048 bytes. Root code, the other 132 routines and seven selected vector bodies are byte-identical. All phase metadata is exact, and the root remains 940,264 bytes with a 50,560-byte frame. The worker remains 1,884,504 bytes. All 142 functions retained by the linker start at the same addresses; linked root bytes are exact. Two metadata-only functions are stripped on both sides. This is an instruction-count improvement, not a measured code-size, stack or allocation improvement.

## Complete-call matched results

Two fixed primary cohorts each use 24 balanced blocks, all six control/candidate/reference orders repeated four times and 200 ms warmup per actor/case. Level 12, runtime inputs, live borrowed output, scratch construction, output transfer and private cleanup match. Validation/logging remain outside. CPU and heap are independent cohorts. No build, test, profile or diagnostic compressor overlaps any primary or counter cohort.

Ratios below use pooled observation medians against the same fresh Roc denominator; changes use balanced block means. Historical denominators stay in their original reports. Source algorithm, tables, depth 300, nice length 258, near-EOF rules, tree/cache/path capacities, allocation sites, allocator and timing endpoints remain pinned.

| Case | R253 / Roc | R255 / Roc | Elapsed change | Cohort A | Cohort B | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.431510x | 0.434532x | +0.4137% | +4.0459% | +0.0588% | +0.0000% | -0.6185% |
| heldout-random-65537 | 2.797955x | 2.781194x | -0.5990% | -0.6615% | -0.4880% | +0.0004% | -0.4486% |
| prefix-natural-32768 | 2.133446x | 2.127455x | -0.2808% | -0.1623% | -0.3356% | -0.6618% | -0.0364% |
| prefix-random-32768 | 2.543242x | 2.544112x | +0.0342% | -0.3268% | +0.2641% | -0.0002% | +0.0947% |
| prefix-periodic-32768 | 1.432484x | 1.424933x | -0.5271% | -0.1799% | -0.5919% | -0.0028% | -0.7964% |
| prefix-repeated-32768 | 1.370750x | 1.370144x | -0.0443% | +0.1306% | -0.3202% | -0.0041% | -0.3352% |
| prefix-natural-1048576 | 2.172846x | 2.174604x | +0.0809% | +0.0838% | +0.1004% | -0.6974% | -0.1078% |
| prefix-random-1048576 | 3.086275x | 3.070799x | -0.5014% | -0.5447% | -0.4890% | -0.0870% | -0.4940% |
| prefix-periodic-1048576 | 1.693067x | 1.691775x | -0.0763% | +0.0296% | -0.1311% | -0.0030% | +0.0403% |
| prefix-repeated-1048576 | 1.585071x | 1.582587x | -0.1567% | -0.0338% | -0.2115% | -0.0037% | -0.1321% |

All 81 descriptive geometric elapsed change is -0.1774%: 56 improve and 25 worsen. Every point and balanced gate passes in both cohorts. Eight remains below 1x, with contrary confirmations preserved; its instruction medians are equal. Text 1 MiB is slightly slower in both elapsed confirmations despite fewer instructions. Random 1 MiB improves about 0.50% elapsed, but its independent instruction change is much smaller; do not attribute all of that elapsed difference to the removed comparison.

Across the independent counter cohort, 66 instruction medians fall, 11 rise and four tie; 60 cycle medians fall and 21 rise. All 81 payload requests, requested bytes, initialized copying, zeroing, private peaks and listed whole-heap medians remain identical between Mech variants. Requested capacity, initialized bytes copied and OS RSS remain separate quantities. Whole-call instruction deltas are not a per-function or omitted-comparison execution ledger.

## Fresh current-image profile

| Case | Running samples | Root | Numeric phases | Other/external |
|---|---:|---:|---:|---:|---:|
| eight-literals | 5351 | 990 | 691 | 3670 |
| prefix-natural-1048576 | 5354 | 887 | 4419 | 48 |
| prefix-periodic-1048576 | 5355 | 2199 | 3083 | 73 |
| prefix-random-1048576 | 5634 | 3440 | 2159 | 35 |

The exact current-image mapper bounds symbol attribution to executable text and retains aggregate per-phase PCs. Narrow current disassembly verifies the removed comparison. Root state transport still dominates random samples, while existing numeric phases dominate text. Samples include harness/validation and are neither a retired-instruction partition nor removable-time budgets. The R254 external-address label repair remains in use. The pinned reference binary and prior independent C/Rust scalar controls remain unchanged.

## Validation and failure disposition

Fresh maintained qualification: 712 passed, zero failed/ignored, same seven inherited exclusions. Backend tracker: eight passed. Final native probes: 58,816 calls. Source controls: 3,500 original/decoded calls including 700 zero trips across six programs/seven variants; current values, aliases, errors, publication, cleanup and retry agree. Complete smokes: 162 calls/all 81 cases. Instrumented confirmation: another 162 calls/all 81 exact R253 phase counts. Every timing/counter/profile call verifies exact compressed bytes, independent inflate, input/output aliases, capacity and zero final private payload/owners. Instrumentation is absent from timing workers.

Warnings-as-errors remain enabled for all rebuilt compiler libraries, dependencies, probes and workers. Compiler libraries use opt3; the maintained test root uses opt1. The six native features and pinned nightly-2026-03-03 on Apple ARM64 remain. The previous extended matrix/buffer/boundary controls are inherited, not relabeled fresh. Full enabled/disabled decoded compressor images were not emitted; seven source pairs and maintained tests cover decoded behavior.

Corrected setup/diagnostic failures: a missing space in the initial probe declaration; dependency selection stopped on multiple frozen hashbrown/libc artifacts until exact dependency metadata was inspected; the initial linked-symbol audit included two normally stripped functions. None was a compression failure. The initial conservative tracker deliberately declined through the zero-extension move; the final explicitly certified move retains flags and passes the final probes. No introduced unresolved semantic failure or increased allowance remains. All adverse timing and counter observations are retained.

The seven maintained exclusions, ordinary extra managed-contract omissions and enclosing child-funding/selection refusal remain inherited. Ordinary IR215 admission/accounting/publication/recovery stays unchanged. Compact aliases/cleanup are not restoration of that additional managed contract. This experimental backend build feature is not enabled in the ordinary maintained distribution. No green hosted CI is claimed; cancel only exact-head CI under the standing instruction and keep the PR draft pending green CI and clean exact-head review.

## Reproduction and next bounded work

The packet includes the upstream source package, exact three-file backend patch/manifests, unchanged generated definitions, retained sparse frontend source, frozen dependency choices and complete command arrays. Rebuilds preserve the declared feature profiles. Source verification reconstructs the backend and validates the unchanged 822-file Mech closure from the R253 packet. Numerical replay regenerates five reports without a compiler or compressor. Native executables/Objects, bytecode, full compressor descriptors/IR/maps and raw traces are excluded.

This closes one small backend opportunity without establishing a large time reduction. Move the main effort back to state transport: recover the exact enclosing recurrence funding/cold-handoff requirement and the remaining empty outer reduction path. Preserve the guard, all 81 controls and the eight-byte ceiling. The single R247 placement control remains separate and open; phase-only isolation and the rejected SIMD cutoff experiment must not be repeated.

## All 81 configurations

| Case | Bytes | R253 / Roc | R255 / Roc | Balanced change | Instructions change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -5.4878% | +0.0000% |
| one-stored | 1 | 1.976190x | 1.976190x | -1.2829% | +0.0000% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | -0.1105% | +0.0000% |
| eight-literals | 8 | 0.431510x | 0.434532x | +0.4137% | +0.0000% |
| eight-overlap | 8 | 0.429237x | 0.437646x | +1.9227% | -0.0013% |
| 320-overlap | 320 | 0.488720x | 0.491495x | +0.3227% | -0.0030% |
| eight-literals-prefix | 8 | 0.428324x | 0.441553x | +1.1018% | +0.0000% |
| 128-biased-dynamic | 128 | 0.435443x | 0.435588x | -0.2427% | -0.0124% |
| 32-high-entropy-stored | 32 | 0.446701x | 0.450853x | +0.1834% | -0.0018% |
| 16384-distribution-change | 16384 | 2.351248x | 2.331244x | -0.8508% | -0.0789% |
| 33027-periodic-window | 33027 | 1.437834x | 1.426207x | -0.8086% | -0.0028% |
| random-64 | 64 | 0.461296x | 0.470429x | +2.5398% | -0.0013% |
| random-256 | 256 | 0.543337x | 0.549964x | +1.6390% | -0.0011% |
| random-1024 | 1024 | 0.642684x | 0.649841x | +0.9291% | +0.0017% |
| repeated-1024 | 1024 | 0.567953x | 0.571373x | +0.4921% | +0.0045% |
| random-32769 | 32769 | 2.515242x | 2.514559x | -0.0271% | -0.0003% |
| repeated-32769 | 32769 | 1.369651x | 1.367773x | -0.1372% | -0.0029% |
| reymont-prefix-1024 | 1024 | 0.809739x | 0.809736x | +0.0335% | -0.2761% |
| reymont-prefix-16384 | 16384 | 2.059503x | 2.050391x | -0.4425% | -0.6741% |
| reymont-prefix-32769 | 32769 | 2.134637x | 2.129041x | -0.2621% | -0.6618% |
| heldout-window-32767 | 32767 | 2.136502x | 2.128159x | -0.3905% | -0.6647% |
| heldout-window-32768 | 32768 | 2.135478x | 2.127728x | -0.3629% | -0.6664% |
| heldout-slides-65537 | 65537 | 2.210322x | 2.205205x | -0.2315% | -0.6058% |
| heldout-random-65537 | 65537 | 2.797955x | 2.781194x | -0.5990% | +0.0004% |
| heldout-mixed-131073 | 131073 | 2.271093x | 2.272213x | +0.0493% | -0.2818% |
| prefix-natural-1024 | 1024 | 0.819083x | 0.820394x | -0.1762% | -0.2773% |
| prefix-natural-4096 | 4096 | 1.564692x | 1.542794x | -1.3995% | -0.6865% |
| prefix-natural-8192 | 8192 | 1.863595x | 1.860608x | -0.1603% | -0.7147% |
| prefix-natural-16384 | 16384 | 2.064067x | 2.047143x | -0.8200% | -0.6678% |
| prefix-natural-32767 | 32767 | 2.139014x | 2.131957x | -0.3299% | -0.6661% |
| prefix-natural-32768 | 32768 | 2.133446x | 2.127455x | -0.2808% | -0.6618% |
| prefix-natural-32769 | 32769 | 2.131820x | 2.128181x | -0.1707% | -0.6615% |
| prefix-natural-33027 | 33027 | 2.132253x | 2.128295x | -0.1856% | -0.6662% |
| prefix-natural-65535 | 65535 | 2.130423x | 2.128858x | -0.0735% | -0.7306% |
| prefix-natural-65536 | 65536 | 2.126442x | 2.124778x | -0.0782% | -0.7314% |
| prefix-natural-65537 | 65537 | 2.157687x | 2.127389x | -1.4042% | -1.0571% |
| prefix-natural-131072 | 131072 | 2.126584x | 2.129745x | +0.1486% | -0.7570% |
| prefix-natural-262144 | 262144 | 2.137119x | 2.140011x | +0.1353% | -0.7681% |
| prefix-natural-1048576 | 1048576 | 2.172846x | 2.174604x | +0.0809% | -0.6974% |
| prefix-random-1024 | 1024 | 0.652985x | 0.656630x | +0.4637% | +0.0019% |
| prefix-random-4096 | 4096 | 1.221196x | 1.221582x | +0.0316% | -0.0007% |
| prefix-random-8192 | 8192 | 1.586681x | 1.610085x | +1.4750% | -0.0001% |
| prefix-random-16384 | 16384 | 2.127065x | 2.118671x | -0.3946% | +0.0000% |
| prefix-random-32767 | 32767 | 2.535862x | 2.539888x | +0.1588% | -0.0003% |
| prefix-random-32768 | 32768 | 2.543242x | 2.544112x | +0.0342% | -0.0002% |
| prefix-random-32769 | 32769 | 2.509991x | 2.505762x | -0.1685% | +0.0005% |
| prefix-random-33027 | 33027 | 2.536627x | 2.532534x | -0.1614% | +0.0005% |
| prefix-random-65535 | 65535 | 2.800753x | 2.791049x | -0.3465% | +0.0038% |
| prefix-random-65536 | 65536 | 2.805868x | 2.792846x | -0.4641% | +0.0003% |
| prefix-random-65537 | 65537 | 2.798431x | 2.780617x | -0.6365% | +0.0006% |
| prefix-random-131072 | 131072 | 2.927314x | 2.927954x | +0.0219% | -0.0005% |
| prefix-random-262144 | 262144 | 2.996320x | 2.995264x | -0.0352% | -0.0002% |
| prefix-random-1048576 | 1048576 | 3.086275x | 3.070799x | -0.5014% | -0.0870% |
| prefix-periodic-1024 | 1024 | 0.575065x | 0.579668x | +0.0366% | -0.0200% |
| prefix-periodic-4096 | 4096 | 0.895778x | 0.882739x | -1.4557% | -0.0106% |
| prefix-periodic-8192 | 8192 | 1.077191x | 1.073716x | -0.3226% | -0.0077% |
| prefix-periodic-16384 | 16384 | 1.258795x | 1.253892x | -0.3895% | -0.0027% |
| prefix-periodic-32767 | 32767 | 1.434478x | 1.428904x | -0.3886% | -0.0028% |
| prefix-periodic-32768 | 32768 | 1.432484x | 1.424933x | -0.5271% | -0.0028% |
| prefix-periodic-32769 | 32769 | 1.431303x | 1.428118x | -0.2225% | -0.0028% |
| prefix-periodic-33027 | 33027 | 1.435375x | 1.425971x | -0.6552% | -0.0027% |
| prefix-periodic-65535 | 65535 | 1.555393x | 1.552524x | -0.1845% | -0.0032% |
| prefix-periodic-65536 | 65536 | 1.553665x | 1.551618x | -0.1317% | -0.0032% |
| prefix-periodic-65537 | 65537 | 1.550408x | 1.548151x | -0.1456% | -0.0033% |
| prefix-periodic-131072 | 131072 | 1.627421x | 1.622137x | -0.3247% | -0.0028% |
| prefix-periodic-262144 | 262144 | 1.662838x | 1.665242x | +0.1445% | -0.0028% |
| prefix-periodic-1048576 | 1048576 | 1.693067x | 1.691775x | -0.0763% | -0.0030% |
| prefix-repeated-1024 | 1024 | 0.568261x | 0.562690x | -0.1971% | -0.0024% |
| prefix-repeated-4096 | 4096 | 0.865119x | 0.866064x | +0.1092% | -0.0107% |
| prefix-repeated-8192 | 8192 | 1.043351x | 1.040479x | -0.2753% | -0.0077% |
| prefix-repeated-16384 | 16384 | 1.220106x | 1.220539x | +0.0355% | -0.0027% |
| prefix-repeated-32767 | 32767 | 1.366338x | 1.364688x | -0.1208% | -0.0028% |
| prefix-repeated-32768 | 32768 | 1.370750x | 1.370144x | -0.0443% | -0.0041% |
| prefix-repeated-32769 | 32769 | 1.374246x | 1.369132x | -0.3721% | -0.0041% |
| prefix-repeated-33027 | 33027 | 1.375017x | 1.370911x | -0.2986% | -0.0041% |
| prefix-repeated-65535 | 65535 | 1.481564x | 1.472921x | -0.5834% | -0.0029% |
| prefix-repeated-65536 | 65536 | 1.478125x | 1.475253x | -0.1943% | -0.0075% |
| prefix-repeated-65537 | 65537 | 1.476846x | 1.472387x | -0.3019% | -0.0028% |
| prefix-repeated-131072 | 131072 | 1.544740x | 1.541676x | -0.1984% | -0.0048% |
| prefix-repeated-262144 | 262144 | 1.573771x | 1.574080x | +0.0197% | -0.0027% |
| prefix-repeated-1048576 | 1048576 | 1.585071x | 1.582587x | -0.1567% | -0.0037% |

## Public evidence

[Evidence packet](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r255-shared-compare-dc222e201c8a.tar.gz), 18,393,131 bytes, SHA-256 `dc222e201c8a92f8f5bdd4baffeb0674a164585c8cd4befc73051e821dae55ec`. Downloaded public bytes verify all 1,366 file hashes, five byte-identical numerical reports, exact reconstruction of the three-file backend patch, and all 822 unchanged Mech compiler source files. Source/package/generated-definition/frontend manifests and exact build commands are included. Native executables/Objects, bytecode, complete compressor descriptors/IR/maps and raw traces are excluded.
