# Bounded GNU APL nucleotide candidates

These additive sources avoid retaining the entire FASTA input as boxed APL
characters. They read 64 KiB buffers, handle split headers/comments and CRLF,
select the THREE record, and count all overlapping windows at all seven widths.
All seven independently growing maps remain live through the final output.
Each width carries only its last `k−1` codes into the next chunk.

The compact form retains scalar rolling-key map updates. The optimized form
builds every window key with ordinary bounded arrays, grades the incoming keys,
combines equal-key amounts, and probes the growing tables in batches. Existing
and incoming hash collisions are both resolved explicitly with exact key
equality. A table resize reinserts every old key and its complete count. This
uses no native module, fixed key universe, query-only counter, input-period
cache or workload-specific executor. Every application helper is counted.

The normalized sources contain 173 and 193 LOC respectively, using one
independent statement per line. Only blanks/comments and standalone closing
`∇` delimiters are excluded, consistently with the existing APL suite.

```sh
python3 prepare_sources.py
python3 validate.py --wait-for-lock
python3 prepare.py
```

Validation is serialized with full benchmarks. It compares complete canonical
and adversarial output, exercises header/sequence/comment chunk boundaries,
and checks generic collision/growth/duplicate batches. Additional independent
fixtures compare every key/count of all seven maps after one chunk and tiny
chunk splits. Frozen original application/runtime and regex-v3 pins are
verified unchanged. Full timings remain the parent harness's responsibility.

The memory improvement is bounded input and intermediate array storage. Map
storage still depends on actual distinct keys. In a hypothetical input with
more than 105,696,460 unique entries in one map, this growth policy reaches
301,989,887 slots, requiring 9 GiB for its two 16-byte APL-cell arrays alone.
That is a conditional capacity bound, not a measured claim about canonical
FASTA data. No period assumptions or precomputed distinct counts are used.
