# R325 — Use a writeback load in the reverse scalar reduction

**Retain R325 with the R272 runtime.** Across 81 cases the candidate paired elapsed change is -0.499%. The pooled and block gates pass 81/81 and 81/81. The candidate beats Roc in 46/81 primary cases (56.79%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both in the candidate.

The 17 priority cases identified before the run change -0.743% by primary paired elapsed and -0.901% in the four-actor comparison. Their primary block-mean ratios change -0.576% and -1.161%. The largest adverse all-case movement is +3.818% on one-stored.

Retention uses the established combined gates and the repeated Reymont and 256 KiB text witnesses. The full priority-group results and individual-cohort misses are reported for review.

Candidate elapsed changes -0.433% and -0.564%. Crossings below parity include none. Crossings above parity include none.

Both individual cohorts pass the block-mean gate on all cases.

Measured parent ecd961c4f707cb61f23dd4cb5cc2be99c82b92ea. Published parent 1d91c6f4644d15554633a4105a4585d6f56564fd. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R320 compiler closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Carry the current reverse address

The checked reduction already admits a forward and a reverse window. The one-candidate exit finishes after its first visit. A continuing short route carries the reverse address of the first load. A vector remainder seeds its cursor at the last vector candidate. Each subsequent scalar visit subtracts one element stride and reads the same 32-bit low word. Bounds, strict unsigned winner order, final values and resource admission follow the original reduction.

The ARM64 selector fuses an adjacent pure decrement with an admitted native-endian word load when the updated address is also live. Allocation ties the incoming and outgoing base and assigns a distinct load destination. Displaced, potentially trapping and debug-sensitive loads use the ordinary selector. Disabling cursor generation reproduces the retained Object and metadata byte for byte.

All four selected parser copies emit the writeback load. In the inspected recurring scalar loop the two loads keep their order and one address instruction goes away. The loop runs 16 instructions per continuing visit, down from 17. The reference has an eleven-instruction counted loop with a post-indexed load.

Four of 347 bodies change. The inspected parser code is 1,552 to 1,552 bytes and the frame moves from 240 to 256 bytes. Both recurring paths are free of stack accesses. Static reads in the whole parser change from 18 to 22 and static writes from 16 to 21.

## Matched results

| Input | R320 / Roc | R325 / Roc | R320 / libdeflate | R325 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.182140× | 0.181623× | 2.322590× | 2.315992× | +0.381% |
| prefix-natural-4096 | 0.972967× | 0.962622× | 1.662757× | 1.645078× | -1.589% |
| prefix-random-16384 | 1.084564× | 1.080017× | 2.920775× | 2.908531× | -0.642% |
| reymont-prefix-32769 | 1.312570× | 1.300001× | 1.197689× | 1.186219× | -1.018% |
| prefix-natural-32769 | 1.315501× | 1.302841× | 1.197020× | 1.185501× | -1.444% |
| prefix-natural-33027 | 1.317768× | 1.303519× | 1.201516× | 1.188523× | -1.160% |
| prefix-natural-32768 | 1.325325× | 1.304124× | 1.204637× | 1.185366× | -1.091% |
| prefix-natural-65537 | 1.321319× | 1.307254× | 1.109463× | 1.097654× | -0.961% |
| prefix-natural-262144 | 1.331517× | 1.321870× | 1.079355× | 1.071535× | -0.722% |
| prefix-natural-1048576 | 1.337900× | 1.328470× | 1.079916× | 1.072305× | -0.715% |
| prefix-periodic-1048576 | 0.709009× | 0.706669× | 1.201583× | 1.197617× | -0.305% |
| prefix-repeated-1048576 | 0.711252× | 0.709879× | 1.115845× | 1.113691× | -0.204% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 15-case pilot measured -0.572% by paired block-median ratios.

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 and 64 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| reymont-prefix-32769 | 1.300001× | 1.186219× | 23.08% |
| prefix-natural-32769 | 1.302841× | 1.185501× | 23.24% |
| prefix-natural-33027 | 1.303519× | 1.188523× | 23.28% |
| prefix-natural-32768 | 1.304124× | 1.185366× | 23.32% |
| heldout-window-32768 | 1.304625× | 1.183684× | 23.35% |
| heldout-window-32767 | 1.306422× | 1.186665× | 23.46% |
| prefix-natural-65537 | 1.307254× | 1.097654× | 23.50% |
| prefix-natural-32767 | 1.309101× | 1.192266× | 23.61% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R320 instructions | R325 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,889.5 | 715,947.0 | +0.008% | +1.132% |
| prefix-natural-4096 | 9,263,607.0 | 9,211,218.0 | -0.566% | -2.287% |
| prefix-random-16384 | 10,446,263.0 | 10,449,460.5 | +0.031% | -0.373% |
| reymont-prefix-32769 | 76,115,014.0 | 75,751,255.0 | -0.478% | -3.004% |
| prefix-natural-32769 | 75,958,266.0 | 75,622,639.5 | -0.442% | -0.903% |
| prefix-natural-33027 | 76,606,824.0 | 76,269,534.5 | -0.440% | -1.250% |
| prefix-natural-32768 | 75,710,746.5 | 75,373,735.5 | -0.445% | -1.112% |
| prefix-natural-65537 | 181,306,427.5 | 180,337,316.0 | -0.535% | -1.194% |
| prefix-natural-262144 | 759,135,901.5 | 755,049,544.5 | -0.538% | -0.883% |
| prefix-natural-1048576 | 2,713,295,994.0 | 2,699,578,173.5 | -0.506% | -0.807% |
| prefix-periodic-1048576 | 489,214,508.5 | 489,235,421.5 | +0.004% | -0.393% |
| prefix-repeated-1048576 | 497,047,120.0 | 497,071,112.0 | +0.005% | -0.214% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| candidate-profiles/eight-literals | 5253 | 0x1842795e9 | 421 |
| candidate-profiles/prefix-natural-262144 | 5349 | mech_compact_numeric_15296_no_growth | 3035 |
| candidate-profiles/reymont-prefix-32769 | 3481 | mech_compact_numeric_15296_no_growth | 1740 |

## Qualification

The warnings-as-errors build and 10 ordered-reduction tests pass. The component cases cover the independent scalar oracle, original and decoded input, singleton and vector tails, strict ties, wrapping, finite limits, rejection, prior publication, aliases and retry. Each backend executes 270 focused cursor calls. Both reject the unsupported non-native-endian control. The first test expected it to compile and the revised test checks the inherited rejection against the retained backend.

All 324 linked original and decoded calls agree on exact streams, independent inflate, held aliases, capacities and final cleanup. The Object, metadata, parser CLIF and VCode hashes match between the two forms.

The linked scalar route still adds the same forward-window offset before packing each winner. The profile locates that instruction at the current linked PC. The next test can carry the shared affine index for the continuing scalar route under its existing bounds proof, with the original one-candidate and vector routes.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, matched peer results and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r325-reverse-load-writeback-9561f7980d3f.tar.gz). SHA-256 9561f7980d3fd2822196670ceba9c0050bb51b51e0724203ac48c7b5648d6c53.
