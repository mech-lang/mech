# R293 — Reuse the caller’s flat-view proof in packed reads

**Retain R293 with the R272 runtime.** Candidate paired elapsed changes -0.923% across 81 fixed level-12 cases. The pooled and block gates pass 81/81 and 81/81. R292 beats Roc on 45/81 cases and R293 beats it on 45/81 (55.56%). The separate four-actor cohort gives R293 3/81 cases below libdeflate.

The two primary cohorts change elapsed by -0.966% and -0.881%. The largest paired slowdown is +1.322% on prefix-periodic-1048576. Cases crossing below 1× Roc are none. Cases crossing above are none.

Pooled gate misses are none. Block gate misses are none.

Published and measured parent 3c70684d55aa921ead329567d7d87d7f5d5ad4f5. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R292 closure and a one-file compiler patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, inputs, tables, capacities, allocator runtime and complete-call timer are pinned.

## Current proof and generated code

The compact caller checks the exact current view before selecting its vector body. The packed-read emitter now consumes that contract for an immutable, fixed view when the reached source window proof authorizes the read. A complete pack within a cloned loop must still be confined to that loop. The raw load replaces only the admitted little-endian byte construction. Source range checks, defaults, reached failures and publication stay in the same order. Rectangular views decline vector admission. The caller retains its general native body and its earlier alias fallback.

One of 347 machine bodies changes. Its code shrinks from 26,520 to 14,308 bytes and its frame from 880 to 800 bytes. The source IR retains 29 packed loads and 17 source width guards. Current row and column comparison pairs fall from 25 to 8. The general matcher, parsers and entry retain their Object instruction bytes.

| Vector matcher instruction class | R292 | R293 |
|---|---:|---:|
| Stack loads | 987 | 734 |
| Stack stores | 204 | 146 |
| Other loads and stores | 914 | 467 |

The exact linked reference loops use flat buffers. The Roc excerpt compares current scalar bounds and uses paired word reads with no shape dispatch. The libdeflate excerpt uses indexed word loads from its two bases. A separate safe Rust witness slices both ranges before reading them. The packet includes the pinned ARM64 excerpts and source for that witness.

The 27-case screen changes paired elapsed by -0.286%. It records 0 pooled misses and one block miss on empty. Both pooled empty medians are 83 ns. The original block miss and every screen observation stay in the packet. Full acceptance uses the unchanged 81-case gates.

## Headline matched ratios

| Input | R292 / Roc | R293 / Roc | R292 / libdeflate | R293 / libdeflate | R293 / R292 elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.196668× | 0.196441× | 2.516293× | 2.513395× | +0.034% |
| prefix-random-8192 | 0.840841× | 0.809658× | 3.051630× | 2.938457× | -4.931% |
| prefix-natural-4096 | 1.059592× | 1.040108× | 1.820329× | 1.786857× | -1.570% |
| prefix-natural-32768 | 1.438761× | 1.417344× | 1.309116× | 1.289629× | -1.323% |
| prefix-natural-1048576 | 1.469640× | 1.452843× | 1.186922× | 1.173356× | -1.121% |
| prefix-random-1048576 | 1.768916× | 1.707633× | 2.504031× | 2.417281× | -3.498% |
| prefix-periodic-1048576 | 0.811491× | 0.823094× | 1.373233× | 1.392868× | +1.322% |
| prefix-repeated-1048576 | 0.804555× | 0.814135× | 1.264046× | 1.279097× | +0.943% |

The four-actor cohort contains 245,664 measured calls. The reference ratios use its same-case balanced blocks. The paired elapsed-change column and parity score use the two primary cohorts.

## Qualification and independent counters

- All 324 complete original and decoded calls produce the expected byte stream and independently inflate. Capacities and held aliases agree. The two forms have identical Objects and metadata. Both end with zero live private payload and owners.
- Four linked images each pass 1,440 focused cases. Row and column vectors execute the selected native variant. Rectangular inputs execute the general path. The oracle models the one-based source order independently and checks short/defaulting reads, full-width indices, held aliases, prior publication on rejection and cleanup.
- The 87 direct parser controls and 44 cleanup tests pass with warnings as errors. The existing packed-window test rejects missing, stale, wrapping and different-view proofs. The 43,923 window and 3,840 continuation calls pass.
- The logical payload count, requested bytes, copy and zeroing counters, peak private payload and outstanding private bytes agree for all 81 cases.

Two diagnostic harness corrections precede the final controls. The linked entry uses an opaque heap pointer at the ABI boundary. The observer counts only the actual phase header and dispatch events. The receipts include the original compilation and observer errors.

