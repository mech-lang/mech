# Fold grapheme continuations and replay longer statements

A sealed raw grapheme can scan and emit its result in the same poll. The parser uses the existing scanner and charges the existing result frame. A short allowance retains the suspended scan or result at the original boundary.

A completed one-shot statement can record up to 32,768 source bytes, 96,000 events and 262,144 identity ordinals. The existing sealed context, resource and diagnostic checks still govern admission. The replay creates fresh identities with the original logical work.

Folding the grapheme frames saved **14.3 ms** in eight standalone parser pairs. A further eight pairs saved **53.7 ms** after allowing longer statement recipes. The tree, indexes and parser work receipt match the reference. **538 syntax tests pass**.

[Grapheme measurements](folded-graphemes/result.json) · [Statement measurements](combined/result.json) · [Syntax tests](syntax-tests.json)

## Matched fresh builds on Apple M1

All four arms use fresh processes from source through final executable link, with compilers and fixed deployment support prebuilt. Each arm has 8 measured trials.

| Build | Source through final link | Mech parsing | Encoder text |
| --- | ---: | ---: | ---: |
| Qualified Mech control | 2.262 s | 0.787 s | 66,192 B |
| Mech with parser changes | 2.195 s | 0.719 s | 66,192 B |
| Roc | 2.979 s | | 57,696 B |
| libdeflate | 0.479 s | | 37,080 B |

Paired Mech build time is **0.737× Roc** and **4.578× libdeflate**. The median paired saving is **63.2 ms** across **8/8 faster trials**.

The encoder object, metadata, canonical bytecode, stripped executable and linked worker match the qualified build byte for byte. The two runtime orders of that worker scored 81/81 against Roc and 62/81 against libdeflate.

[Matched measurements](deployment.json) · [All raw builds](build-comparison.json.gz) · [Worker identity](exact-comparison.json) · [Full runtime evidence](../encoder-size-gap/dense-scorer-direct-branches/README.md)

## Checked encoder text

The exact checked entry contains 49,280 B, including 13,076 B of stack-relative loads and stores and 12,400 B of branches and calls. Pruning all private phase results and remapping all 41 checked windows emitted 66,288 B.

[Checked entry census](checked-entry-census.json) · [Phase result pruning](phase-result-pruning.json)

## Replay

Apply the compiler patch to an isolated checkout of `cebfd544a69880e537b05be21235df956a611f0e`. Rebuild the frame-copy variant at compiler optimization level 3 with direct source, preselected root inlining and skipped refused general SSA. Run replay.py with the rebuild directory and a fresh output path.

[Compiler patch](compiler.patch.gz) · [Rebuild command](../encoder-size-gap/shared-continuation-returns/README.md#replay) · [Replay pin](candidate.json)
