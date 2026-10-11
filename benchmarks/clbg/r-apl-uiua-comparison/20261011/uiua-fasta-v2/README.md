# Additive Uiua FASTA output batching

This optimized candidate writes each bounded sequence chunk with one ordinary
stdout write. The original optimized source has 24 LOC; this candidate has
34 LOC, including every formatter and ALU chunk helper. The original compact
source remains unchanged at 22 LOC. Original sources, runtime pins, frozen
manifests, and timing evidence remain preserved.
Both original applications remain measured and selected. This is an additional
optimized candidate; actual results determine the eventual endpoints.

The formatter builds a complete native gather index for a chunk of `L`
characters. Its output has `L+ceil(L/60)` characters. At output position `j`,
the ordinary character index is `j-floor(j/61)`; every 61st position and the
last output position select a single appended LF instead. Native arithmetic
and gather construct the whole result, followed by `&w` to stdout handle `1`.
There is no callback or write for each 60-character line. Empty chunks emit
no bytes; complete lines and the final partial line each have exactly one LF.
The character array is encoded as UTF-8 by the ordinary runtime write operation,
so character width and non-ASCII encoding remain separate.

All three records use chunks of at most 65,520 characters. That limit is a
multiple of 60, so every non-final chunk ends on a line boundary. ALU chunks
carry their global character offset and gather from the same repeated ALU
literal; the pattern does not restart at a chunk boundary. The canonical
headers, record order, and `2n/3n/5n` sequence lengths are unchanged.

The random-generation block remains byte for byte identical to the original
optimized source: generic modular affine prefix scan, all requested LCG draws,
seed propagation between TWO and THREE, direct binary64 division by 139,968,
and unscaled binary64 cumulative probability comparisons. There is no
RNG-period shortcut, exhaustive production lookup table, host helper, or
benchmark-specific runtime kernel. At full `n=25,000,000`, the application
emits 250,000,000 bases and performs all 200,000,000 random draws.

Preparation commands from this directory:

```sh
python3 prepare.py
python3 prepare_manifest.py
python3 validate.py
python3 prepare_manifest.py
```

The first manifest generation can record explicit
`prepared-pending-locked-validation` metadata before validation exists. It
does not execute Uiua. A completed manifest requires a passed validation
report with 27 production output comparisons, 6 work checks, 13 generic
formatter checks including Unicode, and 2 exhaustive CDF classification
checks over states `0..139967`. The exhaustive CDF fixtures are diagnostic
only; their results are not used by the production source. All source,
fixture, input, stdout, stderr, runtime, validator, and proof files reported
by validation are pinned.

Validation uses `/tmp/mech-clbg-timing.lock` at a natural trial boundary and
records no full performance run. Every candidate remains ineligible for a
chart until explicit selection through a new frozen audit and qualifying
fresh full candidate/control measurements.