| Input | R292 instructions | R293 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 778,455.5 | 778,429.5 | -0.003% | -1.759% |
| prefix-random-8192 | 6,163,276.0 | 5,845,279.5 | -5.160% | -3.681% |
| prefix-natural-4096 | 10,338,316.5 | 10,103,375.0 | -2.273% | -1.418% |
| prefix-natural-32768 | 87,423,042.0 | 85,099,788.0 | -2.657% | -1.237% |
| prefix-natural-1048576 | 3,182,104,820.0 | 3,095,953,989.0 | -2.707% | -1.100% |
| prefix-random-1048576 | 706,989,884.5 | 659,309,538.5 | -6.744% | -3.287% |
| prefix-periodic-1048576 | 581,648,403.0 | 581,440,555.5 | -0.036% | +1.291% |
| prefix-repeated-1048576 | 589,072,731.0 | 588,858,959.0 | -0.036% | +1.150% |

The natural 1 MiB process lifetime peak physical footprint moves from 54,576,064 to 54,576,064 bytes. This includes the harness.

Timing directions differ between cohorts on 5 cases. The packet retains both observations.

## Exact executable profiles

The CPU-time profiles include harness and validation samples. The counts are locations observed during sampling, independently of the retired-instruction and cycle cohorts.

| Input | Running samples before / after | Vector matcher before / after | Matcher stack loads before / after | Matcher stack stores before / after |
|---|---:|---:|---:|---:|
| eight-literals | 5450 / 5366 | 643 / 667 | 17 / 10 | 2 / 2 |
| prefix-natural-1048576 | 5511 / 5380 | 2370 / 2273 | 1059 / 808 | 140 / 59 |
| prefix-natural-32768 | 5339 / 5375 | 2086 / 2044 | 835 / 692 | 180 / 79 |
| prefix-natural-4096 | 5521 / 5579 | 1636 / 1537 | 526 / 468 | 158 / 88 |
| prefix-periodic-1048576 | 5510 / 5366 | 3785 / 3693 | 1556 / 1995 | 225 / 207 |
| prefix-random-1048576 | 5386 / 5361 | 3426 / 3374 | 1301 / 1491 | 635 / 486 |
| prefix-repeated-1048576 | 5371 / 5365 | 3439 / 3430 | 1453 / 1863 | 222 / 235 |

The first removed geometry route had 21 samples on the old linked 1 MiB random worker. The sampled locations in the smaller body are mapped independently. Its most sampled stack load slot is offset 488 with 724 samples across the matcher.

The eight current geometry pairs remain in captured-window admission. Those gates already consume current scalar bounds and overflow facts. A gate can use the same flat-view fact for its immutable capture while keeping the scalar checks and general body available. The next candidate targets that entry operation.

The closest candidate case above parity is prefix-natural-4096 at 1.045576× Roc. Reuse a proved current flat view at captured-window entry. The eight remaining row and column tests are part of captured-window admission. Keep current lengths, offset checks, source fallback and funding. Measure the selected-body result separately.

The timing scope is the experimental compact entry. Ordinary managed integration remains open.

## All primary timings

