# R316 — Carry child addresses through the native tree walk

**Retain R315 with the R272 runtime.** Across 81 cases the candidate paired elapsed change is +0.482%. The pooled and block gates pass 81/81 and 81/81. The candidate beats Roc in 46/81 primary cases (56.79%). The matched four-actor run records 3/81 faster than libdeflate and 0/81 faster than both in the candidate.

Both primary cohorts favor the retained compiler. Candidate elapsed changes +0.600% and +0.365%. The greatest adverse paired movement is +2.834% on empty. Crossings below parity include none. Crossings above parity include none.

Both individual cohorts pass the block-mean gate on all cases.

Measured parent 78eaf501bf8d899dc4e058a9bce6580871630864. Published parent 78eaf501bf8d899dc4e058a9bce6580871630864. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R315 closure and attached patch. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The pinned source, bytecode, allocator, tables, limits, buffers and full-call timing boundaries are identical.

## Carry the loaded child address into the next visit

A block argument may carry the byte address instead of its element index when each use is a compatible wrapping scaled address with the same invariant base. The compiler checks all incoming and outgoing arguments and retains any class with a scalar use, shared expression, changing base or incompatible address. It reuses an already available address or computes an equivalent seed only when the replacement removes more address calculations than it adds.

The compiler rewrites 12 store operands in the vector matcher. All four tree descent backedges carry the already computed lookup address. The original typed loads and stores, their order, source checks and loop argument counts are unchanged. All four generated stores address through the carried pointer. Both search directions lose the table-base reload immediately before their stores. Both skip directions still reload saved addresses from the stack.

The exact linked libdeflate executable loads through the newly calculated child address. The current ARM64 selector instead still folds base and index into each of the four Mech lookups, even though the complete address is computed for the next iteration. The exact Roc executable carries indices and holds the tree base in x30. The retained Mech matcher and path parser save the link register, restore it before each authenticated return, and make zero calls. The next pilot will test register allocation in an already framed leaf while preserving the saved return address and excluding lowered uses of the link register. The duplicate-address selector remains a separate lead.

1 of 347 bodies change. The vector matcher body changes from 12,776 to 12,768 bytes and its stack frame from 640 to 640 bytes. Static stack reads change from 547 to 551. The sampled stack-load share inside the matcher changes from 24.64% to 28.65%. These are running-sample shares from separate profiling runs.

## Matched results

| Input | R315 / Roc | R316 / Roc | R315 / libdeflate | R316 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.181342× | 0.181776× | 2.321878× | 2.327430× | +0.067% |
| prefix-natural-4096 | 0.982333× | 0.973444× | 1.670232× | 1.655119× | -0.234% |
| prefix-random-16384 | 1.084962× | 1.087406× | 2.951368× | 2.958015× | -0.335% |
| heldout-window-32768 | 1.328781× | 1.325128× | 1.211483× | 1.208152× | -0.148% |
| reymont-prefix-32769 | 1.329802× | 1.325456× | 1.213437× | 1.209472× | -0.281% |
| prefix-natural-32769 | 1.330624× | 1.327784× | 1.212484× | 1.209896× | -0.249% |
| prefix-natural-32768 | 1.336861× | 1.333004× | 1.214620× | 1.211115× | -0.299% |
| prefix-natural-65537 | 1.342941× | 1.339247× | 1.128424× | 1.125320× | -0.257% |
| prefix-natural-262144 | 1.353486× | 1.346831× | 1.100848× | 1.095436× | -0.360% |
| prefix-natural-1048576 | 1.372799× | 1.366529× | 1.105684× | 1.100634× | -0.362% |
| prefix-periodic-1048576 | 0.709492× | 0.719282× | 1.199253× | 1.215800× | +1.419% |
| prefix-repeated-1048576 | 0.709285× | 0.718663× | 1.118825× | 1.133619× | +1.351% |

The separate four-actor cohort has 245,664 timed calls and exact raw stream agreement across all 81 cases. Each libdeflate call includes a fresh level-12 compressor. The 15-case pilot measured -0.124% by paired block-median ratios.

