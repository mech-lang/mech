# Cached join values

Two equal, already cached predecessor values can satisfy a sealed Cranelift
join directly. This screen skips creating a transient block parameter.
The checked root creates 1,399,985 → 1,382,715 values and retains the same
165,899 after compaction. Both pinned objects, metadata files, and workers
match their controls byte for byte.

In twelve balanced native-only pairs, median emission was 1.76846 → 1.77066 s.
The matched trials show no speed improvement. The exact compiler patch and
measured calls remain available as evidence of the discarded approach.
