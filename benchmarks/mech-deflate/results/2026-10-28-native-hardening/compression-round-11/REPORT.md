# Round 11: native read-only numeric leaf recurrences

The unchanged eight-byte, level-12 compressor measures **408.21× → 346.11× Roc**, with Roc = **1×**. The paired median falls **0.117155000 → 0.099332833 seconds**, **15.21% less elapsed**, or **1.1794× faster Mech**. This is a useful compiler gain; the remaining gap is still large.

## Revisions and unchanged experiment

- PR parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`; stack #934 → #935 → #939 → #973. Published round baseline: `1fb6b06dea1a20b0976f0a42f66288b48ec7e312` (projection-key implementation `89a48f961a0905a83d81c3acd97a9e93fa40b691`). Its executable is retained unchanged.
- Measured compiler: **`06885e0d9a1466ba477d642ee9fd9fd8510b06f2`**. `3a16ffadb96d4c246b4eba86969902ccf3575e9d` implements the representation/emission/image proof; `88ee934f882e4fc404abcd28dff0df6e0189e1c8` audits actual emitted hot graphs; `e57c6278eed2a3f3d2becf4da3bcab18ae65e174` fixes the post-entry account snapshot; `06885e0d9` supplies the completed qualification. Native IR **116**, work policy **91**, ABI **35**; canonical bytecode producer revision **14** stays unchanged.
- Caller source SHA-256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7` (4,138,627 bytes). Fresh ordinary source compilation and decoded re-encoding agree byte-for-byte. The 25-component assembly retains SHA `345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`.
- Same algorithm, tables, 262144-entry U16 tree, 1500513-word U32 cache, depth 300, nice length 258, near-EOF rules, six deliberate Huffman initializers, source allocation sites and caller reserve/clear/rebind discipline. The optional round-09 source variant is separate and was not used.
- Apple M1, arm64 macOS 15.6.1; Rust `nightly-2026-03-03`, LLVM 22.1.0. See `environment.json` and exact Cargo/library receipts.

The same harness and pinned mimalloc 2.1.7 object are used before/after, without allocation probes. Only `evaluate_into` is timed; JIT, reserve-13/clear/rebind, validation and disposal remain outside. Each side retains three measured calls, an excluded warmup and excluded extra call. The fresh reference median is **0.000287000 s**, over 20,000 observations. A separate process supplies the 2-second profile; all its observations are diagnostic. No compiler, qualification or other benchmark overlaps protected timing. Process times of 38.53/39.82 s include JIT and are not compression times.

| Identity | SHA-256 |
| --- | --- |
| Before executable | `30a5ab3e35fc9a4d68902ab73a6686a28e9b985bdbb0030dbabeec04c1022625` |
| After executable | `7d1553881fc896b541d93345170e49a01bd7bbcf15cdce0abb12522508962dcf` |
| Reference executable | `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324` |
| Pinned allocator object | `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874` |

Reference compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, package `671077457ff7ef3a3d44caf715a3fc595addf379`. Exact raw bytes `4b4c4a4e494d4bcf0000`, independent miniz_oxide inflation to `abcdefgh`, immutable input aliases, capacity-13 retention and terminal zero bytes/nodes pass on every retained call. **`same_backing` is false on all before/after calls**: the caller clears/rebinds the prior result, but compression copies the backing across the immutable public argument boundary. That compiler/runtime cost is timed. Physical output-backing reuse across that boundary is not established, and physical allocator activity is not claimed identical to the reference. **Eight bytes is the largest input timed this round.** Historical larger-input receipts retain their original revisions.

## Representation and execution

The proof follows all definitions and every recursive edge through existing exact-field, connected-use frame facts. Managed array fields must retain their original carrier identity. Swaps, replacement, mutation, escaping frames, constrained shape relationships and child loops decline selection. Original selected factories/templates remain authoritative; no function names, dimensions or contents select code.

Invocation entry validates current packed U8/U16/U32/U64 owners and geometry, retaining owners in existing frame lanes. Fresh views contain pointer, length, rows and columns. Changing scalar fields live in native SSA words. The generated body performs typed loads, one-based bounds checks, column-major-to-row-major address translation, checked arithmetic, branches and backedges. It preserves source guards, short-input zero getters, simultaneous next state, assertion/arithmetic/load order and `sym - base` order.

Only one native leaf loan runs, under an ordinary suspended parent. Entry and exit check its identity and the parent's active state. Finite fuel uses the existing invocation account. Current scalar fields synchronize into already admitted carrier lanes on exit/fallback/error; immutable owners remain retained. Ready-state invalidation ends view validity; pointer homes need not be physically zeroed. Bounds errors use the original selected operation at the original source site. Existing unwind, publication and retirement handle cleanup and retry.