## Cases where Roc is the faster peer

Roc is faster than libdeflate in 17/81 matched cases. The nearest substantial inputs are natural text around 32 and 64 KiB. The frontier uses the same balanced blocks for all three ratios.

| Input | Mech / Roc | Mech / libdeflate | Elapsed cut to Roc |
|---|---:|---:|---:|
| heldout-window-32768 | 1.328781× | 1.211483× | 24.74% |
| reymont-prefix-32769 | 1.329802× | 1.213437× | 24.80% |
| prefix-natural-32769 | 1.330624× | 1.212484× | 24.85% |
| prefix-natural-32767 | 1.331689× | 1.213456× | 24.91% |
| heldout-window-32767 | 1.334026× | 1.213987× | 25.04% |
| prefix-natural-33027 | 1.335442× | 1.217548× | 25.12% |
| prefix-natural-32768 | 1.336861× | 1.214620× | 25.20% |
| prefix-natural-65537 | 1.342941× | 1.128424× | 25.54% |

## Independent counters and profiles

The six Mech payload and heap families agree across all 81 cases. CPU counter measurements and profile samples were collected in separate runs. The profile includes validation.

| Input | R315 instructions | R316 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 716,185.5 | 716,175.5 | -0.001% | +1.119% |
| prefix-natural-4096 | 9,275,399.0 | 9,247,728.5 | -0.298% | -0.684% |
| prefix-random-16384 | 10,496,603.0 | 10,471,066.0 | -0.243% | -0.578% |
| heldout-window-32768 | 76,080,247.0 | 75,779,138.5 | -0.396% | -0.437% |
| reymont-prefix-32769 | 76,330,710.5 | 76,026,377.5 | -0.399% | -0.521% |
| prefix-natural-32769 | 76,327,880.0 | 76,026,538.5 | -0.395% | -0.354% |
| prefix-natural-32768 | 76,078,801.0 | 75,778,275.0 | -0.395% | -0.344% |
| prefix-natural-65537 | 182,371,059.5 | 181,674,237.5 | -0.382% | -0.263% |
| prefix-natural-262144 | 764,589,154.0 | 761,528,020.5 | -0.400% | -0.294% |
| prefix-natural-1048576 | 2,739,419,611.0 | 2,726,872,273.5 | -0.458% | -0.315% |
| prefix-periodic-1048576 | 488,170,652.5 | 492,309,633.0 | +0.848% | +1.444% |
| prefix-repeated-1048576 | 496,004,497.5 | 500,119,621.5 | +0.830% | +1.302% |

| Profile | Running samples | Leading linked leaf | Samples |
|---|---:|---|---:|
| candidate-profiles-screen/eight-literals | 5227 | mech_compact_closed_v2 | 1623 |
| candidate-profiles-screen/prefix-natural-262144 | 5345 | mech_compact_numeric_15296_no_growth | 3017 |
| candidate-profiles-screen/reymont-prefix-32769 | 4115 | mech_compact_numeric_15296_no_growth | 2050 |

## Qualification

The compiler and focused Rust tests use warnings as errors. Fifty-five CFG tests pass. The focused control executes 180 before/after JIT pairs across six use shapes, wrapping indices and both reached and rejected loop visits. The 324 linked original and decoded calls agree on exact bytes, independent inflate, held aliases, capacities and final managed cleanup. Both linked forms have identical Object, metadata and matcher instruction dumps.

The initial compile exposed an ambiguous set type. Inserting copied SSA values into the set resolves it. A full local disk interrupted the first profile export after the capture had finished. Re-exporting the same capture and finishing the other profiles succeeded. The original elapsed, CPU and heap observations remain in the packet.

The replay checks all four balanced cohorts and the 81 matched cases from raw per-call samples. It regenerates the timing gates, hardware counters, dual-peer frontier and this report.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r316-child-address-c83157433c0f.tar.gz). SHA-256 c83157433c0f586c8fd95a514306560e7109793f8f7df639e8c46e6aafa5d7d8.
