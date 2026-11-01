# R75 scoped budget codegen experiment

The two targeted inlining annotations recover most of the R74 regression but do not beat the accepted R70 runtime. The complete scoped candidate is rejected.

Primary timing is a fresh three-way matched cohort: accepted R70, unfixed R74, and R75. Hardware, heap, census and profile cohorts compare R70 with R75 only. Raw measurements and exact provenance are retained. See REPORT.md and reconstruction.json; reconstruct.py verifies and replays committed receipts without running a compiler or compressor. No larger-input timing or fresh full GPU qualification is claimed.
