# R60: arithmetic removal exposes global pointer reload instability

Parent measured the vector-address candidate separately. This audit concerns the actual generated Objects; it does not create or infer new timing/hardware-counter measurements.

| Quantity | R59 Object | R60 Object | Change |
|---|---:|---:|---:|
| Generated text bytes |13,046,668|13,806,352|+759,684|
| Machine entry stack bytes |108,368|108,880|+512|
| Static encoded instructions |3,261,667|3,451,588|+189,921|
| UDIV instructions |1,825|101|−1,724|
| MSUB instructions |878|16|−862|
| All scalar stack loads |562,473|628,971|+66,498|
| High-offset indexed scalar stack loads |402,254|468,335|+66,081|
| All scalar stack stores |144,709|144,326|−383|

These are static whole-Object counts, not dynamic executions. All text is one large generated function in these Objects. Vector simplification did exactly remove two divisions at each of862 eligible static sites. At actual source797, baseline address calculation at0x2039c–0x203bc becomes the direct `ldrb w10,[x1,x20]` at0x21ab0. Nonetheless, unrelated transport code expands.

## Proven cause of most added code

The newly added instructions overwhelmingly reload already known pointers from machine stack slots. They do not load literals or copy input buffers. Each high-offset load uses the sequence `mov w16,#low; movk w16,#high,lsl#16; ldr target,[sp,x16,sxtx]` because the spill slot is more than100KiB from SP.

`pointer-delta.json` maps the exact pointer roles from the entry prologue and frame ABI:

| Pointer | R59 spill slot / loads | R60 spill slot / loads | Extra loads |
|---|---:|---:|---:|
| Original locals base |0x1a6d0 /118,456|0x1a8d8 /181,076|62,620|
| Helper argument lane |0x1a6a8 /15,523|0x1a8b0 /31,255|15,732|
| Invocation context |0x1a6c8 /55,450|0x1a8d0 /55,450|0|
| Admission callback |0x1a6c0 /39,938|0x1a8c8 /39,938|0|
| Helper callback |0x1a6b8 /15,518|0x1a8c0 /15,518|0|
| Helper result lane |0x1a6b0 /27,564|0x1a8b8 /27,564|0|
| Argument capacity |0x1a6a0 /8,336|0x1a8a8 /8,348|12|
| Native frame pointer |0 /113,038|0 /113,038|0|

The listed pointer changes total+78,364 loads; other scalar stack loads decrease11,866, producing the net+66,498. These are overlapping explanations of the whole count, not additional independent savings.

A matched ordinary source796 block shows the difference directly. In R59, after one locals reload, `ldr x0,[x13,#0x1ac0]` and `ldr x1,[x13,#0x278]` share the same locals register. In R60, `ldr x0,[x0,#0x1ac0]` overwrites that register, followed by a fresh three-instruction locals-pointer reload before `ldr x1,[x1,#0x278]`. See `before-same-source796.asm` and `after-same-source796.asm`. This block is the ordinary checked-add fallback, not the simplified native address calculation. Thus a local arithmetic simplification changed register allocation across the containing function.

The512-byte stack increase is also generated machine-frame layout, not source buffer growth. The prologue's local allocation changes108,272→108,784 after the same96 bytes of saved register pairs. The highest observed spill access moves0x1a6d8→0x1a8e0 (520 bytes), with final alignment/padding differing. This does not establish an exact count of newly allocated spill slots; backend frame-layout instrumentation is needed for that. No matrix dimensions, source scratch capacities or compressor allocation sites changed.

## Backend mechanism: confirmed code, unconfirmed event

The pinned dependency is Cranelift0.131.3 with regalloc2 0.15.2. `codegen/control/mod.rs:467–471` creates the locals/context/callback pointers once at entry, and the same SSA values span the entire generated function. `program/compiler.rs:20–33` chooses speed optimization and normal default register allocation. Cranelift's `machinst/compile.rs:52–65` maps the default backtracking algorithm to regalloc2 Ion.

The exact regalloc2 source has `MAX_SPLITS_PER_SPILLSET: u8 = 2` at `ion/data_structures.rs:313`. `ion/process.rs:430–440` invokes `split_into_minimal_bundles` after that limit; lines793–823 explain that this deliberately falls back to per-use minimal register reservations plus a spill bundle to bound allocator complexity. The saved source files and SHA-256 receipts establish the version and implementation.

The actual code is consistent with that per-use spill behavior, but the Object alone does not prove that R60 crossed this exact threshold, which pointer bundle hit it, or whether a normal spill/reload conflict produced some portion of the change. Do not report “the two-split limit caused the regression” as established. Likewise, fewer arithmetic operations do not necessarily mean higher abstract register pressure: their removal changes use lists, live-range boundaries, coalescing and allocation order. The output establishes an allocation sensitivity; the detailed causal path remains to be instrumented.

## Next bounded investigation

1. Keep exact R59/R60 Objects, source and compiler settings as the control pair. Obtain one compile-only diagnostic pair outside primary timing, with register allocation statistics and a filtered event sink. Record overall spills, reloads, spill-slot count/size, bundle splits and minimal-bundle fallbacks. Preserve failure/work/error logic; do not modify optimizer behavior in this diagnostic.
2. Map the entry frame's locals pointer and argument pointer from CLIF values to machine VRegs. The exact entry loads are at `Frame.locals` and private `Frame.arguments`. For those two spill sets only, record original ranges, split positions/counts, fallback event, conflict/eviction reason, allocated physical registers and resulting reload sites. A complete trace of this huge function is unnecessary and could dominate compile memory/disk.
3. Distinguish: (a) the two-split fallback fires differently; (b) normal pressure/coalescing changes without that fallback; (c) common reloads are inserted then not eliminated. Check whether repeated reloads within one machine basic block survive the move/reload optimization. Do not increase allocator thresholds as a production “fix” before this is known.
4. Run the already bounded first-change `spill_all` deletion separately. It removes actual explicit local-bank users and may improve both dynamic transport and the allocator's global problem; it may also perturb allocation adversely. Reuse this same static Object census plus matched dynamic instruction/elapsed measurements after it. Do not presume it will fix the R60 expansion.
5. Depending on the trace, the smallest generic compiler follow-up is likely shorter pointer live ranges or boundary-specific local exports. An entry-hoisted locals pointer that is used throughout a3-million-instruction function is a poor fit for coarse spill decisions. Candidate designs must remain in the existing frame/SSA architecture, retain exact private ABI checks and avoid a second unchecked execution path. Explicit narrow pointer rematerialization from the immutable native frame, or outlining an already proved numeric region, require their own bounded implementation and verification; neither is implemented or accepted here.

Acceptance for any follow-up: same source/typed-IR pins and exact output, all existing finite work/failure/alias/retry tests, unchanged source buffer capacities, matching logical work, fewer actual pointer reloads in the Object, whole-call retired instructions and elapsed pairs. Report contrary timing and code/stack effects; static instruction reductions alone do not establish faster execution.

## Reproduction and files

`../audit-stack-reloads.py` streams the retained R59/R60 Objects into `before.json` and `after.json`. It recognizes exact generated ARM64 scalar SP-relative loads/stores, reconstructs the common `w16` high-offset sequence, and retains counts/examples. It does not count vector or pair stack accesses; those opcode totals are unchanged in the separate full Object comparison. `pointer-delta.json` derives the role-level deltas. `../compare-native-objects.py` supplies full opcode/text/stack comparison and input hashes; the parent retained that output under R60's `binary-audit/`.

No compiler build, dependency edit, compressor run or benchmark was performed by this audit agent. All dependency-source snapshots are evidence, not replacement production files.
