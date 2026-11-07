# IR252 selected ranges and host JSON ownership (unselected)

IR252 integrates PR973 through 7f71c3796bac86eb963f6fe28eda7504022089e1 with the selective capture and demand-liveness changes. It removes 766 root selected-range branches and no numeric-worker branches. Root code shrinks; this does not establish less dynamic work.

All 434 randomized latency outputs (31 rounds, seed 990252) and 105 separate resource outputs (seven rounds, seed 9902527) match Rust. Every first/adverse sample remains. The continuous post-JSON boundary includes admission, setup, worker create/run/join and materialization. Fresh launch-to-exit includes restoration, JSON and teardown. Profile and instrumented timings are excluded.

Fastest eligible Rust takes 1.598708ms processing and 23.001917ms whole process. Selected IR242 takes 2.820875ms/1.764472x and 34.434666ms/1.497035x. IR252 mask reuse takes 2.783750ms/1.741250x and 34.086667ms/1.481905x, winning only 18/31 processing and 20/31 whole-process pairs. Required 21/31 in both is unmet. Hoisted reads, partial sums, compact U8/U32 with four or eight workers, and borrowed JSON also fail. Four workers and selected compiler IR242 remain unchanged.

Borrowing JSON IDs/titles with Cow and owned escaped fallback saves exactly 10,000 parsing allocations. It worsens measured processing and memory. Standard 31-case Rust edge suites pass for both borrowed hosts; 32 additional fresh-process escaped/unicode differential checks pass against unchanged owned hosts, whose accepted input domain is wider than the Rust bare-str adapter.

IR252 passes 884 GPU checks with the same two inherited failures and all 389 actual CI-selected names. All 128 scalar and five DEFLATE rows retain every non-timing field, including allocations/bytes. Six native source variants and two borrowed hosts pass 248 Rust edge outputs. Full adoption qualification is not claimed for rejected candidates.

The fresh restore allocation profile exactly matches 26,837 requests/12,830,999 bytes in the resource cohort, with zero dropped, truncated, invalid or unattributed PCs. These are allocation ancestry counts, not CPU-time percentages. Parent composition frames still reconstruct compiler-only field/path vectors; a separate IR253 experiment addresses that concrete gap without changing native kernels or runtime accounting. It has no speed claim here.

The 24-hour comparison is separately anchored at 2026-11-07T09:48:57Z. Historical and current cohorts describe checkpoints, not a paired causal estimate. Original selection timings and the earlier anchored report remain unchanged.

verify.py validates every hash, decoded payload, exact cohort output and paired gate. No compiler source or manifest changes in this evidence commit; its source inventory remains exactly frozen. Large binaries and workstation-wide process snapshots remain local, with published snapshot hashes and unchanged predicates.
