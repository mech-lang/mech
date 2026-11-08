# Borrowed-key caller census, verified loading, and private numeric writes

IR300–IR302 extend IR299's borrowed String-matrix cell to retained Map insertion bridge. Cranelift remains production, with no LLVM dependency. The Mech source and numeric algorithm are byte-identical to IR295. IR295 remains the selected processing compiler: the new processing-speed result did not independently confirm.

| Implementation | First processing ms | First / fastest Rust | Confirmation ms | Confirmation / fastest Rust |
|---|---:|---:|---:|---:|
| Fastest eligible Rust | 1.552500 | 1.000000x | 1.536334 | 1.000000x |
| IR295 selected | 1.914375 | 1.233092x | 1.908500 | 1.242243x |
| IR297 constructors | 1.933416 | 1.245357x | 1.925666 | 1.253416x |
| IR299 borrowed cells | 1.929500 | 1.242834x | 1.918292 | 1.248617x |
| IR300 checked Object loading | 1.920083 | 1.236768x | 1.912709 | 1.244983x |
| IR301 ready-loan caller census | 1.922541 | 1.238352x | 1.904125 | 1.239395x |
| IR302 private-write-first | 1.897333 | 1.222115x | 1.905750 | 1.240453x |

The fastest Rust changes from rust-ordered-mask-tables256 in the first cohort to rust-ordered-ordered-mask-tables in confirmation. All 22 retained Rust controls ran; 20 are eligible for the continuous post-JSON processing comparison. Preinitialized pools remain whole-process-only. Each cohort contains 868 timing outputs and 203 separately measured resource outputs, all exact against Rust: 2,142 total. Both randomized schedules, every first/adverse sample, all output hashes and content-addressed output payloads are preserved. No build, test, profile or archive overlapped either measurement cohort.

The apparent ratio improvement from C25's 1.257635x selected baseline to 1.242243x is not a compiler gain: unchanged IR295 increased from 1.899292 to 1.908500 ms while the fastest Rust reference increased from 1.510209 to 1.536334 ms. The latest selected gap is 372.166 microseconds, requiring about 19.50% less Mech processing time. The Rust comparison includes input-dependent preparation, allocation and worker lifecycle; the Mech interval includes admission, all native loan setup, worker create/run/join, cleanup and materialization. Restoration and JSON IO are reported separately and included in whole-process time. Whole-process regression is not an acceptance gate.

IR300 removes one duplicate full-plan validity pass only when decoding genuine metadata bound to the already verified linked Object. Fresh complete-use, definition, adjacency and lifetime facts are still derived. Independent metadata loading retains full validity checks; a tampered Object binding rejects before proof derivation. Compile-time preparation also retains validation. This is a checked-loading compiler gap fix, not trusted serialized optimization authority.

IR300's restoration improvement repeats: paired -28.9147% first and -29.3041% confirmation, 31/31 favorable in both. Confirmation median restoration is 11.726250 -> 8.296542 ms. Paired whole-process changes versus IR299 are -6.8875% (95% interval [-7.7884,-5.0641]) and -8.6160% ([-11.2967,-6.8264]); confirmation medians 41.526458 -> 37.869667 ms. Restore allocations drop 62,607 -> 37,758, requested bytes 58,510,822 -> 15,417,036, and instructions 180,421,375 -> 131,064,965. This repairs most of IR299's restore regression, but IR297 restoration remains faster at 7.991333 ms. IR300 establishes no repeated processing win.

IR301 lets an actually admitted borrowed matrix-key loan participate in the existing complete native-loop caller census. The census is attempted once after a new loan becomes ready, using the current producer visit and owning iteration. All loop operations must be supported and all applicable loans must exist. Refusal, unsupported operations and emission ablation retain the canonical/full-census path. Current shared holdings and cache/table/key residency still contribute on every operation. Added metadata inspections are explicitly charged; work policy advances from 118 to 119 without repricing scalar or DEFLATE execution.

Actual full5000 original/restored diagnostics observe 55 cached and three full caller censuses, 28 borrowed cells, zero canonical String Values and 28 retained insertions, with exact output. This capability has no established full-processing or resource win against IR300: paired processing +0.1428% (interval [-0.3951,+0.6272]) first and -0.1814% ([-1.0776,+0.2748]) confirmation. Its first whole-process improvement also does not repeat. No elapsed saving is inferred from the observed cache hits.

IR302 changes native numeric write emission: check existing private ownership before reading/comparing the old cell. An already-private bank stores directly. An unowned bank still loads and compares, preserving equal-write and zero-to-zero no-op behavior and the original checked first-change materialization boundary. The change applies to generated native numeric bodies, not a host implementation of posting scatter. Checked increment, narrow type admission, bounds, finite work, first-change incoming/current SSA state, aliases and cleanup remain checked. The previous emission order is retained as an ablation.

The private-write processing result is mixed and retained. First versus IR301: paired -1.0099%, interval [-1.5462,-0.6165], 29/31 favorable. Confirmation: -0.1600%, interval [-0.7414,+0.2646], 18/31 favorable. Against selected IR295: first -0.7862%, interval [-1.2689,-0.2561]; confirmation -0.1562%, interval [-0.7896,+0.7659]. There is no independently confirmed processing improvement, so there is no processing promotion. The native emission change remains a resource/composition candidate: instructions decrease by 325,294 first and 271,927 confirmation against IR301; code decreases 316 bytes, while native stack increases 16 bytes. Cycles do not improve in confirmation.

