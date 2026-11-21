# Paired Apple M1 runtime comparison

The exact checked-regions and compact-match-equality workers ran alongside
the maintained 71,192-byte frame-copies worker. Each order covers all 81
named cases with 96 balanced blocks. Each original worker performed fresh
compression and checked exact output, independent inflation, borrowed aliases
and private ownership cleanup. The two sweeps share a cohort and use every
permutation of the three workers equally. [Plan](plan.json) ·
[Forward blocks](forward-balanced-blocks.json.gz) ·
[Reverse blocks](reverse-balanced-blocks.json.gz)

| M1 recipe | Established slowdowns forward / reverse | Established in both orders | Unresolved forward / reverse |
| --- | ---: | ---: | ---: |
| Checked regions | 6 / 7 | 4 | 3 / 4 |
| Compact match equality | 6 / 7 | 6 | 3 / 2 |

The paired geometric means and per-case 90% bootstrap intervals compare each
recipe directly with frame-copies. A lower bound above the 0.5% measurement
resolution establishes a slowdown. An upper bound at or below that resolution
resolves the case up to measurement precision. The remaining intervals are
unresolved. These are per-case intervals. Both recipes remain opt-in.
[Every case and interval](case-results.json) · [Replay checks](result.json)

Both archived recipes separately qualify at 81/81 Roc and 62/81 libdeflate
in both V3 orders. This three-worker measurement establishes the comparison
with frame-copies. [Runtime qualification](../result.json)

## Matcher instruction counts

The measured 16-byte-only matcher executes 15,994 more unconditional branches
on prefixed random 4 KiB and 31,231 more on prefixed random 8 KiB than
the 64+16-byte matcher. Those jumps account for the full instruction-count
increase in both cases. Stack and payload load and store counts are equal.
On prefixed natural 4 KiB and 8 KiB the 16-byte-only matcher executes 122,376
and 253,782 fewer instructions, respectively. The instrumented runs verified
exact outputs and measured visits to the original machine blocks. Runtime
measurements use the uninstrumented workers. [Counts](matcher-counts.json) ·
[Machine blocks](matcher-blocks.json.gz)
