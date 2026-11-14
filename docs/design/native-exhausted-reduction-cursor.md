# R351 — Reuse the exhausted reduction cursor

**Retain R350 with the R272 runtime.** The 16-case pilot missed the elapsed gates for natural 256 KiB and Reymont to advance to full timing.

Measured and published parent `51bbe5d5f7a0a4cf40624287b83c90dc24cdbcf3`. PR base `92f75db84e2523693648084bea16cd52ad60777a`. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The complete linked calls use the compact diagnostic entry, fixed level 12, runtime input, the pinned source, bytecode and allocator, and the existing output capacities and timing boundary.

## Loop completion

The existing admission proves `start < stop ≤ U32::MAX`. The final scalar successor cannot exceed stop. A successful termination comparison proves equality. The vector prefix fits inside the admitted interval and equals stop when it has no scalar tail. The singleton edge passes its original stop. These equal current values supply the same terminal cursor after the original strict cost selection, preserving wrapping, winner order and the checked fallback.

The scalar finish reuses the register that already holds the successor. The common singleton and the exact vector completion each gain an unconditional branch after their required cursor copy. The matcher and root Object code are identical.

| Linked route | R350 instructions | R351 instructions | Stack reads in each |
|---|---:|---:|---:|
| Singleton through next record | 41 | 42 | 2 |
| Two candidates through next record | 57 | 56 | 2 |
| Continuing scalar visit | 15 | 15 | 0 |
| Final scalar through next record | 16 | 15 | 0 |
| Exact vector finish through next record | 25 | 26 | 0 |

Eight of 351 bodies change. The composed body grows from 1,376 to 1,392 bytes and retains the 224-byte frame. All 4,600 calls and 170 vector decisions agree. The linked excerpts include the pinned comparison routines and the Rust scalar control. Flag-off emission reproduces R350 byte for byte.

## Paired screen

| Input | R350 / Roc | R351 / Roc | R350 / libdeflate | R351 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.159190× | 2.151729× | 0.016726× | 0.016668× | -0.346% | +0.000% |
| one-stored | 2.008864× | 2.004364× | 0.018146× | 0.018105× | -0.224% | +0.000% |
| seven-stored-prefix | 2.309015× | 2.308630× | 0.019165× | 0.019162× | -0.017% | +0.000% |
| eight-literals | 0.185279× | 0.185550× | 2.313606× | 2.316984× | +0.146% | -0.178% |
| reymont-prefix-32769 | 1.211492× | 1.219842× | 1.101991× | 1.109586× | +0.689% | +0.816% |
| heldout-window-32768 | 1.205624× | 1.217117× | 1.099790× | 1.110273× | +0.953% | +0.911% |
| prefix-natural-32767 | 1.203834× | 1.213035× | 1.099412× | 1.107816× | +0.764% | +0.874% |
| prefix-natural-32768 | 1.206794× | 1.218046× | 1.098441× | 1.108683× | +0.932% | +0.801% |
| prefix-natural-65537 | 1.211702× | 1.220790× | 1.020067× | 1.027718× | +0.750% | +0.729% |
| prefix-natural-131072 | 1.213961× | 1.223475× | 0.998678× | 1.006505× | +0.784% | +0.748% |
| prefix-natural-262144 | 1.219492× | 1.229823× | 0.984443× | 0.992783× | +0.847% | +0.731% |
| prefix-natural-1048576 | 1.233475× | 1.241098× | 0.996155× | 1.002311× | +0.618% | +0.747% |
| prefix-random-16384 | 1.014050× | 1.015324× | 2.755310× | 2.758773× | +0.126% | -1.139% |
| prefix-random-1048576 | 1.535431× | 1.536420× | 2.176581× | 2.177983× | +0.064% | -0.100% |
| prefix-periodic-1048576 | 0.687629× | 0.687880× | 1.167093× | 1.167520× | +0.037% | +0.022% |
| prefix-repeated-1048576 | 0.691562× | 0.691560× | 1.086227× | 1.086225× | -0.000% | +0.002% |

56,592 primary calls and 154,752 four-actor calls. Pooled and block 1.05 gates pass 16/16 and 16/16. The primary geometric mean changes +0.309% and the matched priority mean changes +0.522%. All six heap distributions match on 16/16 cases. The recorded screen stops before all-81 timing. R350 remains the retained all-81 result.

## Separate instruction and profile cohorts

| Input | R350 instructions | R351 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 716,281.0 | 716,301.5 | +0.003% | -0.961% |
| reymont-prefix-32769 | 64,658,328.0 | 64,751,479.5 | +0.144% | +0.608% |
| prefix-natural-131072 | 311,977,498.0 | 312,411,991.5 | +0.139% | +0.352% |
| prefix-natural-262144 | 634,024,699.5 | 634,878,062.0 | +0.135% | +0.632% |
| prefix-natural-1048576 | 2,290,168,279.0 | 2,293,097,598.0 | +0.128% | +0.824% |
| prefix-random-16384 | 9,847,896.5 | 9,849,347.5 | +0.015% | -0.252% |
| prefix-random-1048576 | 557,544,516.5 | 557,535,045.5 | -0.002% | +0.101% |

| Candidate profile input | Running timer PCs | Parser PCs | Matcher PCs |
|---|---:|---:|---:|
| eight-literals | 5,138 | 0 | 699 |
| prefix-natural-1048576 | 5,355 | 2,746 | 2,258 |
| prefix-natural-262144 | 5,341 | 2,981 | 2,028 |
| prefix-random-1048576 | 5,350 | 0 | 3,368 |
| prefix-random-16384 | 5,181 | 0 | 2,366 |
| reymont-prefix-32769 | 4,432 | 2,215 | 1,711 |

Profiles include validation and run separately from the timing and counter cohorts.

## Retained-image diagnostics

The R350 random profiles still concentrate in the matcher. One adjacent tree read/read/write/write span uses four reloads of the same table base. Keep the four memory operations and their order. The next independent address pilot can test materializing their shared effective address before the first read and keeping it in one register through both stores.

| R350 input | All running PCs | Matcher PCs | Tally PCs | Tree-pair span PCs |
|---|---:|---:|---:|---:|
| prefix-random-16384 | 5235 | 2385 | 1104 | 264 |
| prefix-random-1048576 | 5365 | 3392 | 1003 | 106 |
| prefix-natural-262144 | 5555 | 2117 | 108 | 18 |
| prefix-natural-1048576 | 5336 | 2288 | 88 | 16 |

The separate tiny diagnostic links the exact R350 Object to the existing instrumented R272 runtime. All 26 calls reconcile 111 requests, 9,024,420 requested bytes, 8,982,372 peak private payload bytes and 2,188 copied bytes. The 24 measured allocation, release and owner windows total a median 11.52 µs. The three large buffers coexist for 52.38 µs. These windows exclude later page faults and output deallocation.

## Qualification

Compiler, producer and witnesses build with warnings as errors. The 324 complete stream checks cover both identical image forms and verify reference bytes, independent inflation, aliases, capacity and cleanup. The linked witness covers 136,192 candidate and 68,096 retained positions, including zero visits, nonzero cursor starts, strict ties, wrapping costs, scalar and vector tails, source rejection and recovery. The first static audit expected the two-candidate route to tie. The linked count showed a one-instruction reduction, and the replay now checks that count. The archive preserves both performance directions.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r351-exhausted-reduction-cursor-68df2e5f7028.tar.gz). SHA-256 68df2e5f70281a3ed6fe81d8f09601d2bbeb92977e20b62f856931dc17f254ef.
