# Move disposable private-frame owners after validation

Retain this compiler/runtime improvement. Complete eight-byte level-12
compression improves **1.85418%**, from **4.371896 to 4.290833 ms**, or
**15.127668× → 14.847173× Roc** using one shared fresh 0.289 ms reference.
All four independently prepared pairs and **62/64 descriptive blocks** improve.
The fixed historical 0.300 ms ruler moves **14.572987× → 14.302777×**.
This is a matched improvement; earlier raw times/references belong to other
cohorts. Parity remains outstanding at 1×. No larger input is timed.

| Metric per compression | Before | After | Change |
| --- | ---: | ---: | ---: |
| Hardware instructions | 49,519,459.5 | 48,601,797 | **−1.85314%** |
| CPU cycles | 13,899,757 | 13,567,326.5 | **−2.39163%** |
| Allocation requests | 13,994 | 13,994 | unchanged |
| Requested allocation bytes | 98,586,843 | 98,586,843 | unchanged |
| Additional peak tracked heap | 20,575,432 bytes | 20,575,432 bytes | unchanged |
| Prepared tracked heap | 142,706,080 bytes | 142,706,080 bytes | unchanged |
| Kernel physical footprint after compression | 228,706,048 bytes | 228,706,048 bytes | unchanged |
| Resident memory after compression | 319,684,608 bytes | 319,733,760 bytes | +0.01538% |

Time, hardware counters, allocator probes and profiles are separate cohorts.
Tracked heap, RSS and kernel footprint are different views and must not be added.
Whole-task memory includes retained setup/compiler/catalog state. See the
[updated executable comparison](../executable-comparison/REPORT.md) for reference
ratios, exact memory scopes, unchanged backing requests and code inventory.

## Exact revisions and bounded design

Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
Published start/baseline production source:
`a0ad02497c96ea876799d1f844554cdc23d6c669`.
Implementation: `f85d1d670aba40b145fd5b9586def4aab061b507`.
Optimized host-build source: `be651d31a82008daf24b4ad2d4dcbc553264a10a`.
Separate CI guard adjustment: `7ae13bf4801102120812ff2a0ae060f60aeaa474`.
The final review head is recorded on PR #973. `analysis-inputs` archives the
complete source/test/workflow patch relative to the published start and a full
pre-closure commit inventory. IR/work/native ABI/bytecode remain **131/95/37/14**.

The prior real-compressor profile placed 25.57% of ancestry under frame actions.
The selected Pack path copies current arena handles into temporary call inputs,
validates the selected call and complete pack, clones those handles into private
frame owners, then immediately clears the temporaries. `Value` already shares
shape, root and optional schema using Arc. This duplicate copy costs reference
counting; it does not allocate shape metadata or copy the backing buffer.

`BufferFrames::pack` now accepts its disposable input slice mutably and takes
each managed temporary only after the original work charge and complete existing
validation pass. `Invocation::buffer_frame_action` supplies that owned temporary
slice. Its ordinary clearing/error cleanup remains. Original arena roots,
external aliases, scalar words, composite child copies and all finite charges
remain unchanged. There is no new owner, executor, cached permit, source-site
proof, exclusivity assertion or special benchmark path. Source algorithms and
allocation sites are untouched.

The new focused test is committed first in `d82a9ab068e3facc31d693cb528a6d60155e92aa`,
with its layout borrow corrected in `4094938a0`. On the unchanged production path
it fails exactly at “successful pack must consume disposable call inputs”. With
the transfer it passes for original and decoded artifacts. It uses the actual
compiled layout: current owners move on success; a later invalid Boolean or
one-short pack work limit retains pending owners and the prior frame; retry
installs a fresh owner, retained aliases keep their content, exact pack charge
is unchanged, and final managed bytes/nodes are zero.

## Binary evidence and remaining profile

The exact measured linked workers show **469 → 448 instruction sites** in the
Pack helper and **three → zero atomic increment sites**. The enclosing frame
action retains 1332 instruction sites; its cleanup now observes taken/empty
slots on success. Static sites include errors and unreached paths and are not
dynamic counts. The hardware cohort separately establishes the dynamic reduction.
`host-code` archives commands, binary hashes, selected symbols, full disassembly
and mnemonic counts.

The generated compressor Object is byte-identical, SHA-256
`3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`.
Its generated text and entry stack remain **13,885,696 / 108,400 bytes**.
Only host ownership transport changes.

Fresh isolated serial profiles run the actual primary workers, 695 calls each,
after all counter workers finish. Current frame-action ancestry is 322/1237
weights (**26.03%**); other helpers, ordinary visits and library calls still
matter. These noisy proportions include surrounding harness work and are not
precise timer percentages. They do not contradict the smaller directly measured
instruction/time gain. The next bounded target should follow these remaining
frame/admission costs and repeat the same scorecard.

## Configuration and unchanged compressor

Both primary variants and their respective CPU/memory diagnostics use the
**same exact coherent optimized libraries and same-build Object/images**.
The transitive Compute artifact is explicitly frozen alongside all eight direct
libraries. Fresh Objects/images are emitted for both versions. Every non-GPU
library hash is unchanged; only GPU rebuilds, in **22.8644 seconds**. No Cargo
output is mutated and the math libraries are not rebuilt.

