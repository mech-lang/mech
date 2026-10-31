# R62 locals-pointer placement: verified static effect

R61 Object SHA-256 `4e222709764d92ac2b2b74e06a7237d3b901bf07cf095e1c23a7751c0dd8269d`; R62 Object `4921d95fdc25eaff386a16eb3de22c0178c3171d0eba668a5bee7fabaa0f7224`. Typed-IR and source program revision are unchanged. `binary-audit.json` contains hashes, canonical metadata/descriptor agreement, complete opcode deltas and corrected stack attribution.

| Static quantity | R61 | R62 | Change |
|---|---:|---:|---:|
| Generated text bytes |13,082,688|11,800,572|−1,282,116|
| Encoded instructions |3,270,672|2,950,143|−320,529|
| Machine entry stack bytes |105,552|107,344|+1,792|
| Scalar stack loads |593,954|514,629|−79,325|
| Indexed high-offset stack loads |438,478|319,377|−119,101|
| Scalar stack stores |137,100|138,046|+946|
| Locals-pointer home loads |139,076|39,933|−99,143|
| Argument-pointer loads |30,900|15,445|−15,455|
| UDIV / MSUB |101 /16|101 /16|0 /0|

These are static whole-Object counts, not executed visits or hardware-counter measurements. Do not multiply them by sampling shares or add role deltas to whole-stack deltas. Additional spill changes elsewhere partly offset the reduction in pointer-home loads. The increased machine stack is retained as a contrary result.

The entry prologue stores the checked locals pointer into the fixed home atSP0, at Object address0x108. The native frame pointer is now atSP8. There are39,932 loads from the low locals home and one load from the surviving cold original-pointer spill at0x1a2d8. Entry argument capacity, callback, context and result pointer counts remain unchanged. Arguments benefit from changed register allocation even though the production patch modifies only locals placement.

## Matched ordinary source796 action

The source action and its descriptor identity remain the same checked addition. In R61, the successful computation reloads the high locals spill separately for the two operands:

```
0x203e4: mov  w16,#0x9bd0
0x203e8: movk w16,#0x1,lsl#16
0x203ec: ldr  x0,[sp,x16,sxtx]
0x203f0: ldr  x0,[x0,#0x1ac0]
0x203f4: mov  w16,#0x9bd0
0x203f8: movk w16,#0x1,lsl#16
0x203fc: ldr  x1,[sp,x16,sxtx]
0x20400: ldr  x1,[x1,#0x278]
0x20404: adds x0,x0,x1
```

In R62, a single source-block snapshot at0x1b1e4 stays in x26 across the existing admission call at0x1b220. The same computation and output use that register:

```
0x1b1e4: ldur x26,[sp]
... existing admission and failure check ...
0x1b23c: ldr  x0,[x26,#0x1ac0]
0x1b240: ldr  x1,[x26,#0x278]
0x1b244: adds x0,x0,x1
... existing checked-add failure sequence ...
0x1b268: str  x0,[x26,#0x1b00]
```

This verifies both cheaper low-offset access and better reuse of a short-lived base register. It does not establish a particular regalloc split/eviction event. The two-split fallback remains a diagnostic hypothesis. Full source796/797 windows and the prologues are retained under `pointer-homes/before` and `pointer-homes/after`.

## Historical correction

The old audit tracked only `mov w16` immediate materialization and could misattribute later accesses after `mov x16` to a stale slot. Corrected R61 locals and argument counts are139,076 and30,900, superseding165,897 and31,199. All whole-stack totals, opcode counts, Objects and measured dynamic cohorts are unaffected. The separate `pointer-role-correction` packet reconstructs all four historical Objects, preserves the old script/notes, and contains four passing parser regressions including the exact missing instruction form. This report uses the corrected parser throughout.

This audit did not compile or execute a program. The parent owns timing, hardware counters, allocation and semantic qualification; use those separate receipts for the performance acceptance decision.

The full disassembly, corrected histograms, parser regression receipts and historical correction packet are preserved under `binary-audit/` in `analysis-results/receipts.tar.gz`; the adjacent manifest verifies every file.
