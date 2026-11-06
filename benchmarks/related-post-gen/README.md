# Related-post native compiler review

Current integration: [related-post compiler plus latest #973](integration/README.md), **IR227 / work115 / ABI41 / image9**. The compiler builds; four variants retain53 native numeric functions and pass124 output fixtures,839 GPU tests retain two inherited failures,289 Engine checks and runtime/application checks pass. All128 scalar controls remain exact; the compression output is exact but its changed route/accounting remains under audit. No integrated timing or adoption claim. Historical timing tables below retain their recorded compiler revisions.


This branch contains the accepted compiler optimization series for
[zupat/related_post_gen](https://github.com/zupat/related_post_gen), starting from
the DEFLATE compiler. The review targets the branch behind
[PR #973](https://github.com/mech-lang/mech/pull/973):
`codex/level12-compression-consolidated`.

The measured compiler is **5925a2b3a4fd4dfcca7ada541e37a69d514171f8**
(IR223, work114, ABI40, image8). The selected overall speed checkpoint remains
**c3a6925108ad3523f4e29022b66f1c17b92c9878** (IR208). Neither meets the goal of
matching the fastest eligible Rust implementation in both processing and
complete fresh-process time.

## Current measurements

These are the retained 15-round confirmation medians on the same Apple M3 and
the same 5,000-post input. The continuous post-JSON timer includes input
preparation/admission, allocations, worker creation/run/join, and result
materialization. Complete time covers launch through exit, including JSON IO.

| Implementation | Post-JSON | Complete process |
| --- | ---: | ---: |
| IR222 parent, original source, 4 workers | 3.209042 ms | 36.319916 ms |
| IR223 candidate, original source, 4 workers | **3.140833 ms** | **36.801792 ms** |
| Fastest matched Rust: checked U8 IDs / U32 ranks | **1.651875 ms** | 23.899291 ms |
| Fastest complete-process Rust: streaming histogram | 1.657375 ms | **22.249042 ms** |

The candidate takes **1.9014x** the fastest matched Rust processing time and
**1.6541x** the fastest complete-process Rust time. Those references differ;
they are identified explicitly in the reports. The calculation-only Mech
median is 2.977084 ms, but it excludes preparation/admission and is not the
matched headline comparison. Separate interval medians must not be added or
subtracted as an exact phase decomposition.

IR223 improves post-JSON time by 2.13% in confirmation and 2.63% in the initial
cohort. Complete time regresses against IR222: only 3/15 paired confirmation
wins, with a 1.113 ms median paired regression. It is therefore not selected as
the overall speed winner. There are no discarded latency samples. Resource
and diagnostic-profile runs are separate from the latency cohorts.

| Post-JSON resource, separate 7-round cohort | IR222 | IR223 | Fastest matched Rust |
| --- | ---: | ---: | ---: |
| Allocation requests | 4,848 | **3,858** | **58** |
| Requested allocation bytes | 8,384,366 | **8,043,926** | 6,825,713 |
| Retired instructions, all participating threads | 65,390,240 | **64,443,191** | **19,234,550** |

The candidate still executes **3.35x** the instructions of the matched Rust
control. Allocation requests are 66.52x Rust, but requested bytes are only
1.18x. This points to repeated small control/metadata work as well as generated
query inefficiency; it does not establish a CPU percentage for either.

## Why the full boundary is still above 3 ms

All 53 numeric functions, including the numeric workers, emit native machine
code. The latest storage change leaves all 212 numeric functions across the
four tested variants, and their four complete native text sections, byte
identical to IR222. It replaces general allocation planning for fixed native
frame storage; it does not improve the main query's generated instructions.

The fresh allocation profile attributes all 3,858 requests. Worker setup falls
from 1,215 requests to 424, while root canonical library-call handling still
accounts for 2,303 requests. General type/schema copying, call planning,
snapshot ownership, and remaining worker setup are concrete targets.

The native query remains the leading sampled application-code hotspot. It has
a 688-byte stack frame and 7,299 static instructions, with repeated checks,
stack accesses, register copies, and ranking/selection code. Static instruction
counts and sampled positions are not dynamic instruction counts or CPU-time
shares; the profiles do not justify assigning all remaining time to the host.

Compiler follow-ups to review include proving canonical Boolean values to
remove redundant normalization, propagating immutable producer-derived array
element ranges to remove repeated bounds/overflow checks, and proving funded
inner-loop bounds to avoid repeated funding tests while retaining exact
debits/errors. These are proposals, not implemented gains. Source-level
subset/histogram and compact-ID variants are included, with corresponding
checked Rust controls; no further SIMD or parallel speedup is assumed.

## Review scope and qualification

The full series lowers numeric loops, collections, popcount and workers;
specializes native frame/state layouts; simplifies proven control flow and
unsigned checks; preserves exact work accounting; and reduces metadata and
allocation setup. Review the compiler commits in order. The latest fixed-arena
change is separately available in
[latest-fixed-arena-change.patch](review/latest-fixed-arena-change.patch).

The latest change introduces a closed typed `FixedHostArenaPlan` with one exact
backing allocation and one private owner. It preserves the full existing
shared-budget admission envelope, initialized element destruction, fresh
opaque identities, generation/type checks, and zero-length behavior. It does
not replace the efficient scalar native entry, weaken finite accounting, or
move processing work outside the timer.

Accepted qualification records 391 Core tests, 289 relevant Engine tests,
689 GPU tests, 434 loader/fixture checks, 16 forged-frontier rejections,
21 rejected old Objects, runtime-only and Managed application checks, and
1,437 exact cohort outputs. The 133 scalar/DEFLATE controls preserve every
nonallocation, nontiming field; allocation requests/bytes never regress. Two
pre-existing GPU test failures remain explicit:

- `batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable`
- `native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`

These results qualify the pinned measured compiler, not a new merge of the
latest changes on #973. Hosted CI and integration with concurrent #973 changes
are not claimed by this publication.

The publication merge preview against #973 head `dcc09ee9e75...` identifies
36 conflicting compiler files. Their exact names and compared revisions are
recorded in [the integration preview](review/pr973-integration-preview.json).
The PR is a draft for review of this measured checkpoint; resolving and
qualifying the combined compiler remains integration work.

## Evidence and reproduction

- [Candidate report](results/compiler-native-fixed-frame-arenas-candidate-20261106.json)
- [All confirmation samples](results/comparison-native-fixed-frame-arenas-confirmation-20261106-5000-m3.json)
- [Resource samples](results/comparison-native-fixed-frame-arenas-resources-20261106-5000-m3.json)
- [Allocation stacks summary](results/profile-native-fixed-frame-arenas-post-json-allocations-20261106.json)
- [Native worker instruction profile](results/profile-native-fixed-frame-arenas-worker-instructions-20261106.json)
- [Complete-process phase diagnostic](results/profile-native-fixed-frame-arenas-complete-process-phases-20261106.json)
- [File hashes and compiler-source hashes](review/manifest.json)
- [Recorded build/qualification recipes](review/recorded-recipes)
- [Test and compatibility receipts](review/qualification)

Run `python3 benchmarks/related-post-gen/verify-review.py` from a checkout to
verify the published evidence and unchanged measured compiler files. It does
not execute the benchmark or re-run qualification.

`sources/` contains the four actual original/compact, four/eight-worker Mech
sources; `hosts/` contains their exact adapters. `rust-controls/` contains the
standard and fastest checked Rust sources with their lockfiles. The shared
input is `fixtures/posts-5000.json`, SHA256
`500530b81cb81c5b1a5e48aa30de6a89ba0a350460ad826c4c7bb023e6e46cb8`.
Rust controls were built using nightly-2026-03-03, with the flags in each
manifest and recorded receipt. The checked U8/U32 control includes overflow
checks. Mech retains its checked source semantics and unwind/error behavior.

The recorded native recipes retain their original absolute paths and pinned
library hashes. They require the original retained dependency closure or a
separately reconstructed matching build; they are not advertised as a portable
one-command build. Native binaries/Objects, dependency libraries, and raw
sampler traces are not uploaded in this packet. Their separately retained
local archive is identified by `review/local-archive-index.json`; this is an
archive inventory, not a remote download link.

The original snapshot `2fdacb1b2...` omitted 36 ignored lockfiles/test fixtures
from DEFLATE commit `fa65038fce...`. Publication restores those exact files and
connects that real ancestor to the snapshot history. All existing measured
compiler files stay unchanged. This establishes a meaningful comparison with
the branch behind #973 while retaining the original measured commit hashes.
