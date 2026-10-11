# regex-redux: source-visible Mech and general byte regex operations

`regexredux.mec` is the complete 24 LOC program. The `Count` function is defined
in Mech; it combines a single general regex count with source-defined formatting.
The nine counts, cleanup expression, five ordered replacements and output are
all present in the source. There is no host regex-redux executor.

The program follows the [official workload](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/regexredux.html):
read the entire FASTA input, record its byte length, remove headers and newlines
with a regex, count each of nine patterns separately, and perform the five
substitutions in order. The full input is FASTA N=5,000,000 (50,833,411 bytes).
Input and complete reference-output hashes are in `input-pins.json`.

The new general library calls are `regex/count-bytes(subject, pattern)` and
`regex/replace-bytes(subject, pattern, replacement)`. They accept byte subjects,
including non-UTF8 input. Regex syntax is the Rust regex library's byte syntax
with Unicode disabled. Counts use nonoverlapping matches. Replacement text is
literal, including dollar signs; it is not a capture-expansion template. Invalid
patterns produce errors. A context caches compiled patterns without recognizing
benchmark strings, input lengths, program names or DNA alphabets.

The existing dependency scheduler now prioritizes ready regex counts alongside
other independent count operations. With `MECH_SEQUENCE_ASYNC=1`, sufficiently
large independent counts run in the existing Rayon pool while the source's
dependent replacement chain proceeds. A scalar consumer waits for its count;
entry completion also waits for unconsumed requested counts. IO order is
preserved. This is the same 24 LOC source under two general execution policies.
Bulk `io/read()` also now reads complete bytes directly rather than repeatedly
reading individual lines. No fixed-overlap chunking of arbitrary regexes is used.

The optional `MECH_SEQUENCE_BUFFER_REUSE=1` policy retains at most two dead byte
allocations for subsequent outputs. Reference-counted ownership prevents
reclaiming a buffer while aliases or independent readers still use it. Regex
replacement can fill this reusable storage directly, adopting Rust's general
buffer reuse technique without hiding the source's replacement pipeline.

The capabilities are admitted through the experimental Cranelift AOT frontend
backported onto the v0.4 integration. This is not a claim that every interpreter
or distributed execution path already supports these operations.

`manifest.json` pins complete upstream programs. `run_remaining.py` builds all
listed native-cohort versions and the smallest and published-fastest compact
runtime versions (including ties). The result report records local build errors
and full-output exclusions. Java rows explicitly use OpenJDK JIT rather than
the upstream site's GraalVM native-image configuration. Lua and LuaJIT use
separately compiled, pinned copies of the general lrexlib PCRE2 binding.

Measurements use sequential fresh processes on Apple M3, AC power, Low Power
Mode off, full output to a regular file, and shuffled orders. Build and warmup
are outside the clock; startup and JIT compilation are inside it. Every accepted
sample must match the complete reference output. Reported size is nonblank,
noncomment physical LOC of the complete language source, excluding generic
libraries and compiler machinery.

## Reproduction

From the worktree root, with the pinned peer dependencies available:

```sh
PYTHONPATH=/tmp/mech-clbg-compact-libs python3 benchmarks/clbg/run_remaining.py regexredux --prepare-only
PYTHONPATH=/tmp/mech-clbg-compact-libs python3 benchmarks/clbg/run_remaining.py regexredux --reuse-builds --samples 3
```

The harness refuses to overwrite an existing measured report. The adjacent
compiler archive retains changed and untracked compiler/runtime inputs relative
to its recorded v0.4 base. Archives are reproducibility artifacts, not size scores.

`remaining_conformance.py` checks unrelated byte regexes, literal replacements,
non-UTF8 data, empty matches, and both scheduling policies, as well as this full
workload. `remaining_regressions.py` rebuilds prior benchmark endpoints and checks
their native instructions and complete outputs without replacing old artifacts.
