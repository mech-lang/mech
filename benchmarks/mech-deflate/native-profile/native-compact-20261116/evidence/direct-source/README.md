# Direct source compilation

`rebuild.py --direct-source` uses the existing single-process compact diagnostic
compiler. It passes the in-memory Program artifact and source catalog to Cranelift.
This removes the emitter's bytecode reload, decode, canonical re-encode, and
second catalog construction. The driver writes the same canonical bytecode
receipt once. The frontend syntax tree, lowered source product and encoded
byte buffer are released before native emission.

## Apple M1 source through final executable link

Six balanced orders use the same source, Rust opt-level 3, fixed runtimes,
allocators and link inputs. One warmup per actor and the profiled trials are
excluded from the medians. Compiler and adapter construction are prebuilt.

| Recipe | Encoder text | Encoder object | Executable text | Executable file | Split | Direct |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Mech retained | 72,520 B | 160,136 B | 475,444 B | 956,056 B | 4.07355 s | 3.99144 s |
| Mech smaller | 71,960 B | 159,576 B | 474,884 B | 956,056 B | 4.08628 s | 3.99930 s |
| Roc | 57,696 B | 79,160 B | 150,416 B | 230,600 B | 2.98598 s | |
| libdeflate | 37,080 B | 52,128 B | 129,900 B | 210,168 B | 0.48232 s | |

The smaller encoder's build takes **1.339× Roc** and
**8.292× libdeflate** time. Its encoder text is **1.24723× Roc** and
**1.94067× libdeflate**. Direct compilation reduces its matched median by
**2.13%**. The paired fractional change has a 90% bootstrap interval of
**-2.21% to -1.33%**, using 8,000 resamples stratified
by relative actor order. Median peak compiler RSS grows from **382,443,520 B**
to **401,408,000 B**, an increase of **18,964,480 B**.
The smaller direct build is faster in five of six orders. The first order is
4 ms slower overall and spends 7 ms longer in the final link.

The direct smaller build records **2.108 s parsing**,
**0.061 s catalog construction**,
**0.233 s source lowering**,
**0.010 s artifact formation and encoding**,
**1.518 s native emission**, and
**0.042 s final linking**. The separate Roc profile
records 1.645 s specializing and 0.809 s in LLVM optimization and emission.
The separate C profile records 0.074 s frontend and 0.438 s backend.
These diagnostic runs have their own wall time. The C trace includes nested totals.

[Input identities and trial orders](matched/plan.json) · [Raw build trials](matched/timing.json.gz) ·
[Phase logs](matched/profiles/) · [Report](report.json)

## Artifact checks

Both paths reproduce the canonical bytecode, encoder object and complete
metadata on every trial. The matched executable is byte-identical within each
recipe. Separate links recreate the historical retained, smaller, and aligned
benchmark workers exactly with the original runtime and adapter. The retained
and smaller workers carry their recorded **81/81 Roc** and
**61/81 libdeflate** wins in each full order. The aligned recipe keeps its
earlier runtime decision. The first aligned link used a different output
basename and acquired that name in its ad hoc signature. Linking under
`mech-worker` reproduces the pinned file.

All seven maintained recipes reproduce their pinned objects and complete
metadata. The smaller recipe also links the same standalone worker in
both modes. There are **1,134 exact encoder calls** across the historical,
matched and recipe workers. Both compiler drivers reject the malformed
source check before writing bytecode. Managed and frontend-only modes
produce identical bytecode on the scalar fixture.

[Historical and matched worker checks](workers/) · [Recipe checks](recipe-checks/) ·
[Driver boundary checks](boundary/receipt.json)

Run from the repository root on Apple ARM64.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-native-direct --compiler-opt-level 3 --direct-source \
  --variant fresh-owner-counts-cold-tails --preselect-root-inlining
```

Supply the fixed allocator object via `--mimalloc-object` to link the
standalone worker and check every fixture. The separate benchmark sources
and prebuilt adapter inputs are identified in the raw matched-build plan.
The archived harness used [the existing peer build driver](../native-pass-work-reuse/as-run/compare-builds.py).
