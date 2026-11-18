# Cached terminal failures

A standalone Apple M1 screen from `78b064c5bb70937db72db21ebd3ab2fd556816c3`
compares the published grouped-page parser with a fast path for failed exact
terminals already in the literal cache. Eight balanced orders use matching
Rust opt-level 3, mimalloc, source and parser settings. All trials match the
syntax tree digest, 718,353 emitted events and 3,967,302 parser steps.

| Standalone parser | Median parse | Median peak RSS |
| --- | ---: | ---: |
| Grouped-page control | 1.885779 s | 113,999,872 B |
| Cached terminal failure | 1.878394 s | 113,983,488 B |

The paired median ratio is **0.997245×**, with **6/8** faster pairs. The
0.28% reduction falls below the earlier standalone screen. The focused
candidate remains archived.
[Summary](report.json) · [Timing plan](timings/plan.json) · [Raw trials](timings/trials.json.gz)

## Work counts and correctness

The diagnostic probe observes **5,535,739** exact-terminal cache probes and
**4,263,296** hits. The one-shot allowance check admits all **3,874,036**
cached negative hits. Each admitted result has already established that
the terminal consumes no source or events. The candidate replaces its
checkpoint, transient rule push, and zero-width rewind restoration with
the same rule high-water mark, rewind count and work charge. Positive
matches, resource finalization and streaming follow their usual paths.
[Census counts](census/counts.json) · [Census build](census/build.json)

The implementation passes **536 syntax tests** and **60 differential
snapshots**. A focused test primes a real cached failure and compares
execution at the exact cache allowance and one unit below. It checks
contiguous and segmented input, rule origin, rewind counts, token and
node IDs, exact event tape, and remaining fuel.
[Tests](validation/receipt.json) · [Matched snapshots](validation/matched-snapshots.txt)

Start from the base revision above and decompress the source patch.
Apply the separate census patch on top to reproduce the work counts.
The control and candidate receipts record exact rustc and probe
argument arrays. Their absolute paths are the original workspace.
Use the same pinned source, linked dependencies and allocator as the
[preceding page-index probes](../batch-page-index/probes/).
[Source patch](patches/cached-negative.patch.gz)
· [Census patch](patches/census.patch.gz)
· [Build receipts](build.json)
· [Executed commands](as-run/)
