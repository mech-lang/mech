# Terminal byte appends

The compact emitter derives a terminal append route from the typed operations
and selected branches of the same program. The proof traces each admitted
length interval from the real entry, transports the typed array lanes through
the selected copies and tail initializer, and rejects routes with a repeated
visit, an unproved error, a different live owned buffer, or an observable
intermediate value. The route reuses the normal scalar instruction emitter.
The original checked body handles all other calls.

At entry the route checks borrowed owners, current row geometry, and space
for the original reserve and every write. The original sequence of operations
therefore needs one fresh backing and no growth. This permits one prefix copy,
packed little-endian writes and a bounded initialized source span. The
allocator, output capacity, input and prior-output aliases and allocation
failure origins are preserved. The optional first owner slot borrows stable,
caller-owned metadata storage for the invocation. Workers still have their
own storage and imported owners retain their original lifetimes. The heap
destructor performs the actual cleanup inside the whole-call timer.

The optimization is opt-in under `MECH_TERMINAL_BYTE_APPENDS` and applies to the
existing compact diagnostic. These results measure complete compression,
allocation and ownership transfer using the compact benchmark contract.

| Cohort | Cases | Validated timed calls | Trial / control | Faster than Roc | Faster than libdeflate | Faster than both |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Direct acquisition with loan and one cleanup | 19 | 161,088 | 0.901698× | 12 → 15 | 15 → 15 | 8 → 11 |
| Combined prototype, full | 81 | 245,664 | 0.974807× | 76 → 78 | 35 → 36 | 30 → 33 |
| Combined prototype, repeat | 81 | 245,664 | 0.976497× | 74 → 75 | 35 → 35 | 28 → 29 |
| Review build, full | 81 | 245,664 | 0.975403× | 75 → 76 | 36 → 35 | 30 → 30 |
| Review build, repeat | 81 | 245,664 | 0.975720× | 71 → 74 | 34 → 35 | 24 → 28 |
| **Integrated build, full** | **81** | **245,664** | **0.974266×** | **73 → 76** | **34 → 36** | **26 → 31** |
| **Integrated build, repeat** | **81** | **245,664** | **0.976169×** | **73 → 74** | **35 → 35** | **27 → 28** |

Each full run uses 24 balanced orders per case. Both original 1.05 timing
gates pass on all six complete comparisons. The table uses balanced-block
geometric means with fresh same-run peers. The clock-quantized single-call
medians and both order halves remain in the receipts.

| Input | Integrated / control, full and repeat | Integrated / Roc, full and repeat | Integrated / libdeflate, repeat |
| --- | ---: | ---: | ---: |
| Empty | 0.544510× / 0.552285× | 0.902915× / 0.915864× | 0.007312× |
| One stored byte | 0.503414× / 0.518773× | 0.815957× / 0.841091× | 0.007453× |
| Seven stored bytes with prefix | 0.483987× / 0.507572× | 0.912856× / 0.941752× | 0.007828× |
| Eight literals | 0.997912× / 0.997995× | 0.162373× / 0.166263× | 2.117454× |
| Natural 32 KiB | 0.994027× / 0.999530× | 0.998049× / 1.000093× | 0.905744× |
| Natural 33,027 bytes | 1.000658× / 1.004664× | 1.002588× / 1.005336× | 0.909091× |
| Natural 1 MiB | 1.001075× / 0.999640× | 1.002909× / 1.006801× | 0.812412× |
| Held-out slides, 65,537 bytes | 1.000300× / 1.000401× | 1.016263× / 1.015827× | 0.888098× |

The three stored cases win against both peers in both integrated runs. The
other 78 cases take 0.998824× and 0.999672× paired control in aggregate.
Natural 32,767 bytes and Reymont 32,769 bytes lose a Roc crossing in the
first run. Natural 32,769 bytes and the held-out 32,767-byte window lose
one in the repeat. Repeated 4 KiB is the largest adverse repeat at 1.020796×
control. The largest time ratio above Roc is the held-out slide case in
both runs. Every case wins against at least one peer in both runs.

The earlier review build loses natural 32 KiB against Roc in both runs at
1.002097× and 1.002863× Roc. It has an adverse 1.040582× control for random
1 KiB in the first run and 1.026108× and 1.009454× for periodic 8 KiB.
Periodic 65,537 bytes loses one libdeflate crossing in the first run, and
periodic 65,536 bytes gains one in the repeat. In the linked review build
the fallback root retains its frame and instructions. All 356 other
previously measured generated bodies are byte-identical, and 1,232 common
symbols move. These comparisons measure end-to-end behavior of each exact
worker and cannot identify a specific code placement as the cause.

