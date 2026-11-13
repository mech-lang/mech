# Parser execution work amplification

The level-12 compressor takes about **3.5 seconds to parse 160,857 bytes** after
the [2.94× parser improvement](native-parser-compile-profile.md). A new diagnostic
probe counts the machinery behind that remaining time:

| Operation | Calls or frame pops |
| --- | ---: |
| Base continuation frames | 92,975,818 |
| Operator continuation frames | 36,389,308 |
| Base plus operator frames | **129,365,126** |
| Parser checkpoints | **42,639,485** |
| Parser rewinds | **35,397,697** |
| Mutable journal-page accesses | 13,289,669 |
| Literal-cache replay probes | 19,941,589 |

Base and operator frame pops are disjoint: together they exceed **804 per source
byte**. The other counters occur inside these operations and must not be added to
them. Other continuation families were not counted. Public
`ParseStats.parser_steps` remains 1,967,640; that counter measures a different
operation and does not describe the physical continuation work.

The probe copies the current canonical document module and adds relaxed atomic
counters at six call sites. It preserves the control's structural hash,
434,637 emitted events, 92,316 final nodes and zero recovery bytes. Its elapsed
time is unqualified: instrumentation was enabled and a compiler bootstrap ran
concurrently. The previously published clean parser timings remain authoritative.

These counts support targeting the parser's execution architecture. The next
experiment should reduce failed alternatives before constructing transactional
state, and fuse common terminal paths. Candidate dispatch must derive from the
existing canonical specification. This changes how the same grammar executes;
it does not require changing productions or adding syntax. The counts alone do
not establish how much work can be removed or predict a speedup. Complete trees,
diagnostics, Unicode boundaries, resource limits and resumable behavior remain
the comparison controls.

Parsing is about 7% of the last qualified 51.113-second full compilation.
Eliminating it entirely would still leave about 47.6 seconds elsewhere, so the
overall sub-10-second compilation goal also requires frontend and native work.

Two separate schema experiments were screened and rejected:

| Frontend screen | Frontend time | Maximum RSS | Instructions |
| --- | ---: | ---: | ---: |
| Frozen control | 8.685 s | 7,474,184,192 B | 195,595,410,458 |
| Project component environments | 8.805 s | 7,487,520,768 B | 199,047,472,661 |
| Reuse environment validation and intern retained trees | 8.794 s | 7,386,316,800 B | 195,867,227,013 |

These are individual screening observations, not balanced regression estimates.
Neither candidate establishes a frontend latency or instruction benefit. The
second candidate's small RSS reduction does not justify its additional API and
retention machinery. Both patches were reverted. Source and bytecode are exact
matches; no full native qualification followed rejection. The shared-context
candidate passes 192 Core tests and two differential retention tests. Its host
compiler build takes 452.429 seconds, separately excluded from program timings.

The [compact results](../../benchmarks/mech-deflate/native-profile/parser-work-schema-screens-apple-m3-20261113.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/parser-work-schema-screens-apple-m3-20261113.archive.json)
pin the [public raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-parser-work-schema-screens-apple-m3-20261113.tar.gz):
238 files, 2,220,065 compressed bytes, SHA-256
`d2d7fdc8e597870944e0036e2026d63e3c6e814e536762df486648dffec515f5`.
The packet retains source copies, exact counter sites, both rejected patches,
compiler build commands and hashes, tests and every screening observation.
Executables and dependency rlibs remain local. Independent public retrieval,
complete manifest verification and `python3 replay.py` reproduce the compact
results byte-for-byte without a compiler.
