# Native query compiler experiments — 2026-11-12

This packet preserves **2,314 exact timed outputs**: 31 × 51 primary observations, a separate 7 × 49 instrumented resource cohort, and a 13 × 30 independent confirmation. It covers compiler experiments IR339, IR341, IR342, IR343 and IR344. Later compact-popcount and direct packed-load work is outside this packet.

**IR334 remains the selected checkpoint.** IR341’s U32 comparison lowering improved against its matching IR338 U32 parent, and that limited gain survived independent confirmation. It did not establish a gain over the selected checkpoint or the fastest eligible Rust. IR342 wide’s initial gain did not survive confirmation. All producer-proof combinations and their regressions remain unselected.

Primary medians use the continuous post-JSON interval including admission, intervening setup, fresh worker creation/run/join, internal cleanup and result materialization. Every sample launches a fresh process. Whole-process time remains visible and is not an acceptance gate. Resource instrumentation is separate and has no ordinary latency authority. Two preinitialized Rust controls remain diagnostic; the fastest eligible reference includes setup and worker creation/run/join.

| Implementation | Wide (ms / × fastest Rust) | U32 (ms / × fastest Rust) |
|---|---:|---:|
| IR338 parent | 1.461584 / 1.190295× | 1.464208 / 1.192432× |
| IR340 dense owner banks | 1.460833 / 1.189684× | 1.433625 / 1.167526× |
| IR339 producer element ranges | 1.459167 / 1.188327× | 1.462875 / 1.191347× |
| IR341 comparison conjunctions | 1.451042 / 1.181710× | 1.439542 / 1.172345× |
| IR342 dense + comparisons | 1.451500 / 1.182083× | 1.442667 / 1.174890× |
| IR344 dense + producer ranges | 1.456666 / 1.186290× | 1.469375 / 1.196640× |
| IR343 all three | 1.450959 / 1.181643× | 1.473333 / 1.199864× |

The same primary cohort measured selected IR334 at **1.451041 ms / 1.181709×** and fastest eligible Rust, `rust-ordered-mask-tables256-fresh4`, at **1.227917 ms**. These are medians; paired comparisons use every matched round and remain separately archived.

IR341 U32 versus IR338 U32 changed by −1.627797% in the primary paired analysis, 95% interval [−2.390371%, −0.899175%]. Independent confirmation found −1.519678%, interval [−3.251617%, −0.965052%]. IR342 wide versus IR340 wide initially showed −0.832959%, interval [−1.956288%, −0.458911%], but confirmation found +0.335592%, interval [−0.754960%, +1.060946%]. No observation was removed or rerun for selection.

The compiler changes are general. Producer facts remove only independently proved guards; unsuccessful joins retain their guards. AArch64 conjunction lowering recognizes bounded, canonical, single-use integer comparison trees. Real query filters become CMP + four CCMP + branch, matching Rust’s six comparison instructions. All operands are prepared before flag emission; cold consumers, raw Boolean values, floats, phis and unsupported trees fall back. Cranelift remains the compiler backend; LLVM tools only disassembled machine code.

The packet includes 640 exact qualification outputs across 20 ordinary/instrumented adapters, four pure matcher contracts, and 1,031 native functions exercising 1,847,152 correctness calls. Fresh coherent IR343 coverage passed 37 distinct focused tests and 1,088 broad tests, with the same 16 inherited failures, 11 ignored and 35 filtered tests. Raw failed ISLE and two-producer designs, the literal-include test wrapper failure, and both confirmation preflight failures are retained. Both confirmation failures launched zero native executions.

Historical unselected IR335 total-work variation is preserved and remains unattributed. Its exact numeric-worker bill is checked. Other managed totals are checked against each variant’s own fixed source/compiler qualification. Four tasks and one batch are required; observed peak concurrency from one through four is preserved, with explicit per-cohort histograms in verification output. The full-path sampled profile retains the initial failed sampler, retry and original outputs; its averages under changing load are diagnostic, never timing authority.

Run the standard-library-only verifier from a normal checkout:

```sh
python3 benchmarks/related-post-gen/integration/native-query-optimization-20261112/verify.py
```

It needs the published IR338 commit `b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164` and IR340 commit `1ef5985a9c58fd2857bdf8eedc06a2b7b50ef622`. It reads no historical temporary directory, compiler executable or native benchmark binary. It verifies compressed records, outputs, orders, timers, paired intervals, resource summaries, qualifications, source inventories and backend configuration pins. Candidate sources are stored only as minimal overrides and reconstructed patches against those published bases. Libraries and executables are omitted, with their exact hashes preserved. SHA-addressed storage deduplicates repeated full outputs and other equal records.

`report.json` indexes the evidence; `records.json` maps each historical record to its compressed content. Existing integration packets are unchanged.
