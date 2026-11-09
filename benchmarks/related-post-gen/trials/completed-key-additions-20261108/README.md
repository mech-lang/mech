# Completed-key arithmetic: qualified, not selected

IR308 remains selected. IR309 exports a compiler-only proof from completed checked key producers to exact key+1 consumers; all original array, cursor, post-ID and ownership boundaries remain intact. Cranelift remains the backend.

The confirmation cohort measured IR308 at **1.720250 ms (1.122208× fastest eligible Rust)**, IR309 at **1.727250 ms (1.126774×)**, and its same-runtime disabled control at **1.733542 ms (1.130879×)**. Rust ordered-mask-tables256 took **1.532916 ms**. The paired IR309 change versus IR308 is **+0.792961%**, with 95% interval **[+0.091961%, +1.231767%]**. The control comparison does not establish a gain. No unchanged favorable rerun is planned.

IR309 retires about 226,000 more instructions than its ablation despite 1,108 fewer native code bytes. The fresh phase profile finds reductions in serial indexing but additional query instructions. The main query function grows 324 bytes and its frame grows 16 bytes; static stack references are evidence for further investigation, not a dynamic spill-cost measurement.

Qualification preserves all 484 parent compiler contracts and passes 489 current contracts, 176 Map contracts, 29 image/caller contracts, 28 UTF8/range/protocol contracts and two packed-frame contracts. The same 168 Engine contracts are reused only after exact source/library verification. All 133 scalar/DEFLATE non-timing records, 64 linked correctness outputs, 2,418 latency outputs, 560 resource outputs and 64 new profile captures match their reference outputs.

See report.json and the complete first/confirmation receipts for allocations, requested/peak heap, instructions, cycles, code/executable/frame sizes, restoration and whole-process costs. Whole-process and restoration remain outside the mandatory acceptance gate.

Next: transport the same proof to the late unsigned check-removal pass, preserving earlier code shaping. The two fresh postings payload copies were already eliminated by selected IR308; real positions/prefix aliases remain protected.