Independent image restoration rederives closed regions, all-edge array provenance, factory associations, offsets, boundaries and prices. Corrupted views, scalar/view word counts, prices, missing proofs, swapped call bindings and mismatched type-slot keys/definitions are rejected. Shared JIT/Object emission is retained.

The unchanged full Object selects **45** read-only regions, all parented, with **72** typed load sites. The actual emitted hot CFG audit finds **zero call instructions** on successful paths before cold/fuel/exit boundaries. This is a structural audit, not a runtime counter. Original/decoded exact-source tests qualify `optimal-true-cost-literals` and `optimal-true-cost-extra`; the complete Object additionally retains its origins/proofs and decline reasons. Compiled selection is distinct from a count of dynamic invocations. Whole-run logical work falls **875610515844 → 789930572624** because native bodies replace ordinary construction/read transactions; entry and conservative native work remain charged under policy 91.

## Regression and allocation evidence

**128 scoped Rust tests pass:** 115 maintained component tests, one directly observed finite-fuel unit test, one real linked/AOT test, eight protected scalar correctness tests and three integer-bit tests. The prior component filters, expected outputs and allowances are unchanged. Twelve static contracts pass; late module-layout, unsafe-boundary, formatting and diff checks also pass. Warnings remain errors and the unsafe allowlist is unchanged.

The weighted reduction uses original/decoded artifacts and an explicitly declared finite diagnostic policy. It also runs under an ordinary parent which switches equal/different-shaped owners and scalar seeds, with inner lengths 0/1/2/8/64/256. Tests cover all four widths/high bits, empty/short inputs, actual shared backing between fields, rectangular resident agreement, rejected array-forwarding/escape cases, selected/unselected checks, fresh Boolean state, error order, publication, aliases, cleanup and retry.

| Elements | Requests before → after | Requested bytes before → after | Ordinary visits before → after |
| ---: | ---: | ---: | ---: |
| 0 | 245 → 245 | 39219 → 40715 | 36 → 6 |
| 1 | 252 → 245 | 39347 → 40715 | 93 → 6 |
| 8 | 301 → 245 | 40243 → 40715 | 492 → 6 |
| 64 | 693 → 245 | 47411 → 40715 | 3684 → 6 |
| 256 | 2037 → 245 | 71987 → 40715 | 14628 → 6 |

This isolated feature removes the post-key patch's **7 requests, 128 bytes and 57 ordinary visits per element**. The reviewed pre-key compiler had 83 requests/2440 bytes per element; that larger reduction combines two separately measured patches. New entry/view transport increases fixed bytes by 1496. Both artifact forms agree. No per-element general-runtime work is hidden in a smaller dispatcher.

A separate System/probe diagnostic initializes the real U16/U32 buffers in Mech, then reduces 0/1/8 positions. Its tuple result exports both buffers to inspect packed kinds, lengths and contents, so its allocation totals include those exports. After declared warmup, both forms remain at **2387 requests, 13273725 bytes, 21 ordinary visits** across lengths. Eight positions complete in **0.000827708 s original / 0.000824416 s decoded**. Work is 812565965; there is no resource denial and terminal bytes/nodes are zero. These are component observations, not compression throughput. A retained cold observation has 12 more metadata requests/768 bytes and is excluded from the warmed slope.

The finite-fuel observer forwards the actual generated helper protocol unchanged inside its existing audited test owner. It observes five genuine fuel exits per artifact, after 61/53/37/21/5 native traversals. Actual account debits and current fields agree: e.g. 61 traversals restore `left=3,total=183,expected=192`. Every loan is released before the next ordinary action. The insufficient remainder rejects, preserves the published result/aliases, cleans up and successfully retries. No test fuel override or production limit was added. Warm exact/one-short work tests are separate.

The linked test builds and runs an actual runtime-only consumer with unchanged Object/descriptor pins. Inputs **3 → 0 → 2** yield **4 → lexical rejection before division → 6**; prior publication, byte/node cleanup and recovery pass. Its closure excludes parser, Cranelift and compiler features. Pins, consumer feature closure and binary identity are recorded in the linked receipt.

