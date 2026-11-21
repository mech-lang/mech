# Source-to-block mapping and broad matcher work

The selected build remains **63,468 B native text, 276,072 B stripped executable,
81/81 Roc and 63/81 libdeflate**, preserving all required named point wins.
This change adds an optional compiler diagnostic and establishes no new runtime
win. Original 2,719-line source and canonical bytecode are unchanged.

`MECH_INSPECT_NUMERIC_SITE_BLOCKS=/output/475-vector.json` records original
numeric source sites and their pre-optimization CLIF blocks. The filename selects
the phase and kind (`vector`, `no-growth` or `general`). It does not set source
locations, alter IR, enable the general observation route, weaken admission, or
change scalar instruction emission. Eight actual public normal/checked source
replays reproduce all three artifacts exactly. All **79 optimized scalar tests**
pass. Compiler source, immutable build snapshots and raw test output are archived.

## Actual native scope

Both selected sidecar-off/on objects, full metadata, bytecode and linked workers
are byte-identical to the published selected recipe. The AArch64 backend rejects
disassembly together with post-emission block layout. Consequently the separate
raw tracing pair removes only the matcher's machine-layout profile, retaining its
register weights and the other phases' profiles. That pair also has identical
artifacts, linked workers and VCode off/on. Its source map equals the selected
pre-optimization map.

The raw tracing object has **63,452 B** text. Its counts and offsets belong to
that actual object; they are **not selected-build instruction counts or timings**.
The mapping joins 2,169 emitted source records to 1,176 VCode labels, with 982
surviving mapped source blocks. Deleted sites, merged labels and empty blocks are
recorded rather than counted as independent observations. Actual machine-head
anchors are used below, with absent counts treated as zero only after establishing
the static head; the nearest visited block is never substituted for a zero block.

| Input | Mech private search-tree steps | C search-tree steps | Mech / C skip-tree steps | Collection-10 handoffs |
| --- | ---: | ---: | ---: | ---: |
| Natural 1,024 | 755 | 755 | 8 / 8 | 0 |
| Natural 4,096 | 7,392 | 7,392 | 44 / 44 | 0 |
| Natural 8,192 | 15,677 | 15,677 | 0 / 0 | 0 |
| Random 4,096 | 118 | 118 | 0 / 0 | 0 |
| Random 16,384 | 2,044 | 2,044 | 0 / 0 | 0 |
| Random 65,536 | 22,842 | 22,843 | 0 / 0 | 1 |
| Random 1,048,576 | 477,306 | 477,320 | 0 / 0 | 31 |
| Distribution change | 2,225 | 2,225 | 0 / 0 | 0 |
| Overlap 320 | 2 | 2 | 312 / 312 | 0 |

The search/skip anchors match on seven inputs. The larger inputs leave a scope
remainder of one/fourteen search steps; checked continuation work is outside this
private phase and has not been measured here. No attribution of that remainder,
whole-matcher work equality, or equal bytes compared is claimed. Collection 10 is
the already identified SIMD tree-rebase handoff, not a newly missing optimization.
The C counters are exact copies of the earlier published instrumented C traces.

This directs the next broad experiments toward register reloads, copied loop
state and repeated checks per step. The raw 1-MiB diagnostic records 18,128,229
stack loads and 1,696,938 stack stores, leaving substantial code-generation work
to investigate. It does not establish how much latency any individual change can
remove. Register-profile derivation also needs complete static block heads so
unvisited blocks do not inherit a nearby hot block's count.

## Complete-build timing

Apple M1/macOS 15.6.1/arm64; eight balanced serial fresh-process orders, source
through final executable link. All compilers and fixed support are equally
prebuilt. Profiling, tests, Rust bootstrap and stripping are outside these clocks.
Every trial checks the generated artifacts and final stripped executable.

| Actor | Median seconds |
| --- | ---: |
| Previous compiler, selected recipe | 2.199101 |
| Compiler with diagnostic disabled, selected recipe | 2.199818 |
| Roc | 2.988482 |
| libdeflate C | 0.481730 |

The new compiler's paired median is **0.999881× previous / 0.736360× Roc /
4.562243× C**. No compilation-time change is established. Its median phases are
0.702677 s parsing, 0.218351 s frontend and 1.203641 s native pipeline; remaining
artifact/encoding/link/process work is included in the complete-build total.
The compiler's separate Rust bootstrap takes 222.835 s. All **324 actual timed
stripped-image executions** have saved streams and pass independent inflation.

`matched-builds/compiler-bindings.json` binds each actual timed command and binary
SHA to its own immutable compiler source/build snapshot. Two descriptive source
fields of the new actor were inherited from the reference when drafting the raw
plan. That original plan and all measurements remain unchanged; the explicit
bindings identify the correct source for each binary.

## Reproduce and verify

From this directory, with the repository's supported Rust/macOS toolchain:

```sh
python3 build_compiler.py /tmp/mech-block-map-compiler
python3 reproduce.py /tmp/mech-block-map-compiler/release/compile_profile /tmp/mech-block-map-selected --role selected-mapped
python3 reproduce.py /tmp/mech-block-map-compiler/release/compile_profile /tmp/mech-block-map-checked --role selected-mapped --checked
python3 verify.py
```

Use the compiler path reported by `build_compiler.py`; source-built binary hashes
may differ between hosts. Recipes pin original source, profiles and expected
object/full-metadata/bytecode hashes. `raw-trace-off` and `raw-trace-mapped` reproduce
the separate diagnostic artifacts. `verify.py` requires only Python and checks
all archive members, compiler bindings, actual native identities, source/VCode
joins, translated counters, the nine C comparisons, 162-success/resource/cleanup
gates, six error cases, twelve recoveries, the 4,752-B measured stack, scalar
tests, every complete-build result and all 324 saved deployment streams.

The selected recipe's full 81/63 runtime qualification remains in
[the preceding study](../encoder-size-gap/packet-family-root-study/README.md).
Its exact worker is unchanged. **64 C wins is next; 81 remains the full goal.**
