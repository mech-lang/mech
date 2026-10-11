# Additive bounded Uiua k-nucleotide native counting

This optimized candidate retains the original parser, all seven widths
`1,2,3,4,6,12,18`, and one complete growing map for each width. Each chunk
contains at most 65,536 window start positions, with `width-1` extra characters
so windows crossing a chunk boundary are retained. The next chunk advances by
the number of starts, including the final partial chunk.

The visible Uiua application uses ordinary native `classify` to visit every
window in a chunk. Ordinary `group first` obtains each distinct complete key;
`group length` counts every classification index. The application folds a
weighted generic map update once per distinct chunk key, adding that key's
entire count to its retained map. No width uses a query-only map, fixed bucket
table, host helper, or assumption about the input's contents or period. Native
source excerpts pin the primitive group specializations, classification loop,
map equality/collision probing, dynamic growth, and key copying.

The source is 29 nonblank, noncomment physical and normalized LOC. Every
independent binding/data declaration has its own line; ordinary pipelines are
single expressions. The unchanged multiline frequency formatter keeps its
original physical formatting.

Preparation and validation from this directory:

```sh
python3 prepare.py
python3 validate.py
python3 prepare_manifest.py
```

Preparation derives the candidate from the explicitly pinned original bounded
source, which remains unchanged. Installation uses `../install_uiua_runtime.py`;
the official Uiua 0.19.1 ARM64 release, archive hash, executable hash and runtime
source archive are recorded in the manifest and `../uiua-install-pins.json`.

All validation/reference work runs under `/tmp/mech-clbg-timing.lock`. Fourteen
fixtures check complete canonical output against an independent scalar Python
counter. They include the canonical `n=1000` input, ties, missing/empty/short
records, case/comments/CRLF, arbitrary DNA, every six-base key, and lengths
65,535/65,536/65,537/65,553/65,554/131,091. The high-diversity fixtures exercise
growing maps and collision handling; the official runtime does not expose a
collision counter, so native collision counts are not claimed as observed.

Separate diagnostic sources retain every original expression, print each
chunk's width/start/count, its number of distinct keys and total grouped weight,
then serialize all seven complete final maps. Every key and count must match
the independent counters. Traces prove the exact ascending start partition,
the 65,536-start limit, every window exactly once, complete weighted merging,
and retained maps across repeated-key chunk boundaries. Diagnostic printing and
serialization are never part of the timed application.

No full workload is measured by this package. At canonical full `n=25000000`,
the THREE record contains 125,000,000 bases: all 874,999,961 windows must be
counted. Performance remains unmeasured, especially when long windows are
mostly distinct. The original compact/bounded sources and timing evidence are
preserved. A new frozen audit and qualifying candidate/fresh-control samples
are required before chart admission.
