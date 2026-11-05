# Native compression milestone

**R153 retains one native reentry capability. It is locally qualified for draft review; parity is not achieved.** Earlier published heads remain frozen.

| Checkpoint | Revision |
|---|---|
| Published base | #982 at `9de07a5a339ef9044133fdd96b644f0279b91781` |
| Evidence base | `6741fccacd5241f02c9b5379f2754c2f0ea1bf67` |
| Before compiler | `dd3253562ee9d33451a66f4ef9b9808577dc4c1e` |
| Measured compiler | `2b6953e9a8532bae417588d607a553f9c8d86a6d` |
| Final compiler, feature gate repaired | `2d27e4cc44261a02c6858a232960d8999ac712eb` |
| Final focused tests | `cfc684c329f843bb7f373b602664b2db5f2c158e` |
| IR / work / ABI / bytecode | **177 / 107 / 39 / 14**; old IR175/176 rejected |

The measured operation is `bt-skip-loop` with its `extend-four`, `extend-words` and `extend-tail` children after the periodic input slides its window. The partial parent already contained their native implementation, but ancestor-first source ownership hid their independent entries. After `CollectionBegin` handed execution to the ordinary suffix, these supported loops stayed in the runtime until the original parent header. This patch adds complete child entries through the existing checked preparation, shared emitter, current admission and independent image reconstruction. Skip retains no cache argument or recording path; search remains separate.

The periodic case reaches the new skip entry **39 times**. Its four loop bodies lose **all** measured admission/library/segment helper calls, including **758 library calls**. Whole-call runtime ABI crossings fall **21,586→5,110**. Fuel exits **4→4**, unsupported exits **1→1** and first-change transitions **138→138** show that the target was continuation coverage, not repeated fuel exhaustion. The actual Object's emitter report has direct U8/U16 loads and U16 stores, no cold exits and zero successful-backedge helpers for this entry. Counts are from separate diagnostic cohorts, three identical complete calls per case and side; they are not timings or per-loop instruction counts.

Primary source membership stays outermost. A secondary entry requires a complete cold-free proof contained in a primary with a real unsupported exit, an exact current primary continuation, no active loan and a conservative proof that its root/descendants are unpaid. It uses the existing single loan and shared numeric scratch. Error settlement and first-change preparation use the actual active entry, preserving the outer cursor, tracked child return, current aliases, full charges, cleanup, atomic publication and retry. There is no new interpreter, permission cache, growing-child feature or binding recipe.

The final 20-case cohort has **3,968 primary / 4,048 total complete calls**, exact reference bytes, independent inflate, input aliases, prefixes, capacities and zero final managed storage. Elapsed improves **1.874060% geometrically**: **12 faster / 8 slower** medians, worst **+1.589571%**, all unchanged **1.05** gates pass. The earlier four-case development screen remains separate. These are fresh matched comparisons, not a causal comparison with earlier rounds' absolute timings.

| Input | Current ms | Mech / Roc before→after | Change |
|---|---:|---:|---:|
|eight-literals|1.347938|4.7059×→4.6966×|-0.1974%|
|16384-distribution-change|7.068771|8.2443×→8.3753×|+1.5896%|
|33027-periodic-window|9.477417|8.0827×→7.1989×|-10.9337%|
|random-32769|7.136896|7.7524×→6.6887×|-13.7201%|
|repeated-32769|8.497166|7.1109×→6.3506×|-10.6913%|
|reymont-prefix-32769|33.604312|7.9728×→7.7074×|-3.3286%|
|reymont-prefix-16384|18.297124|8.6106×→8.5922×|-0.2135%|
|random-1024|1.255250|4.0700×→4.0755×|+0.1347%|
|repeated-1024|1.557959|4.8128×→4.8384×|+0.5310%|
|reymont-prefix-1024|1.908750|5.7241×→5.7148×|-0.1624%|
|320-overlap|1.341104|4.5665×→4.5616×|-0.1086%|
|random-256|1.329458|4.4263×→4.4538×|+0.6212%|
|128-biased-dynamic|1.146000|3.9812×→3.9654×|-0.3965%|
|random-64|1.261875|4.4041×→4.4122×|+0.1836%|
|32-high-entropy-stored|1.243041|4.2643×→4.2863×|+0.5172%|
|eight-overlap|1.321417|4.5402×→4.5724×|+0.7082%|
|eight-literals-prefix|1.349208|4.6775×→4.6685×|-0.1911%|
|seven-stored-prefix|0.120375|clock-limited reference|-0.0174%|
|one-stored|0.120313|clock-limited reference|-0.0175%|
|empty|0.116625|clock-limited reference|+0.2325%|

