# R354 — Combine native exit predicates

**Retain R350 with the R272 runtime.**

Measured compiler head `51bbe5d5f7a0a4cf40624287b83c90dc24cdbcf3`. Publication parent `b4d567fc345d6dbbe4f2154a2b7911befb5ccc22`. PR base `92f75db84e2523693648084bea16cd52ad60777a`. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The timed workers use the existing compact diagnostic entry and the shared startup descriptor. Source, bytecode, allocator, output capacities and complete-call timing boundaries are pinned.

## Conditional compare in the linked matcher

The ARM64 selector combines an OR of two canonical scalar comparison results when one comparison is equality at full W or X register width. The first comparison supplies flags and the inverse of its effective condition gates the second. A skipped comparison forces Z. A single CSET returns the canonical Boolean for the existing consumers. A checked immediate conversion admits 0–31 and the general register form handles other equality operands. The source branches, effect order and continuation merge are unchanged.

Both search and skip exit spans use 7 instructions, down from 9. The new sequence contains CMP, CCMP and one CSET. UXTB and both downstream tests are still present. Roc combines the exit checks in flags and keeps the tree base and depth in registers. libdeflate uses a conditional compare with carried child pointers. The candidate still has recurring stack traffic. The vector matcher shrinks by 100 code bytes and retains its 624-byte frame. The selector changes 33 of 351 bodies, including a 19,580-byte root code increase. All 4,600 calls and all vector selections agree.

## Paired screen

| Input | R350 / Roc | R354 / Roc | R350 / libdeflate | R354 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.098348× | 2.033639× | 0.016865× | 0.016345× | -3.084% | +0.000% |
| one-stored | 2.073795× | 2.008769× | 0.018371× | 0.017795× | -3.136% | +0.000% |
| seven-stored-prefix | 2.261227× | 2.199815× | 0.018842× | 0.018330× | -2.716% | +0.000% |
| eight-literals | 0.183070× | 0.183561× | 2.317241× | 2.323454× | +0.268% | -0.120% |
| reymont-prefix-32769 | 1.206596× | 1.212706× | 1.099237× | 1.104804× | +0.506% | +0.466% |
| heldout-window-32768 | 1.206918× | 1.211426× | 1.098385× | 1.102488× | +0.374% | +0.499% |
| prefix-natural-32767 | 1.208822× | 1.211927× | 1.099936× | 1.102762× | +0.257% | +0.359% |
| prefix-natural-32768 | 1.207847× | 1.212262× | 1.099274× | 1.103292× | +0.366% | +0.135% |
| prefix-natural-65537 | 1.208296× | 1.213972× | 1.020152× | 1.024945× | +0.470% | +0.401% |
| prefix-natural-131072 | 1.213648× | 1.218645× | 1.001164× | 1.005286× | +0.412% | +0.162% |
| prefix-natural-262144 | 1.222020× | 1.227315× | 0.985651× | 0.989922× | +0.433% | +0.365% |
| prefix-natural-1048576 | 1.233829× | 1.239580× | 0.996070× | 1.000713× | +0.466% | +0.372% |
| prefix-random-16384 | 1.020408× | 1.019406× | 2.747524× | 2.744826× | -0.098% | +0.647% |
| prefix-random-1048576 | 1.538385× | 1.534549× | 2.173221× | 2.167802× | -0.249% | -0.482% |
| prefix-periodic-1048576 | 0.688106× | 0.677546× | 1.169135× | 1.151191× | -1.535% | -1.645% |
| prefix-repeated-1048576 | 0.691592× | 0.686326× | 1.087199× | 1.078921× | -0.761% | -1.477% |

56,592 primary calls and 154,752 balanced four-actor calls. The 1.05 gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -0.022%. The matched priority mean changes -0.525%. All six heap distributions match on 16/16 cases.

## Separate hardware counters and profile

| Input | R350 instructions | R354 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 716,399.5 | 715,478.5 | -0.129% | -0.406% |
| reymont-prefix-32769 | 64,658,582.5 | 64,142,115.5 | -0.799% | +0.097% |
| prefix-natural-131072 | 311,995,861.5 | 309,275,352.5 | -0.872% | -0.152% |
| prefix-natural-262144 | 634,032,780.0 | 628,432,715.0 | -0.883% | -0.276% |
| prefix-natural-1048576 | 2,290,278,554.5 | 2,267,254,016.0 | -1.005% | -0.260% |
| prefix-random-16384 | 9,847,883.5 | 9,743,682.0 | -1.058% | -0.543% |
| prefix-random-1048576 | 557,572,041.5 | 550,580,658.5 | -1.254% | -1.323% |

| Input | Running timer PCs | Vector matcher PCs | Parser PCs |
|---|---:|---:|---:|
| eight-literals | 5196 | 649 | 0 |
| prefix-natural-1048576 | 5358 | 2301 | 2722 |
| prefix-natural-262144 | 5354 | 2042 | 2969 |
| prefix-random-1048576 | 5353 | 3364 | 0 |
| prefix-random-16384 | 5240 | 2355 | 0 |
| reymont-prefix-32769 | 4513 | 1761 | 2230 |

Profiles include validation. Hardware counters and sampling use separate cohorts.

## Qualification

Warnings-as-errors builds pass. The two selector binaries execute 14,000 focused cases covering both register widths, mirrored and shared predicates, immediate boundaries and an arbitrary-word fallback. Both image forms agree byte for byte and pass 324 complete stream checks with exact bytes, independent inflation, capacities, aliases and cleanup. The exact retained compiler re-emits its original Object and metadata. The archive preserves the first selector generation failure and its correction.

[Verified backend patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r354-conditional-exit-5773b3dbaec3.tar.gz). SHA-256 5773b3dbaec3214a21ea347ba38039bf21faf2b6d33c98a7650dcfdc8fdc3579.
