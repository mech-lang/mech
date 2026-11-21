# Broader matcher investigation: source attribution and existing bulk work

**The selected build remains 63,804 B / 81 Roc / 63 libdeflate wins.** This is diagnostic work toward improvements across input classes. It does not establish a 64th win, replace the qualified Huffman alternative, or change the normal compiler source. [Snapshot and archive](summary.json) · [Portable verification](verification.json).

The preceding [collector split](../collector-loop-specialization-study/README.md) repairs native admission but delivers only a small large-random gain and loses overlap. Further work must target substantial repeated work and memory traffic, rather than only the closest thresholds.

## What the history and fresh checks rule out

The existing immutable-current-read study already covers current-index copy-phi proofs, shared defaulted byte reads and pinned versus rematerialized placement. A fresh exact-artifact IR dump confirms that the qualified Huffman build shares a canonical byte read before permitted backend rematerialization. These are not new optimization proposals.

The actual rebasing kernel is **word collection 10, header 514**, immediately after the collector's cold sliding boundary. It uses the existing generic saturation selector and four-vector SIMD loop. On random 1 MiB it is called **31 times**, processes **8,126,464 halfwords**, and executes **7,365,104 instructions**, including **1,015,808 `uqsub.8h` operations**. It has one call on random 64 KiB and none on the smaller declared inputs. The earlier phase-362 probe is not a rebasing measurement.

Pinned libdeflate also uses 16-bit positions, window rebasing and a four-vector ARM implementation. Mech's lifted map is already vectorized. Fresh sink allocation, ownership and copying outside this kernel remain separate questions. Its instruction count is not an elapsed-time budget; do not infer a whole-call gain from removing it.

Native matrix constants are already emitted as immutable object data by `compact/constants.rs`. The per-call allocation count is not evidence that those lookup literals are allocated on the heap. An allocation-origin census would be needed to attribute the remaining requests.

## Source-level diagnostic

The separate [compiler patch](diagnostic-source.patch) records each canonical numeric action, annotates diagnostic IR source locations, and exports machine-source ranges alongside the existing selected-phase VCode dump. It applies to source at `153432b2fc1d7b08e05fc4bd5ab378548a5876c0`. The archive pins the patch, actual compiler build, commands, source, generated objects, raw counters and all checks. No LLVM dependency is added.

The patch is deliberately retained as a diagnostic patch; the normal repository compiler files are restored. With inspection disabled, the patched compiler reproduces **all three artifacts of both the selected default and qualified Huffman source exactly**.

Inspection changes optimization and code layout. The old shipping block profile correctly rejects that annotated body, so the diagnostic image is generated without that profile. Its canonical source and bytecode remain identical. Its matcher is **6,232 B**, with **752 source ranges covering 4,020 static bytes**; other bytes have no retained canonical attribution. The nine-case diagnostic records retain unknown attribution explicitly. Counts attached to a source location count generated instructions, not executions of the source statement.

**Do not attach the shipping kernel's instruction counts or elapsed times to these different bytes.** The annotated image is suitable for deriving semantic-work counters after validation; matched Mech/C tree/search/prefix work still needs to be derived. Existing C counters and the shipping 204,521,468-instruction matcher measurement retain their original scopes.

The annotated image passes 162 exact output/inflation/alias/resource/cleanup checks, all six checked allocation errors, 12 injected failures with exact recoveries, and normal/checked emission equality for all three artifacts. Actual measured stack is **4,704 B**, within the 5,280 B cap. No performance claim follows from that stack result.

To inspect after applying the patch and building `compile_profile`, generate the qualified Huffman source with its published recipe, remove its old `MECH_COMPACT_PHASE_BLOCK_LAYOUT`, and set:

```sh
MECH_INSPECT_PHASE_SOURCE_SITES=/tmp/mech-sources
MECH_INSPECT_PHASE_CLIF=/tmp/mech_compact_numeric_475_vector.clif
MECH_INSPECT_PHASE_VCODE=/tmp/mech_compact_numeric_475_vector.vcode
```

Use fresh output directories. The first setting writes canonical action inventories; the latter two select the phase. The VCode path also receives a `.source-ranges.json` sibling. Saved commands in the archive provide the complete recipe. `python3 verify.py` checks the saved evidence without rerunning clocks or requiring Apple hardware.

## Next experiment

The attribution identifies repeated adaptive-block predicates in the per-position collector. The original conjunction computes several pure comparisons and wrapping differences even before the observation count reaches 512. A separately pinned source alternative expresses the same predicate with nested conditionals, preserving every threshold and block decision. Its work reduction, native gates, runtime score, size and complete build time must be measured before selection. This targets repeated work across input classes; the qualified sources remain unchanged.
