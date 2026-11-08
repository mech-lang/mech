# Closed composite constructors: retained whole-process and allocation gains

The selected processing implementation remains IR295 histogram reuse. This is a general constructor optimization, not a new processing-speed record. Production backend remains Cranelift; no algorithm, input-dependent preparation, allocation, worker creation/run/join or cleanup moved outside the continuous post-JSON timer.

IR297 reuses the selected tuple/record constructor when every input and output schema has no shape parameters. Fresh child schema keys/empty parameter tapes are checked at the original visit; a mismatch takes the original current-constructor path. The full original normalization work/memory envelope, current payload measurement, finite errors, change detection and publication remain. Current Map contents/cardinality are never cached.

IR296 is inherited and independently tested, but leaves this benchmark native text byte-identical to IR295. Its general modular-range fix is not credited with a benchmark compiler speedup. IR297 also leaves all numeric native text identical: 49 numeric functions, 4 workers, 8 numeric worker regions, 1031388 code bytes and1568 stack bytes.

| Cohort | Fastest eligible Rust | IR295 selected | IR297 | IR297 / fastest Rust |
|---|---:|---:|---:|---:|
| First C22 | 1.539667ms | 1.910041ms | 1.927209ms | 1.251705x |
| Independent C23 | 1.516500ms | 1.912125ms | 1.917875ms | 1.264672x |

All22 Rust controls ran in each cohort;20 are processing eligible. Preinitialized pools remain whole-process only. Each cohort has744 exact latency outputs and210 exact resource outputs, including separate resource diagnostics for five adverse source experiments. All1908 outputs are retained by content hash.

IR297 paired processing changes are +0.9290% CI[+0.1837,+2.0709] and +0.1439% CI[-0.6567,+0.8283]. No processing improvement is established. Whole-process changes are -4.5207% CI[-7.0282,-2.5671] and -4.0009% CI[-5.3669,-2.8643]. Confirmation whole medians are38.485291ms selected and36.997084ms candidate; fastest whole Rust is24.910292ms. This whole-process gain is repeated, but its entire mechanism is not isolated by the small post-JSON instruction saving. It is retained without claiming parity or making whole-process a gate.

Confirmation post-JSON resources:1989 to1973 allocation requests;7873166 to7871350 requested bytes;additional peak live requested heap unchanged at5763262bytes;34682811 to34649073 instructions;11167664 to11080827 cycles. Candidate RSS27246592bytes/physical footprint16580968bytes. Restore requests/bytes unchanged28643/13662080. Native code/stack/Object bytes unchanged; executable17059856 to17057424bytes.

The general scalar contract improves364 to352 allocation requests and130389 to126749 requested bytes per evaluation, across128 original/decoded results. DEFLATE saves11 requests and1565bytes on each measured output. Every output and nontiming work/report field matches IR295. These are allocation results; concurrent qualification timers are ineligible for latency claims.

Validation:2 new differential actual-kernel contracts,9 existing composite tests,129 native-collection tests,438 GPU CI names with all438 parent names retained, image/caller geometry, parallel, original/decoded/restored and optional LLVM diagnostics. Full5000 plus31edges exact, exact linked Object audit, and full source inventories including all Rust/Cargo files match production/Engine/GPU tests. Initial test helper type errors and preliminary build receipts remain.

Five source experiments from the separate modular-ranges packet remain unselected for processing. Their fresh resource measurements are retained here: compact byte IDs reduce code by41172bytes and instructions by roughly0.4%, yet take more cycles; checked u32 arithmetic adds instructions; constant tables add roughly316KB of requested allocation. Source/code-size and instruction reductions alone did not yield a processing gain.

Remaining selected confirmation gap is0.395625ms,20.690% of Mech processing, against1.516500ms Rust. Next investigation is the internal binding/memory-planning breakdown of dictionary seed construction and key extraction. The constructor's measured post-JSON instruction effect is about0.1%, too small to close that gap.

PR990 integrated head remains IR242; this separately published source is3734388be9. Base PR973 was rechecked at94df733966:production remains R199; its R209 changes are an unadopted diagnostic, not a newly inherited production gain.
