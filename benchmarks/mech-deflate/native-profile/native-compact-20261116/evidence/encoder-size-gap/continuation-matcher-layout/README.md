# Matcher fallthroughs with shared continuations

The LLVM-free opt-in recipe emits **67,648 B**, 776 B below the qualified 68,424 B encoder. Encoder text is 1.172× Roc and 1.824× libdeflate.

The match collector keeps its checked operations and changes the order of its machine blocks. The compiled checked entry and every other encoder atom keep the same bytes and relative relocations. The 8 KiB fixture was excluded from layout weighting. The training includes other lengths from the same natural-text corpus. The block model estimates 86,324 → 57,039 unconditional jumps for natural 8 KiB. The separately instrumented laid-out workers executed 86,324 and 57,039 jumps.

[Profile and 8 KiB estimate](matcher-natural-layout-prediction.json) · [Measured instruction counts](held-out-matcher-census.json) · [Exact candidate and source recipe](candidate.json)

## Compression runtime

The linked proof checks 162 success observations and six checked failures. Each runtime order uses 96 balanced blocks per case. Timings cover compression through private heap cleanup.

| Build | Scope and order | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate |
| --- | --- | ---: | ---: | ---: |
| Paired spills | focus forward | 15/15 | 12/15 | 1.00787× |
| Paired spills | focus reverse | 15/15 | 12/15 | 1.00919× |
| First scorer address | focus forward | 15/15 | 12/15 | 1.00982× |
| First scorer address | focus reverse | 15/15 | 12/15 | 1.01515× |
| Matcher fallthroughs | focus forward | 15/15 | 13/15 | 0.99237× |
| Matcher fallthroughs | focus reverse | 15/15 | 13/15 | 0.99969× |
| Matcher fallthroughs | full forward | 81/81 | 62/81 | 0.99670× |
| Matcher fallthroughs | full reverse | 81/81 | 62/81 | 0.99851× |

V3 returned **Retain** for the complete matcher orders.

[Full replay](full-replay.json.gz) · [Complete forward observations](full-forward-observations.tar) · [Complete reverse observations](full-reverse-observations.tar) · [Linked checks](linked-checks.json)

## Fresh source through final link

Apple M1 ran 10 balanced builds per recipe with each compiler and fixed deployment support prebuilt. The link runs in every trial. Executable sizes are after stripping.

| Build | Median source to link | Stripped executable |
| --- | ---: | ---: |
| Qualified field build | 2.304 s | 276,056 B |
| Shared continuations | 2.298 s | 276,056 B |
| Shared continuations with matcher fallthroughs | 2.301 s | 276,056 B |
| Roc | 2.989 s | 191,912 B |
| libdeflate | 0.480 s | 155,752 B |

Paired build time is 0.771× Roc and 4.789× libdeflate.

[Matched build and phase timings](deployment.json)

## Reproduce the encoder

Apply the [shared continuation compiler patch](../shared-continuation-returns/compiler.patch.gz) and build it using the linked study’s replay commands. Use the resulting baseline rebuild as the first argument to this directory’s replay.py and supply a fresh output directory as the second argument. The new receipt pins the encoder object, metadata and canonical bytecode.

[Baseline compiler setup](../shared-continuation-returns/README.md#replay) · [Verified candidate replay](verified-replay.json)
