# Late completed-key overflow removal: qualified, not selected

IR308 remains selected. IR310 preserves checked arithmetic through early CFG cleanup and capture selection, then removes only surviving overflow branches whose current SSA expression still exactly matches the completed-producer certificate. No global range, array bound, cursor or post-ID permission is exported. Cranelift remains production.

The first randomized cohort measured IR308 at **1.722708 ms (1.125309× fastest eligible Rust)**, IR310 at **1.739625 ms (1.136360×)**, and its same-runtime disabled control at **1.726584 ms (1.127841×)**. Rust ordered-mask-tables256 took **1.530875 ms**. The paired change versus IR308 is **+1.068858%**, 95% interval **[+0.398281%, +1.819810%]**; versus ablation **+0.518992%**, interval **[+0.224215%, +1.201422%]**. This clear regression does not receive an unchanged confirmation rerun.

Native code shrinks 4,144 bytes and the main query shrinks 612 bytes without a larger frame. Nevertheless, retired instructions are essentially unchanged versus ablation (32,322,907 versus 32,325,036), allocations remain 720, and requested bytes remain 7,326,125. Static code/stack changes are not evidence of measured spill costs or cache pressure.

All 490 current compiler contracts pass, retaining all 489 parent contracts; the new exact-SSA/changed-expression contract is included in both CI listing and execution. Map/image/range/protocol/frame contracts also pass. The unchanged 168 Engine contracts are reused with exact source/library verification. All 133 scalar/DEFLATE non-timing records, 64 linked outputs, 1,271 latency outputs and 294 resource outputs are exact. First build/harness corrections are retained.

See report.json and evidence/first/paired-analysis.json for all sizes, resources, whole-process and restoration measurements. Whole-process/restoration remain outside the mandatory gate. No new IR310 phase profile is claimed; current retained profiles continue to prioritize serial indexing.

Next: revisit compact tag IDs and combinatorial lookup tables together on the selected compiler, using the latest dictionary, native loops and ownership handoff. Repair actual compiler lowering gaps and qualify source variants before timing.
