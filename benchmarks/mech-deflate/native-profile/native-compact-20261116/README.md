# Native compact compiler review handoff

This PR contains compiler and parser source, a pinned Mech program, native build
configuration, measured runtime source, and raw benchmark evidence. It is stacked
on #973 at `9087b1f07727a9bfcb7c09059f48edaca6dc2301`.

The accepted runtime floors are **81/81 against Roc and 61/81 against libdeflate**.
62 is an improvement target. The older raw reports retain the former 62 floor;
that wording is historical. Historical evidence has not been edited.

| Apple M1 evidence | Mech code | × Roc code | × libdeflate code | Runtime qualification |
| --- | ---: | ---: | ---: | --- |
| Retained native baseline | 72,520 B | 1.25693 | 1.95577 | 81/81 and 61/81, both full orders |
| Optional cold-tail sharing | 72,200 B | 1.25139 | 1.94714 | 162 exact encoder calls; 15-case screen only |

The code denominators are 57,696 B for Roc and **37,080 B for libdeflate**.
Older receipts used a different 41,112-byte C object; their C size ratios must not
be substituted for this matched encoder comparison.

The retained parser change measured **7.59864 → 7.21202 seconds** from the same
source through a runnable executable (5.09% faster). Matched Roc and libdeflate
source-to-executable medians were 2.98588 and 0.48001 seconds. Compilers, runtimes,
and adapters were prebuilt. The Mech compiler was Rust opt-level 1 without PGO.
The parser change passed 530 tests and preserved bytecode, object, complete
metadata, and executable byte for byte. These are M1 measurements, separate from
#973's M3/LLVM result.

## What to review

- Compiler changes under `hosts/gpu/src/native_artifact/program`: checked regions,
  narrower live packets, direct/register outputs, shared array growth, owner
  proofs, and cleanup passes. They remain opt-in diagnostic policies.
- Vendored native backend: spill forwarding, measured leaf layout, and optional
  sharing of identical cold machine-code tails. Scalar operations keep direct
  native instructions. No external LLVM executable is used by this recipe.
- `src/syntax/src/document/parser/journal.rs`: retain the current journal page
  directly while preserving shared snapshots.
- The allocation policy now lives in `compact_runtime/zero_pages.rs`; the exact
  runtime source used for the measurements is also retained in `recorded-runtime`.

Cold-tail sharing is **not promoted as a new 81/61 result**. Its two focused
orders preserve the screen's peer wins, but tiny inputs have adverse estimates.
Those observations and all raw paired blocks are included. The maintained native
tests pass 10,240 calls and check call/relocation/trap/unwind metadata; warm-only,
debug-location, disassembly, and embedded-literal controls stay unchanged.

## Rebuild and replay

On Apple ARM64 with Rust `nightly-2026-03-03`, from a checkout of this PR:

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/replay.py > /tmp/native-replay.json
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/native-baseline
cargo +nightly-2026-03-03 test --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --release --test native_compact_cold_tails -- --nocapture
```

The portable backend checks have a maintained owner in the Linux
`canonical-policy` CI job. It runs `isa::aarch64::cold_` with the ARM64 backend
and requires exactly **12 passed, zero failed, zero ignored**: eight epilogue
checks plus four cold-tail checks. They cover signed branch limits, independently
reachable interior labels, distinct origins, relocation/call/trap remapping,
recorded deferred traps, unsupported relocations, and hint reset. The same
command passed locally; the source hashes and log are in `evidence/integration`.

The last command must execute **two tests** on Apple ARM64. The cold-tail test is
also registered in the `mech-gpu` library's native test module. Other hosts do not
qualify execution of Apple ARM64 machine code.

`rebuild.py --variant cold-tails` enables the new optional pass. The default
regenerates the retained baseline. Compiler construction and source-to-object
timings are separate. No timing result from this command is a new runtime score.
An optional `--mimalloc-object PATH` links the recorded worker and runs 162 exact
calls using the included fixtures. Use the same allocator object as the peer
workers for a runtime comparison.

The integrated tree preserves #973's newer engine, core, SSA and worker changes.
**A clean build of that complete integrated tree is still pending.** The recorded
81/61 result was measured before the final integration of seven upstream cleanup
files and their engine API. The rebuild command checks canonical bytecode and
native-object hashes and fails if integration changes them. The first integrated
build exposed an omitted `compile_jobs.rs` update for direct runtime calls; that
worker handoff is now restored from the measured compiler source and the build
is being rerun. Do not attach the historical scores to a different output
without measuring it.

The compact entry remains diagnostic: exact ordinary admission/accounting and
publication recovery are not claimed. This does not change the normal language
semantics or introduce an interpreter fallback. Error status/origin, alias,
allocation and stack checks are published in `evidence/baseline`.

Raw receipts preserve original machine paths as provenance; the rebuild recipe
uses paths relative to this checkout. `source-and-evidence-manifest.json` records
the published source/evidence hashes and integration status.