Hardware counters and heap counts are separate complete-call cohorts across all 20 cases. Periodic instructions **188,585,999.5→167,092,396 (−11.397242%)** and cycles **32,751,640.5→28,604,240 (−12.663184%)** fall. Its requests **3,640→3,679 (+39)** and requested bytes **15,777,466→15,798,298 (+20,832)** increase; incremental usable heap peak stays **15,314,928 B**. Loaded descriptor/static usable storage grows 154,464 B; requested bytes, live usable peak and OS footprint remain distinct metrics. Repeated instructions fall 9.015079%; random32K falls 17.265763%. Eight-byte instructions instead rise 0.130197% and cycles 0.640240%, with 3,354 requests unchanged and 240 more requested bytes. Contrary results are retained, with no universal allocation or instruction win claimed.

| Fresh CPU-time samples | Generated code | Host callbacks/cleanup | Outside |
|---|---:|---:|---:|
|16384-distribution-change|69.01%|29.61%|1.39%|
|reymont-prefix-16384|82.42%|15.99%|1.59%|
|repeated-32769|85.89%|12.11%|2.00%|
|33027-periodic-window|83.92%|14.88%|1.20%|

Samples describe time contexts, not operation coverage or retired-instruction shares. Periodic's retained before profile was 73.56% generated/24.32% host; repeated was 75.33%/22.82%. Distribution's callback counts are unchanged, so its descriptive sample-share difference is not attributed to new coverage. No per-function retired-instruction ledger is claimed.

The tradeoff is **8,726,056→9,118,504 B generated text (+392,448)** and **12,192→15,872 B entry stack (+3,680)**. Cold restoration, outside `evaluate_into`, was **34.792→37.283 s** in the screen and **34.572→37.705 s** in the final cohort. The roughly 35-second baseline is inherited; the observed additional 2.5–3.1seconds is an introduced cost. These are two observed setup pairs, not a dedicated repeated matched startup experiment. No whole-command speedup is claimed. Fresh Object generation was 143.475 s versus the earlier recorded 135.363 s; separate builds are descriptive.

Policy107 formulas and finite limits remain unchanged, but logical work is **not equal across compiler inventories/routes**: periodic 266,361,097,312→258,345,086,116; eight-byte 18,119,266,561→18,119,470,149. Each side is stable, and the before side matches its retained pin. Source sites and current-account charging remain. A copied R151 scope string initially mislabeled the new matrix as IR175/work-neutral; the immutable original packets and explicit label-audit packet preserve that mistake and correction. No observation, allowance or numeric result was edited.

Qualification: **580 GPU pass / 7 same exclusions**, six focused tests, **155 CI tests listed/equality-guarded/executed**,132 workflow checks,15 architecture checks, runtime-only and formatting pass. Unchanged **236 component /8 composition /1 real-capacity path** controls pass. **Scalar 12** pass (maximum 1.003698 after/before, eight adverse medians, no allocation increase); **frame 8** pass with no request/byte/work increase. Nightly 2026-03-03/aarch64, warnings denied, library opt3 and unit/component roots opt1; Core/Engine/math/dependencies frozen and rehashed. CI inventory 149→155 is a separate commit.

The focused tests cover real ordinary→secondary→first-change→completion→fresh-parent execution, original/decoded/direct/restored artifacts, exact selected bounds/arithmetic error precedence, success→failure→retry, unchanged publication and aliases, six finite resource dimensions, successful zero/post-prefix fuel settlement, rejected first-change admission after loan settlement, and nine image mutations plus old-version rejection. The clean parent reproduces missing native coverage without a source-meaning change. The read-only reviewer found both final settlement/admission controls closed and no further blocker.

An introduced runtime-only unused-method warning is fixed with the codegen feature gate, retaining warnings-as-errors. A fresh build from the repaired revision reproduces **Object, descriptor, metadata and source/MBC byte-for-byte**. All linked instructions and addresses also match; only 24 Rust panic-source line fields shift by5 after a test-only helper insertion. This preserves the measured executable claim without substituting a favorable timing rerun. Earlier development fixture/access/overbroad-selection and test error-class failures are fixed and archived. Disk-full packaging and restricted local tooling were environmental and resolved; 127 obsolete native builds freed 12,789,367,160 file bytes while preserving current/reference artifacts and evidence.

No unresolved introduced functional failure remains. Same seven profile exclusions, full dependency/all-features rebuild, multi-MB timing and separate ten-workload suite remain skipped. Ordinary hosted CI is cancelled under standing instruction; **draft until green hosted CI and clean external review**. The next implementation is not part of this round. Nested append, binding recipes, startup-analysis reuse and unrelated source changes remain separate candidates requiring fresh attribution.

R152's guarded subtraction remains **rejected and reverted**: its eight-byte +5.821563% observation failed the unchanged 1.05 gate. Its candidate/test/revert history and 33 published packets remain in the [experiment index](../../benchmarks/mech-deflate/experiments.json); IR176 stays reserved. R151 and earlier retained results, exact commands, failures, raw cohorts and replay material are also preserved there. Raw evidence is external; this status file is the current bounded report.
