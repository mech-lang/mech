# Reuse the fixed-view length checked on entry

The phase caller already checks the current fixed view against the length needed by its numeric body. In the vector matcher, search and skip then each repeat the same comparison against the same packet word. The trial removes those two branches only. If the current view fails entry admission, control returns to the original source path before any phase action. The window, arithmetic and indexing checks remain. The proof depends on the current view's length and its unchanged geometry, without using any compressor constants in the proposed compiler rule.

| Fixed-address trial | Cases | Paired elapsed | Roc wins | libdeflate wins | Wins against either |
| --- | ---: | ---: | ---: | ---: | ---: |
| Length reuse alone, screen | 18 | 0.996348× | 4 → 4 | 5 → 8 | 7 → 11 |
| Length reuse and packed reads, screen | 18 | 0.998835× | 4 → 4 | 8 → 11 | 11 → 14 |
| Length reuse and packed reads, full | 81 | **0.997220×** | **53 → 53** | **21 → 25** | **65 → 68** |
| Length reuse and packed reads, confirmation | 8 | 0.995593× | 2 → 2 | 0 → 1 | 2 → 3 |
| Length reuse and packed reads, adverse cases | 10 | 0.999306× | 6 → 6 | 4 → 4 | 8 → 8 |

Both 1.05 gates pass every case in every cohort. The combined full trial beats both references on 10 cases, up from nine in its paired control. The published compiler score is still 68/81 against either reference; the full run's freshly measured control is 65/81. Packed reads alone had lost one win against either in their own matched cohort. The length-only screen loses a periodic libdeflate win while gaining four elsewhere. The two trials use separate measurement orders; their speedups do not multiply.

| Full-cohort input | Control → trial / Roc | Control → trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.173614× → 0.172679× | 2.180255× → 2.168512× | 0.994614× |
| Natural 8 KiB | 0.989371× → 0.991127× | 1.236020× → 1.238214× | 1.001775× |
| Held-out window, 32,767 bytes | 1.102994× → 1.096390× | 1.003712× → **0.997703×** | 0.994014× |
| Held-out window, 32 KiB | 1.099131× → 1.095529× | 1.000868× → **0.997589×** | 0.996723× |
| Natural 32,769 bytes | 1.104386× → 1.097257× | 1.004071× → **0.997589×** | 0.993545× |
| Natural 33,027 bytes | 1.100926× → 1.101972× | 1.005977× → 1.006932× | 1.000950× |
| Held-out mixed, 131,073 bytes | 1.012238× → 1.012063× | 1.175412× → 1.175208× | 0.999827× |
| Held-out random, 65,537 bytes | 1.015709× → 1.011049× | 1.726927× → 1.719004× | 0.995412× |
| Repeated, 65,537 bytes | 0.567557× → 0.565780× | 1.002624× → **0.999485×** | 0.996869× |
| Random 1 MiB | 1.150837× → 1.141056× | 1.627987× → 1.614151× | 0.991501× |

Held-out window 32,767 bytes is 0.996143× libdeflate in the screen and 0.997633× in confirmation. Its paired time improves in both order halves in all three runs. Natural 33,027 bytes is 0.994657× in the screen, 1.006932× in the full run and 1.000678× in confirmation. Natural 32 KiB also crosses back above libdeflate in the full and adverse runs, in both arms. The adverse retest puts periodic 8 KiB at +1.06% versus its paired control (+1.24% full) and natural 8 KiB at 0.992971× Roc. The largest full increase is +1.41% on repeated 4 KiB. Random 65,537 bytes is +3.16% in the screen, −0.17% in full; both measurements are retained.

The full diagnostic changes only the linked vector matcher. Its size goes from 10,172 to 10,020 bytes, static stack references from 441 to 420, and the complete 576-byte frame is unchanged. Length reuse alone produces 10,048 bytes and 423 stack references. The separate hardware-counter comparisons put held-out random at 0.984438× instructions and 0.996315× cycles, random 1 MiB at 0.984387× and 0.995160×, and natural 256 KiB at 0.995784× and 0.997859×. Peak physical memory is unchanged in these cohorts. Eight literals shows 0.999884× instructions and 1.012542× cycles despite the favorable elapsed point estimate. The combined worker's matcher accounts for 3,509 of 5,237 sampled timer PCs for held-out random and 3,763 of 5,363 for random 1 MiB. Counters and sampling do not contribute to the elapsed estimator.

The 275,040 timed calls and 324 separate linked checks preserve level 12, runtime inputs, output and scratch capacities, allocator, reference bytes, independent inflation and complete-call timing boundaries. Compiler control `6e7dd1bed4614034887d11b574251fa06cc0dff5`; source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`. Combined Object `a74dda857cc5f6c4bf27cb52b97f4971a4460d24a096ac26de6eaa9b618d7918`, worker `e90721acd2ef6cf0461d26ab4d812f39d301bfa90cd04f8a50499938cf00e406`. The timed bodies were linked separately; maintained compiler emission is still being qualified.

[Normalized measurements, both profiles, proof and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-6e7dd1bed4-admitted-view-lengths.tar.gz). SHA-256 `f9f920e6984b892a861128b0868f6ce4d713b4daf2a6a87626c4331d43a8c81e`; all 354 files verified from the public download.