The existing direct rustc producer uses opt 3, warnings as errors, nightly
2026-03-03 (`ec818fda3`), LLVM 22.1.0, aarch64, with the same recorded
`aot,canonical-native,canonical-native-program,canonical-native-program-runtime,
canonical-native-runtime,jit` features. Full arguments, environment, paths and
hashes are in `release-build`, `before-library-pins` and `linked-build-*`.
The new test roots use the same dependency graph at opt 1 and `-D warnings`.
A fresh minimal CI-profile Cargo rebuild is not claimed.

The unchanged complete source and bytecode SHA-256 remain
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` and
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
The pinned mimalloc 2.1.7 Object remains
`38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
The reference executable remains
`3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`, with its
recorded compiler revision `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d` and package
`671077457ff7ef3a3d44caf715a3fc595addf379`. Hardware remains the M1 Mac mini, 8 GiB,
macOS 15.6.1 (24G90).

Compression uses the real **262144 U16 tree, 1500513 U32 cache and 305257 U64 path
capacity**, depth 300, nice 258, original near-EOF limits, tables, allocation
sites and timing boundaries. Every measured Mech call checks exact ten raw bytes,
independent inflate, input contents/aliases, output capacity 13 with clear/reuse,
work **60,260,219,686** and zero final managed bytes/nodes. Neither implementation
streams input here. Total copied bytes and branch/cache misses remain unmeasured.

## Timing and qualification receipts

Primary has four prepared pairs × 16 alternating blocks × 31 calls/version:
**3968 primary calls**, 1984/version; warm-up/final make 3984 total. The shared
reference has 20,000 calls. Hardware medians have 32 steady calls/implementation;
memory has eight Mech and 32 reference calls. No hardware observation incurs a
page-in. Boundary-read overhead remains included, not subtracted. Each reference,
pair, summary, counter and profile cohort is committed immediately. No build,
test, audit, diagnostic or profile overlaps primary; CPU/memory/profile cohorts
are serialized. Live workers are never replaced after an observation timeout.

| Group/driver | Observed result |
| --- | --- |
| `linked-build.py`, `run-primary.py` | **3968 primary + 20,000 shared reference calls**, exact checks pass |
| `counters.py`, `profile-isolated.py` | Separate hardware/memory cohorts and both **695-call isolated profiles pass** |
| `unit-checks.py before` | One intended regression failure, exact ownership assertion |
| `unit-checks.py after` | **316 runtime + 31 work/memory/value tests pass** |
| `qualify.py` | **234 unchanged components + 8 connected cases + 1 actual path-selection test pass** |
| `linked-acceptance.py` | **12 AOT executions pass**, original/decoded, success/error/retry |
| `control-build.py`, `controls.py` | **12 scalar controls and 8 frame allocation cases pass** |
| `ci.py` | **18 listed/executed; 132 configured Python workflow/inventory tests pass** |
| `architecture.py` | **10 existing routing/catalog/layout/factory/warning/unsafe/fmt/diff checks pass** |
| `inspect-host.py` | Exact worker hashes and reached helper disassembly archived |

These are **590 distinct Rust tests** across the principal selections. New/CI
reruns are not counted twice; AOT executions and performance controls are separate.
The focused actual-layout transfer test is included in runtime. Existing finite
boundaries, current recursive arguments, original/decoded and restored-image
coverage, error precedence, aliases, prior publication, cleanup and recovery
all pass. AOT success work remains 21,787,354 / 21,862,018 / 21,744,466. Existing
component expected values, allowances and Huffman work 91,452,918 stay unchanged.
The original scalar 1.05 ceiling is preserved; observed ratios range
0.99162–1.02524, with no allocation increase and zero scalar/frame loop allocation
slopes. The CI guard moves 17→18 for the new existing-selection case, separately.

All listed drivers are preserved with their full actual commands, profiles,
stdout/stderr and counts in the corresponding tar/manifest groups. `analyze.py`
checks counter/primary library equality, Object bindings, exact output/work and
host-disassembly binary identity. `reconstruct.py EMPTY_DIRECTORY` independently
verifies **38 archives, 376 file occurrences and 313 unique paths**, regenerating
both JSON/CSV scorecards byte-for-byte without executing the compiler/benchmark.

## Failed adapters and skipped coverage

The first test adapter passes an owned derived layout where a borrowed layout
is required; rustc rejects it. The corrected adapter builds and establishes the
intended pre-fix failure before the implementation change. The first host-symbol
filter incorrectly excludes lowercase local-text symbols; it selects nothing,
then the corrected exact root-function filter disassembles successfully. These
are introduced test/analysis adapter failures, retained in `initial-test-adapter`
and `host-code-adapter`, not hidden product failures. The early claim that
cloning handles allocates shape metadata was corrected by inspecting the current
Arc-backed Value; unchanged allocation measurements confirm the actual scope.

The only maintained runtime exclusion remains the inherited
`complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`, which imports
`stats` unavailable in this reduced catalog. Its clean-parent reproduction is
retained in the preceding checkpoint. No new broad Core/Engine/all-feature suite,
minimal Cargo CI-profile rebuild or larger-input timing is claimed. No unresolved
introduced product failure is known. Full hosted CI is cancelled by user direction,
not reported green; PR #973 remains draft and new review is requested.
