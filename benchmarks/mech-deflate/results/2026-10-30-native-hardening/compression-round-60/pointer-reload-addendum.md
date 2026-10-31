# R60 addendum: where the extra machine code comes from

The retained R60 result remains **13.565443× reference elapsed**, from **13.521790×**, with elapsed **+0.3228%**, cycles **+0.1961%**, and retired instructions **−0.9686%**. Allocation counts are unchanged. This addendum explains the larger generated code; it does not add or revise timing samples.

The direct-address change removes 1,724 static UDIV instructions and 862 MSUB instructions at 862 eligible sites. Nevertheless, generated text grows by 759,684 bytes and the entry stack by 512 bytes. Inspection of the actual Objects now shows that most extra code reloads pointers from stack spill slots.

| Static stack-load class | R59 | R60 | Delta |
|---|---:|---:|---:|
| Original locals pointer | 118,456 | 181,076 | +62,620 |
| Helper argument-lane pointer | 15,523 | 31,255 | +15,732 |
| All scalar stack loads | 562,473 | 628,971 | +66,498 |
| High-offset indexed scalar stack loads | 402,254 | 468,335 | +66,081 |

These rows overlap. The original locals pointer moves from stack offset `0x1a6d0` to `0x1a8d8`; both offsets already exceed 100 KiB. Each high-offset pointer reload uses a `mov`/`movk`/`ldr` sequence. Other major pointer roles—invocation context, admission callback, helper callback, result lane and native frame pointer—retain their original load counts.

A matched ordinary checked-add fallback at source 796 establishes the behavior directly. R59 loads the locals pointer once and reuses its register for two local reads. R60's first local read overwrites that register, then reloads the pointer before the second read. Thus the local arithmetic simplification changed register allocation elsewhere in the same generated function. The additional instructions reload known pointers; they do not copy input buffers or enlarge the source-level scratch arrays.

The exact register allocator trigger is still unproved. The pinned Cranelift 0.131.3 / regalloc2 0.15.2 source has a two-split limit and a per-use minimal-bundle fallback. That mechanism is consistent with the observed code, but disassembly alone does not establish which bundle reached the limit or whether ordinary spilling/coalescing explains part of the change. A filtered compile-only trace of the locals and argument-pointer spill sets is the next diagnostic. This code-growth finding also does not establish that the extra reloads caused the small adverse elapsed result.

See [the detailed audit](pointer-reload-evidence/r60-stack-audit/README.md), [exact pointer counts](pointer-reload-evidence/r60-stack-audit/pointer-delta.json), and the matched [R59](pointer-reload-evidence/r60-stack-audit/before-same-source796.asm) / [R60](pointer-reload-evidence/r60-stack-audit/after-same-source796.asm) blocks. The evidence includes source hashes, dependency source excerpts and read-only counting scripts. All counts above are static; the whole-call retired-instruction measurement remains a separate result.

The main R60 evidence packet has independently replayed successfully: 53 archives, 454 verified file occurrences, and byte-identical scorecard JSON/CSV, profile numbers and qualification results. No executable was required. The R60 decision remains to retain the instruction reduction while recording the adverse elapsed, cycle, text and stack effects. Further experiments must keep those costs visible.
