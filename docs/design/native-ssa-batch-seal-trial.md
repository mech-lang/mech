# Batched SSA parameter sealing trial

The candidate is rejected. In eight serial complete level-12 compiles, source
CFG construction increases **1.59%**, compiler phases **0.39%**, and retired
instructions **0.006%**. Both balanced compiler-time blocks are slower. Every
source, MBC, Object and complete metadata byte matches. Production source is
restored to `8dc06bea88ec709ca5f690013811d710ee3024d3`, the retained point-liveness
compiler. The sub-ten-second goal remains open.

The trial replaces repeated removal of consecutive trivial block parameters
with one list reconstruction. Removing one parameter shifts and renumbers the
remaining parameters; independent known trivial merges could share that work.
The trial keeps placeholder allocation and value IDs, reads all predecessor
definitions before resolving aliases, and uses the original state machine for
unknown, nontrivial, cyclic or dependent merges. A root parameter in the current
block stays on the original path. No grammar, language or runtime code changes.

The measured patch has SHA-256
`0b08b6a15556f4f3a91f99c9e2a6bf6cf7b923b7e66bc9ec145933c6007147ff`.
The evidence reconstructs the complete candidate source from the retained base
and patch. It also preserves the passing new regression test; neither the trial
implementation nor its additional test is retained in production.

## Measurements

Control and candidate use the same locked optimized `compiler` profile, Thin
LTO, one codegen unit, fixed source and current **61 runtime flags**. Eight fresh
processes run ABBA then BAAB, four of each subject. Screens remain separate.
Builds, focused tests, IR probes, fixtures, hashing, copying and archive work do
not overlap primary timing. Historical 60-flag measurements are not pooled.

Arithmetic means follow. Positive changes mean more time, work or memory.
Nested intervals overlap and must not be added. Whole-process time includes the
profiling driver's polling tail; the separately measured compiler phases are
also reported. Eight observations do not establish a population estimate.

| Measurement | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete process | 17.6355 s | 17.6362 s | +0.0040% |
| Compiler phases | 17.4297 s | 17.4982 s | +0.3929% |
| Parsing | 2.0745 s | 2.0889 s | +0.6982% |
| Catalog | 0.0361 s | 0.0359 s | -0.3461% |
| Frontend | 2.5506 s | 2.5511 s | +0.0197% |
| Artifact | 0.0265 s | 0.0258 s | -2.4458% |
| Bytecode encoding | 0.0093 s | 0.0092 s | -0.7259% |
| Native | 12.7314 s | 12.7858 s | +0.4269% |
| Canonical lowering | 0.0989 s | 0.0983 s | -0.6049% |
| Preparation | 2.0249 s | 2.0083 s | -0.8208% |
| Numeric helper generation | 2.1355 s | 2.1455 s | +0.4687% |
| Source CFG construction | 2.3741 s | 2.4118 s | +1.5895% |
| Cleanup including compaction | 3.4082 s | 3.3992 s | -0.2625% |
| CFG cleanup | 3.2398 s | 3.2286 s | -0.3452% |
| Value compaction | 0.1684 s | 0.1706 s | +1.3294% |
| Retired instructions | 257.8680 G | 257.8830 G | +0.0058% |
| Maximum RSS | 5.9115 GB | 5.9673 GB | +0.9439% |
| Peak footprint | 5.8811 GB | 5.9369 GB | +0.9489% |

Compiler-time block means change **+0.1698%** and **+0.6164%**. The first screen
appeared to reduce source CFG time, but that direction does not repeat in the
balanced cohort. No useful instruction reduction is established. One candidate
RSS observation is 6.1314 GB, versus 5.9106–5.9153 GB for its other three
observations; it remains in the mean and raw data. Its cause is not isolated.
Unrelated phase movements are not attributed to the candidate.

Both subjects retain exactly **1,242,559** values from **20,798,230**
pre-compaction IDs. All output files match the separately pinned current
control. Fifteen focused SSA tests pass, including manual parameters and mixed
retained/trivial merge ordering. The 88 paired complete IR cases compare raw
value IDs, immediate and resolved aliases, types, value definitions, queries,
side effects and variable enumeration. All 176 generated functions verify.
Cases include sparse block IDs, mixed integer/floating types, diamonds, loops,
unreachable cycles, manual parameters and forward/reverse sealing. Fourteen
successful scalar/numeric fixture pairs also preserve every output byte.

## Compiler build parallelism

The candidate compiler executable rebuild takes **275.189 seconds** with two
Cargo jobs and warm dependency caches. This Rust build is separate from the
approximately 17.5-second Mech source compile. The three rebuilt crates form a
dependency chain: Cranelift frontend, GPU host, then profiling executable.

Subsequent rebuilds use **four Cargo jobs** on this Apple M3 Air with eight
logical cores, four performance cores and 16 GB of RAM. No four-job build has
been measured here, so no speedup is claimed. Cargo jobs schedule independent
crates; they cannot bypass this dependency chain. The optimized profile keeps
Thin LTO and one codegen unit. Changing codegen partitioning would require new
compiler timing and generated-program qualification.

The current helper and entry emission calls share a mutable Object module.
Separate function compilation is a potential parallelism boundary, but this
trial does not implement or measure it. The retained compiler still needs less
SSA construction and cleanup work to approach ten seconds.

## Evidence and replay

The [compact result](../../benchmarks/mech-deflate/native-profile/ssa-batch-seal-trial-apple-m3-20261114.json),
[receipt](../../benchmarks/mech-deflate/native-profile/ssa-batch-seal-trial-apple-m3-20261114.archive.json)
and [complete repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
preserve the measured patch, full source, all commands and logs, screens, eight
observations, counters, complete output files, tests and IR comparisons.
Executables and copied dependency libraries are excluded and hash-pinned.

Pure offline replay reconstructs the candidate source, checks the stored test
and IR records, verifies every generated byte and recomputes the numerical
summary. It requires no compiler, original scratch paths or network, and does
not rerun timings. No new compressor-runtime latency campaign is claimed and
no scalar execution machinery is retired. Grammar, checked errors, fresh
shape/factory/visit/ownership checks, finite admission and accounting remain.