| Input | Bytes | R292 / Roc | R293 / Roc | R293 / R292 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.118460× | 2.119201× | 1.000350× | pass/pass |
| one-stored | 1 | 2.023415× | 2.038577× | 1.007493× | pass/pass |
| seven-stored-prefix | 7 | 2.239658× | 2.245187× | 1.002469× | pass/pass |
| eight-literals | 8 | 0.195618× | 0.195685× | 1.000345× | pass/pass |
| eight-overlap | 8 | 0.186677× | 0.185437× | 0.993361× | pass/pass |
| 320-overlap | 320 | 0.215046× | 0.214367× | 0.996845× | pass/pass |
| eight-literals-prefix | 8 | 0.198710× | 0.196796× | 0.990372× | pass/pass |
| 128-biased-dynamic | 128 | 0.257823× | 0.255568× | 0.991255× | pass/pass |
| 32-high-entropy-stored | 32 | 0.264708× | 0.262864× | 0.993034× | pass/pass |
| 16384-distribution-change | 16384 | 1.500200× | 1.460850× | 0.973770× | pass/pass |
| 33027-periodic-window | 33027 | 0.718957× | 0.723615× | 1.006478× | pass/pass |
| random-64 | 64 | 0.278013× | 0.276108× | 0.993150× | pass/pass |
| random-256 | 256 | 0.336995× | 0.333531× | 0.989721× | pass/pass |
| random-1024 | 1024 | 0.398110× | 0.393507× | 0.988438× | pass/pass |
| repeated-1024 | 1024 | 0.264072× | 0.264529× | 1.001730× | pass/pass |
| random-32769 | 32769 | 1.427792× | 1.371728× | 0.960734× | pass/pass |
| repeated-32769 | 32769 | 0.703504× | 0.708498× | 1.007098× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.518967× | 0.509591× | 0.981934× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.383109× | 1.364042× | 0.986214× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.436939× | 1.416041× | 0.985456× | pass/pass |
| heldout-window-32767 | 32767 | 1.439100× | 1.418225× | 0.985494× | pass/pass |
| heldout-window-32768 | 32768 | 1.438799× | 1.419631× | 0.986678× | pass/pass |
| heldout-slides-65537 | 65537 | 1.472292× | 1.454692× | 0.988046× | pass/pass |
| heldout-random-65537 | 65537 | 1.588617× | 1.524146× | 0.959417× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.425905× | 1.404784× | 0.985188× | pass/pass |
| prefix-natural-1024 | 1024 | 0.514665× | 0.506379× | 0.983899× | pass/pass |
| prefix-natural-4096 | 4096 | 1.062248× | 1.045576× | 0.984304× | pass/pass |
| prefix-natural-8192 | 8192 | 1.272362× | 1.253217× | 0.984953× | pass/pass |
| prefix-natural-16384 | 16384 | 1.387236× | 1.365142× | 0.984073× | pass/pass |
| prefix-natural-32767 | 32767 | 1.436412× | 1.417564× | 0.986879× | pass/pass |
| prefix-natural-32768 | 32768 | 1.438247× | 1.419224× | 0.986774× | pass/pass |
| prefix-natural-32769 | 32769 | 1.442111× | 1.421910× | 0.985992× | pass/pass |
| prefix-natural-33027 | 33027 | 1.440764× | 1.419597× | 0.985308× | pass/pass |
| prefix-natural-65535 | 65535 | 1.446484× | 1.429002× | 0.987914× | pass/pass |
| prefix-natural-65536 | 65536 | 1.448389× | 1.429263× | 0.986795× | pass/pass |
| prefix-natural-65537 | 65537 | 1.442000× | 1.422254× | 0.986306× | pass/pass |
| prefix-natural-131072 | 131072 | 1.449120× | 1.430394× | 0.987078× | pass/pass |
| prefix-natural-262144 | 262144 | 1.453988× | 1.442144× | 0.991854× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.471992× | 1.455485× | 0.988786× | pass/pass |
| prefix-random-1024 | 1024 | 0.407052× | 0.401208× | 0.985642× | pass/pass |
| prefix-random-4096 | 4096 | 0.701762× | 0.674831× | 0.961623× | pass/pass |
| prefix-random-8192 | 8192 | 0.869951× | 0.827055× | 0.950691× | pass/pass |
| prefix-random-16384 | 16384 | 1.198887× | 1.149205× | 0.958560× | pass/pass |
| prefix-random-32767 | 32767 | 1.447465× | 1.390077× | 0.960353× | pass/pass |
| prefix-random-32768 | 32768 | 1.444935× | 1.388690× | 0.961074× | pass/pass |
| prefix-random-32769 | 32769 | 1.442976× | 1.380754× | 0.956879× | pass/pass |
| prefix-random-33027 | 33027 | 1.448569× | 1.392402× | 0.961226× | pass/pass |
| prefix-random-65535 | 65535 | 1.621461× | 1.556056× | 0.959663× | pass/pass |
| prefix-random-65536 | 65536 | 1.624508× | 1.564085× | 0.962805× | pass/pass |
| prefix-random-65537 | 65537 | 1.614382× | 1.554230× | 0.962739× | pass/pass |
| prefix-random-131072 | 131072 | 1.707602× | 1.660622× | 0.972488× | pass/pass |
| prefix-random-262144 | 262144 | 1.757396× | 1.691618× | 0.962571× | pass/pass |
| prefix-random-1048576 | 1048576 | 1.773589× | 1.711553× | 0.965022× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.331401× | 0.330667× | 0.997787× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.454385× | 0.456611× | 1.004897× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.537232× | 0.542561× | 1.009920× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.626144× | 0.631983× | 1.009324× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.700014× | 0.707707× | 1.010990× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.706012× | 0.710224× | 1.005966× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.711652× | 0.717168× | 1.007750× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.722937× | 0.725973× | 1.004199× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.760047× | 0.766102× | 1.007967× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.764384× | 0.768436× | 1.005301× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.762617× | 0.769173× | 1.008597× | pass/pass |
| prefix-periodic-131072 | 131072 | 0.791091× | 0.800708× | 1.012156× | pass/pass |
| prefix-periodic-262144 | 262144 | 0.803032× | 0.813615× | 1.013178× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 0.810095× | 0.820804× | 1.013219× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.261795× | 0.261980× | 1.000705× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.405812× | 0.406198× | 1.000951× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.494031× | 0.496755× | 1.005514× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.595782× | 0.601487× | 1.009575× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.677840× | 0.684361× | 1.009620× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.676955× | 0.685095× | 1.012025× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.702550× | 0.708345× | 1.008249× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.719676× | 0.722721× | 1.004231× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.755099× | 0.760918× | 1.007706× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.759533× | 0.765517× | 1.007878× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.756549× | 0.763258× | 1.008868× | pass/pass |
| prefix-repeated-131072 | 131072 | 0.788970× | 0.797516× | 1.010833× | pass/pass |
| prefix-repeated-262144 | 262144 | 0.804564× | 0.812836× | 1.010282× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 0.804985× | 0.812578× | 1.009432× | pass/pass |

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r293-flat-packed-reads-d99feb9eddd7.tar.gz). SHA-256 d99feb9eddd7f0a3e37a3d1242c8f40e02837ac67ea151c65bd74bebd106991b.
