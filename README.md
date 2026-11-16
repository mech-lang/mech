# Mech level 12 at 0.970× Roc

The exact Apple M1 diagnostic object emits **0.970× Roc** and **1.361× libdeflate** in machine-code text. Mech, Roc, and libdeflate emit 55,956, 57,696, and 41,112 bytes respectively.

| Full 81-case timing | Mech vs Roc | Mech vs libdeflate |
|---|---:|---:|
| First order | 81/81 | 62/81 |
| Reverse order | 81/81 | 62/81 |

Each order recorded 982,656 verified timed calls across the four benchmark workers. The measured Mech object also passed 162 linked exact-output checks, 616 matrix checks, and six forced allocation-failure checks. The runtime control won 81/81 against Roc and 60/81 and 61/81 against libdeflate in the two orders.

The archive pins the benchmark source, canonical bytecode, native object, and metadata. The timing report records every case and the measured worker hashes. The generated checked root assembly and the matcher's LLVM input are under `codegen`. The native object was assembled through a local diagnostic compilation and object composition path. Compiler integration is tracked in [Mech PR #992](https://github.com/mech-lang/mech/pull/992).

The target for the next round is 1.000× libdeflate with 81/81 Roc wins and at least 63/81 libdeflate wins.

Run `shasum -a 256 -c SHA256SUMS` from this directory to verify the archive.