The initial one-acquisition compiler screen keeps the general owner
allocation. It takes 0.950098× its paired control across 23 cases, with
the three stored inputs at 1.190188×, 1.086071× and 1.190664× Roc. Grouping
geometry comparisons and folding the duplicate reserve bound takes
0.946541× in a separate 23-case screen. The wrapper shrinks from 524 to
388 bytes. A caller loan with the general allocator reaches 1.093022×,
0.963677× and 1.076490× on the stored rows. Specializing the existing U8
layout acquisition under the loan reaches 0.917215×, 0.849916× and 0.958612×
on the same rows in a fresh 19-case screen. Every screen passes both gates.
The individual screens retain their own peers and controls.

A further parser experiment changes the direction of the compiled recurring
branch. The usual continuation takes one branch, with the extra branch
now on completion. It takes 1.007231× across 23 cases and keeps the same
number of wins against both peers. Empty and one stored byte miss the block
gate at 1.066058× and 1.051940× paired control. Natural 1 MiB changes by
1.000315× and slides by 1.000998×. That experiment stays in the evidence.

The focused build uses nightly 2026-03-03, optimization level 3, warnings as
errors, and the existing `aot,canonical-native,canonical-native-program,
canonical-native-program-runtime,canonical-native-runtime,jit` features.
Original and decoded artifacts emit identical machine Objects and link
identically. The only metadata distinction is the source input form. Each
passes 162 complete linked streams. Additional controls cover all eight
short lengths across ten initialized prefixes. They check valid calls,
layout and row rejection before publication, exact first failure origin,
required growth fallback, aliases and successful recovery. A held output
alias is tested at each short length. Thirteen optimized runtime tests
include owner reuse after heap movement, allocation failure, parallel
cleanup, and direct output transfer. The opt-out emits the published
control Object and passes 162 linked checks.

The integrated build uses base
`1ce14381f2c3428a20c0c727a07c4943995a3795` and
`cargo +nightly-2026-03-03 build --locked --offline --release -j2`
with the `benchmarks/mech-deflate/native-profile/Cargo.toml` manifest and
`compile_profile` target. The repository producer independently emits the
timed Object and matching descriptor with the flag on and off. Both linked
workers are byte-identical to the timed artifacts and pass another 162
checks each. The source SHA-256 is
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
The original/decoded producer uses document ID 203 and its bytecode SHA-256 is
`51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`.
The repository producer uses its own fixed document ID and bytecode SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
The common candidate Object is
`38b194ec2c931fbd5d584ace06bdaba1efb59381e64eef1d662a5f2f12bd96b4`.
The integrated worker is
`e2eb20055d46c7f9cc2a2ec9274eeede6412fcadaa0e1da7b533595a1445a8b9`.
The earlier review worker is
`6c96a33eefdf6f6a8a47505784ca1e1c6f7b3d62d91a6f4c869784b99da4eeff`.
The combined prototype worker is
`5284d84c64400a29199e22e8407b1afb876d0467344a68943d24af5e8e9ed61d`.
The disabled control worker is
`29cae80726fda36f7bee459352be9e13fce2b79d8cc7966b95b2d6495d4870a7`.

Reproduce the linked controls with `python3 -B terminal-final/controls.py
original decoded`. The archived `integration/emit.py` and
`integration/prove_link.py` join the repository producer to the pinned
harness. `terminal-final/run.py full full-repeat` runs the paired matrix
sequentially. `screening.py` recomputes each score and both gates from the
recorded comparisons.

Compiler planning, interactive architecture, host catalog, source catalog,
production resident routing, and changed-file formatting checks pass.
Whole-workspace formatting reports inherited differences in
`checked_increment.rs` and two parser files. Warning and unsafe scans fail
at the inherited test and measurement files identified in the previous
report. The separate Rust module-layout scan reports existing file/directory
collisions in native compiler modules and vendored Cranelift. Hosted CI is
outside the timing evidence.

[Matched timing observations, native code, and validation receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/native-compression-terminal-appends-a82fa3f4b31c.tar.gz).
The verified archive holds 12 cohorts and 1,132 file hashes. Its SHA-256 is
`a82fa3f4b31c3054a1a1b498ef94f727099fc77d5dd7e02695312072e6ab6c48`.
