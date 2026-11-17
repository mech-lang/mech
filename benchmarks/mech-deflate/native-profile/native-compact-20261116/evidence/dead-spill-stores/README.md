# AArch64 dead spill-store screen

The candidate uses backward register allocator spill-slot liveness to prune
private stores whenever every path overwrites their cells or exits before a
read. The experimental path skips observed stack locations. The exact
compiler patch was applied at the source revision in the report. Unpack
`compiler-as-run.patch.gz` and apply it at that revision to inspect or reproduce
the experiment.

The 71,960-byte control reproduces the previously timed object, metadata and
worker byte for byte. The candidate emits 71,944 bytes, or 1.24695× Roc and
1.94024× libdeflate, with the same executable file size. It passes 162 exact
linked cases with independent inflation and alias checks. Cranelift's
symbolic allocation checker checks the transformed edits and produces the
same candidate object, metadata and worker. The largest mirrored tails
identified by the separate census are estimated opportunities.

The pass remains an archived experiment. Its 16-byte reduction did not
justify keeping the implementation or running a full timing campaign. The
changed worker's runtime, stack high-water and source-to-link time remain
unqualified. The build command and options are in the report, and each
variant's invocation, options and checks are next to its receipt.
