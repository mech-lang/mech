# Keep shared cold returns in the checked caller

Two continuation regions in the 68,424 B encoder share a terminal return
with an outside predecessor. Each return uses a local status and a
constant error origin. The outliner copies the return into the helper and
keeps the original for the outside edge. It checks local value definitions,
bounded origins, captures, and output uses before replacing the body.
[Compiler patch](compiler.patch.gz) · [Continuation inventory](shared-terminal-returns-size.json)

Sharing the two continuations saves **752 B** of encoder text.
Sharing all six additional families reduced checked entry text by
**3,440 B** and increased total encoder text by **976 B**. The measurement
includes the helpers and caller setup. Only **1** of the
six families saved text in isolation. The smallest measured encoder in
this study contains **67,664 B**.
[Individual sizes](family-costs.json) · [Checked entry and helper sizes](size-attribution.json)

## Source through final link

Apple M1 ran **10** matched fresh builds per recipe with a prebuilt
compiler and fixed deployment support. The control reproduced the
68,424 B encoder that passed both complete runtime orders.

| Build | Median source to link |
| --- | ---: |
| Combined scalar and field control | 2.306 s |
| Shared cold returns | 2.304 s |
| Shared cold returns with one additional continuation family | 2.302 s |
| Roc | 2.990 s |
| libdeflate | 0.481 s |

[Matched measurements and phase breakdown](deployment.json)

## Compression runtime

The checked contracts cover 162 linked success observations and six
checked failures. Each timing screen covers 15 cases per order. Each
complete order covers 81. Natural 8 KiB is paired compression time
relative to libdeflate.

| Encoder | Scope and order | Roc wins | libdeflate wins | Natural 8 KiB |
| --- | --- | ---: | ---: | ---: |
| Shared cold returns | focus forward | 15/15 | 12/15 | 0.99841× |
| Shared cold returns | focus reverse | 15/15 | 12/15 | 1.00365× |
| Shared cold returns with one additional continuation family | focus forward | 15/15 | 12/15 | 1.00026× |
| Shared cold returns with one additional continuation family | focus reverse | 15/15 | 12/15 | 1.00429× |
| Shared cold returns with one additional continuation family | full forward | 81/81 | 62/81 | 0.99933× |
| Shared cold returns with one additional continuation family | full reverse | 81/81 | 61/81 | 1.00521× |

Shared cold returns received **Unresolved** in the focus comparison. [V3 findings](shared-terminal-returns/focus-screen.json)
Shared cold returns with one additional continuation family received **Unresolved** in the focus comparison. [V3 findings](continuation-family-2/focus-screen.json)
Shared cold returns with one additional continuation family received **Reject** in the full comparison. [V3 findings](continuation-family-2/full-screen.json)

[V3 policy](../../../qualification-policy-v3.json) · [Earlier 62-win compiler replay](../dominating-definitions/README.md)

In the instrumented natural 4 KiB run on the qualified 68,424 B build, the scorer used **48.4%** of elapsed compression time and match collection used **29.0%**. The two instrumented hot bodies matched their compiled code and relocations. [Phase profile and artifact identities](natural-phase-profile/result.json)


## Replay

Apply the patch in an isolated checkout at
`06f5cb711473e39c5172506c5858e080b0364b43`. Each recipe verifies the
emitted encoder object, canonical bytecode, and metadata.

~~~sh
# Set study to this evidence directory and run from the isolated Git checkout.
gzip -dc "$study/compiler.patch.gz" | git apply
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-cold-return-control --variant frame-copies --compiler-opt-level 3 \
  --direct-source --preselect-root-inlining --skip-refused-general-ssa \
  --target-dir /tmp/mech-cold-return-target
python3 -B "$study/shared-terminal-returns/replay.py" \
  /tmp/mech-cold-return-control /tmp/mech-cold-returns
~~~
