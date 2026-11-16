# Current-element write proofs (R227)

Retain this separate compiler experiment over R226, on the original pinned source.
All 25 unchanged gates pass; geometric balanced block means improve **1.085%**.
The compiler removes a second load/comparison only when a nonzero modular
increment proves that the exact current array element must change.

R223 indexing stays complete. R224 already confirms the matched comparison;
R225 cache-word bindings and R226 phase proof reuse are retained separately.
R225 is not combined here. The remaining private-write, packet-storage and wider
match experiments are not marked complete.

| Input | Before / Roc | Candidate / Roc | Block-mean change |
|---|---:|---:|---:|
| Random, 32769 | 3.059630× | 3.000583× | -1.930% |
| Distribution change, 16384 | 2.975706× | 2.941638× | -1.145% |
| Natural text, 32769 | 2.361706× | 2.346531× | -0.643% |
| Periodic, 33027 | 2.151660× | 2.134142× | -0.814% |
| Repeated, 32769 | 2.018947× | 2.000092× | -0.934% |
| Eight literals | 0.529005× | 0.519218× | -1.317% |

Ratios are per-call medians; changes use balanced block means. Empty, random1024
and natural1024 have adverse block means (+0.616%, +0.0004%, +0.488%). Five cycle
medians rise. All 25 distributions, including stored-only clock granularity,
remain in the archive. No universal speedup or best-of-variants choice is claimed.

## Design

The program still chooses every compression path, cost, tie and block decision.
This is ordinary array-update algebra, selected without application names,
field layouts, source-site IDs or benchmark constants. On a successful checked
write, `(old + constant) mod 2^width` differs from that same element when the
modular constant is nonzero. Zero modulo the element width retains comparison.

The shared current-traversal engine records exact scalar value recipes and a
conservative content version. Any write invalidates earlier content identities,
including potentially aliased views. Unknown effects/growth stop the supported
prefix; ambiguous definitions/joins, different views/indices and partial masks
decline. Each required DAG node is visited once, with no new depth allowance.
Recipes supply no ownership, safety or completion authority.

A later successful one-based write at wrapping(at+1) also establishes that the
earlier defaulting read's exact at<length branch was selected. At=MAX still
wraps to zero and fails the original write check. Incoming arithmetic, narrowing
and assertions still execute first. Write bounds, current canonical matrix
translation, shared-buffer materialize-on-change, charges and failure order
remain. The broader private equal-write simplification is not included.

## Measurements and qualification

The actual Object selects 51 static writes across 45 numeric routines; dynamic
executed write counts are not claimed. Other 89 numeric routines and two maps
retain identical code. The main histogram body shrinks 812→704 B, static
instructions including padding 203→176, branches 16→13, loads 53→46 and stack
208→192 B. Some saved loads reflect reduced register pressure. All affected
routines lose 1116 B in total. Root text 6781056 B and stack 86736 B stay equal.

Random32 removes 1295004 instructions (64.2414m→62.9464m, −2.016%). Natural32
removes 637608 (153.1227m→152.4851m, −0.416%). Instructions fall on 22 cases/tie
on three; cycles fall on 20/rise on five. All 25 before/after allocation,
requested-byte, reallocation, incremental usable-peak and payload/copy/zero/peak
fields match. No allocation or streaming improvement is claimed.

162222 primary calls, 3900 separate counter calls and 50 immediate stream smokes
cover 20 originals and five held-outs through 131073 B. Fixed level12/runtime
inputs, actual held input/output aliases, equal 200ms warmup, 24 balanced order
permutations, the Rust clock and validation placement remain matched. Complete
compression, scratch construction, transfer and cleanup stay timed. All exact
bytes, independent inflate, prefixes/capacities and aliases agree.

684 maintained tests pass under the six-feature profile and warnings-as-errors,
with the same seven inherited exclusions. Its 26 focused traversal tests include
five new proof tests. 2304 compiled controls match parent/candidate and
original/decoded status, origin, value, geometry and selection/fallback, plus the
resident/independent oracle. All four widths, wrap, invalid indices, selected
paths, fresh visits, intervening writes, matrices, aliases and recovery are
covered; no new resident default denials occur. Shared-alias cases fall back then
enter the native phase during the same invocation.

864 ordinary native controls also pass, retaining finite clone denial, prior
publication, aliases, cleanup and retry, with original/decoded work and errors
identical. Final managed bytes/nodes remain zero. This is a bounded check of the
shared emitter; the standalone whole-compressor comparison still omits the extra
ordinary resource/admission/accounting/publication/recovery contract. Maintained
normal IR215 source is unchanged; the complete experimental compiler is archived.

Fresh wall/current-PC profiles use 24173/98517 correct calls. Large cases have
93.08–96.85% generated exclusive wall weight, including useful computation.
Samples are not instruction shares or removable-time predictions.

Prototype scalar-vocabulary/dead-variant build failures were corrected without
warning exceptions. A new typed-input lifetime bug reproduced on the clean
parent test client; retaining its backing fixes the held borrowed output. The
first tiny profiler export returned an internal error and a following shell
command explicitly hit ENOSPC. After verified completed builds were freed, export
and a fresh tiny capture succeed; five large captures remain byte-identical.
No primary/counter observation changed. The initial interrupted observer session
has no final-cleanup claim; final sessions do. No unresolved introduced compiler
or compressor failure remains. Exact commands and failures are in the archive.

## Revisions and evidence

Base/experimental parent `73124a08e0c01b037a0e4d5a4bf60250b0b70a80`; maintained normal `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5` (IR/work/ABI/MBC 215/107/39/14). Twelve archived Rust files change. Compiler manifest `eb1840bef1a168c102a8cce83d151070bdd39ee5de1b888e9c0f6ae4c32b03a6`; Object `e665bf9ddf71eb3a3b4c41ea1693b1eb51b119701c68c570d7e65aa25a9bc9b4`. Original source/bytecode hashes remain `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` / `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.

[r227-current-element-writes-6ca37d89847d.tar.gz](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r227-current-element-writes-6ca37d89847d.tar.gz) — 4629237 B, 1771 hashed files, SHA-256 `6ca37d89847d9e53cae65e334d314e2cfd1926f9b58a525053e9fe435213808d`. Local and downloaded hashes plus byte-identical numerical replay verify. Complete candidate Rust/Mech source, minimal patch, clients, build specifications, structured cohorts and failure receipts are included. Native binaries, bytecode, full executable metadata and raw traces are excluded.

Pinned algorithm, allocation sites, real scratch capacities, tables, search
limits, allocator and timing boundaries stay unchanged. No LLVM change, new
branch/PR, fresh ordinary-entry timing, multi-MB run or green hosted-CI claim.

Next: separately measure the broader proved-private write simplification, then
packet storage reuse and wider matching. Combine R225 only with an explicit
combined all25 measurement. The brief and parity remain open.
