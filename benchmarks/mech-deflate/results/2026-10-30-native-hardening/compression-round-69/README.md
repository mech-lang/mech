# R69 retained call-port arrays: measured, rejected, restored

REPORT.md explains the experiment, exact sources and decision. Accepted production remains R67; candidate count 6,552 is not production. Two independent call-refresh regressions remain in the maintained Core suite. All performance observations are the unchanged complete eight-byte level-12 workload.

Run `python3 reconstruct.py /absolute/new/replay-directory` from this directory. It verifies every archive member/hash, preserves and overlays chronological versions, then recomputes eight numerical outputs byte-for-byte using only committed raw receipts. It does not execute a compiler, compressor, test binary, remote service or missing scratch input. The original reproduction commands and exact selected release/test feature profiles are in JSON receipts; historical executable/compiler inputs are pinned, not included in numerical replay.

Raw primary, CPU, heap, complete allocation stacks, samples and whole-entry instruction windows are separate cohorts. CPU-time samples and static instruction counts are not per-function retired-instruction counts. Failed disk linking and the corrected report renderer are classified in prior-failures.json; both are resolved without allowances or workload changes. Historical build cleanup is listed separately from compiler behavior.