All eight fresh direct scalar latency gates pass (ratios 0.99395–1.01998, maximum 1.05); allocations and bytes match exactly. Scalar-loop 0/64 counts/bytes have zero slope and all four loop timing gates pass. The protected scalar baseline is the retained round-08 executable, also used by round 10, rather than a reconstructed older rlib. **The historical round-10 decoded-U64 ratio 1.0526036 remains unresolved.** This independent changed-compiler cohort does not erase that failed observation or establish its cause.

## Commands, profiles and failures

All commands, exact library hashes, feature sets, nonzero counts, stdout/stderr and retained failures are in the packet. Primary commands use the **default** `benchmarks/mech-deflate/native-profile/Cargo.toml` consumer, release, locked/offline, `-j1`, five math operations. `--features linked-qualification` supplies process supervision; `--features unit-qualification` supplies the runtime compiler/catalog interfaces for existing unit helpers. Neither feature is enabled for primary/scalar timing. GPU dev-dependencies would activate the full catalog, so focused qualification compiles the actual maintained test sources against identified Cargo artifacts; adaptations/filters are recorded. Actual linked consumers are built and executed through Cargo.

- `cargo build --locked --offline --release -j1 --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --bins --message-format=json`: pass, 238.65 s. Core/Engine reuse their default-profile artifacts; the unhashed math output is rebuilt after profile switching. Exact Cargo messages are retained.
- Linked-profile builds: 1053.53 s, then 1048.65 s after adding the required process-supervision feature. Unit-profile builds: 118.10 s, then 73.85 s with the required routing/catalog interface. These are setup costs, not timing samples. Feature comparisons confirm no added Engine/math operations for unit support.
- Maintained component root, selected filters, `--test-threads=1`: **115 pass**, 49.94 s harness time (50.82 s process receipt).
- Maintained GPU library unit filter `numeric_fuel_exit_synchronizes`: **1 pass**, 0.15 s. Real linked filter `linked_readonly_numeric_child_restores_and_recovers --exact`: **1 pass**, 82.09 s including consumer build.
- `artifact_review` over pinned source/bytecode: pass, 56.64 s; no compression timing collected.
- `primary/run.py`: fresh reference, paired complete execution and separate profile pass. `primary/scalar-controls.py`: every fresh gate passes with unchanged criteria.
- Twelve architecture/catalog/layout/factory/warning/unsafe/format/inventory contracts: pass, 196.37 s; four late changed-file checks pass, 43.41 s.

Introduced development failures are retained and corrected: the initial metadata JSON macro exceeded Rust's recursion limit (split the metadata construction); the first native run funded from a pre-validation account snapshot (`e57c6278e` fixes it); focused harnesses initially omitted process supervision, source/catalog interfaces and test externs; one new fixture had a Rust boxed-slice type mismatch. Fixture attempts assumed named locals were published, attempted a document-only check/unsupported general constraint, compared cold and warm counters, and assumed a particular allowance necessarily reached a fuel exit. Final witnesses use explicit tuple output, ordinary Boolean results, declared warmup and observed real fuel boundaries. One exploratory observer matched a normal exit through the header-only site map and aborted; the final observer gates the actual fuel exit and uses checked lookup. None of these failed runs counts toward passing qualification. The earlier sysctl sandbox denial is environmental; the authorized CPU metadata read succeeds. Expected one-short/source-error denials are successful regression evidence. The historical scalar failure remains classified **unresolved**, not silently assigned environmental cause.

The all-primitives not/xor test is deliberately outside the selected catalog. The other 288 component tests, other 426 library unit tests, broader linked/generated applications, full block/window/full-file campaigns and full CI are not refreshed. CI is cancelled by request; this PR stays draft. The independent round-09 source experiment and historical larger-input receipts keep their original scopes.

## Profile and next bounded step

The separate profile has **1347** samples. Frame handling appears on **482** sampled ancestries, prepared frame-word admission on **333**, ordinary visits on **245**, and unsigned indexed replacement on **93**. Inclusive ancestry counts overlap and are not additive wall-time buckets or predicted speedups. Ordinary mutable/array-producing loops still pay those transactions. The next measured compiler step is private indexed-write recurrences using the established representation and real owner/alias boundaries; SIMD, broader producers and source rewrites remain separate.

The actual Object is retained privately (SHA `e12449a85862db009b43caace65cfa1b0b9a9ebbee1ca8a346207710008b6d95`). The packet contains its metadata, audit recipe and 305-line typed-load/backedge excerpts. A full 1,110,273-line disassembly remains private with its SHA in `disassembly.json`; binaries and the 4.98 MB compressed full listing are excluded from Git. All retained small measurement/qualification receipts are archived, including failed attempts. `manifest.json` inventories the checked-in packet.