Confirmation continuous post-JSON resources (seven samples, separate instrumented executable, all worker threads):

| Implementation | Allocation requests | Requested bytes | Additional peak heap B | Total peak requested heap B | Retired instructions | Cycles |
|---|---:|---:|---:|---:|---:|---:|
| Fastest Rust | 58 | 6,825,713 | 5,648,124 | 7,703,209 | 17,864,425 | 5,953,438 |
| IR295 | 1,989 | 7,873,166 | 5,763,262 | 14,106,637 | 34,657,132 | 11,077,907 |
| IR297 | 1,973 | 7,871,350 | 5,763,262 | 14,106,636 | 34,635,365 | 11,095,040 |
| IR299 | 1,751 | 7,861,191 | 5,763,966 | 14,107,500 | 34,596,085 | 11,175,820 |
| IR300 | 1,751 | 7,861,191 | 5,763,966 | 14,107,496 | 34,552,036 | 10,928,085 |
| IR301 | 1,751 | 7,861,191 | 5,763,966 | 14,107,497 | 34,607,040 | 11,039,189 |
| IR302 | 1,751 | 7,861,191 | 5,763,966 | 14,107,490 | 34,335,113 | 11,134,969 |

IR302 peak RSS is 27,770,880 B, physical footprint 16,974,184 B; fastest Rust 6,111,232 B and 5,538,128 B. Its whole-process median is 38.011250 ms (1.531849x fastest whole-process Rust), versus selected IR295 38.459333 ms. The full process and restoration figures remain visible; neither is used to manufacture a processing gain.

Native code: IR300/301 1,031,460 B, stack 1,568 B, object 5,781,184 B; IR302 1,031,144 B, stack 1,584 B, object 5,780,800 B. Executables 17,092,352 / 17,092,944 / 17,092,928 B, respectively. All have 49 numeric functions, four query workers and eight numeric worker regions. The runtime GPU library is compiler-free. Each ordinary and instrumented executable passes strict same-Object text/relocation/metadata audit v9. No source algorithm was moved into an adapter.

Qualification: IR300 all 445 native CI names; IR301/302 all 446, preserving the original 438. Focused proof/runtime contracts number seven/eight/nine respectively, plus actual-source and full5000 original/restored tests. Existing first-change tests exercise both emission orders across u8/u16/u32/u64, changing/equal private writes, two arrays, real input aliases, changing current SSA values, finite clone caps, denied helper lanes, publication, cleanup and retry, with equal logical work. Retained map, image/caller geometry, UTF8/classification and range contracts pass. Each version passes full5000 plus 31 edge outputs, strict linked Object audit, 128 scalar and five DEFLATE cases. Every scalar/DEFLATE nontiming field, including allocations, bytes, work and visits, is identical to IR297. Concurrent qualification timers are ineligible for latency claims.

Initial IR300/301 linking ran out of disk space; failed receipts remain. Only failed build fragments were removed; historical compiler/test executables were losslessly compressed with original SHA256/mode manifests, preserving all source and primary outputs. No primary measurement had started. IR302's initial recipe launcher ran before recipe preparation completed; it failed before compiler execution, and its corrected launch and note remain. No first/adverse observation was discarded.

Stage A is complete. Stage B (borrowed map/keys through the private argument frame) and Stage C (generic UTF8 nonempty-seed dictionary retained through lookup) are still unimplemented and remain the primary target. The key view must own an immutable Map snapshot independently of the legitimate original-handle retirement into the private frame. Equal key sets must not authorize reuse of overwritten dictionary values. The existing empty-seed, short-key FrozenPackedMap is not a generic substitute. The separately attributable completed-producer element-range proof into scatter is also pending. Recent adverse compact-source screens remain retained; no unchanged failed screen was rerun.

Source commits: IR300 dd94af7fc6d2777b738b496e13c965b81c416353; IR301 fab86d9b5a5d6f5b990789256e3d4b4038363277; IR302 cb32dcd614d69429255b00ec3a720524df77bd64. Combined research branch codex/related-private-write-first-20261108 descends from the retained IR299 evidence; source-only parent branches are published for attribution. PR990's integrated head remains IR242 and targets the branch for PR973. This packet does not claim research integration or Rust parity. All three source inventories, deduplicated source content, source patch, build/qualification receipts, ordinary/instrumented adapters, Object/metadata/audits, randomized cohorts and exact outputs are included. The goal remains active.

A fresh 10-second executable sample uses the unchanged linked IR302 Object in a diagnostic adapter with 10,001 repeated fresh native invocations, then an exact full5000 output. The same-Object audit passes. It still exposes private-frame input admission and footprint/work-accounting paths alongside generated numeric kernels. Raw inclusive stack entries and conservative extraction are retained; they overlap, include worker/wait samples, and are not additive phase costs or predicted savings. Repeated admission/export/materialization are excluded from that diagnostic loop, so its duration is not an authoritative processing comparison.
