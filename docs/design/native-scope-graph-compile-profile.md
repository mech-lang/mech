# Scope-sized current-visit validation: compilation qualification

Complete fresh level-12 compilation takes **20.368–20.777 seconds** on the Apple
M3 Air. Eight matched runs show **3.44% lower whole-process time, 3.75% lower
compiler time, 15.83% lower preparation time and 2.37% fewer instructions**.
All source, MBC, Object and complete metadata bytes remain identical. Peak RSS
is an adverse 0.008% higher with overlapping ranges; no memory improvement is
claimed. The full ten-second compilation goal remains open.

The validator previously allocated incoming edges, degrees, reachability and
definition-row owners for every program block during each nested scope check.
The level-12 program has 41,039 source-plan blocks, while individual scopes need
only their reachable subset. One ordinal scratch table now serves a validation
call; each scope owns its own reachable graph and definition rows. Nested checks
cannot overwrite a parent's local edges. Discovery still invokes its checks in
the original DFS order, predecessor lists retain insertion order, and the proof
uses the same initial source order and edge-driven ready queue. Only immutable
collapsed topology is retained during each scope check; every definition proof
is fresh. No proof survives a validation call.

The original conservative definition-table size calculation and admission limit
remain unchanged. The existing definition-transfer body, semantic reads/writes,
checked errors, grammar, language, scalar instruction emission, factory selection,
runtime accounting and generated-program optimization settings retain their
existing behavior. No execution machinery is retired.

Source `7feb94669905d3413d7a79fb9f8b89b851027226`, parent `c58562fdc38350b6e3b49fce3ba49ca220e529de`.
The frozen control comes from `610f3692c0ad0b894d6537f206a79a4de8833ede`; its production
source equals the documentation-only parent. Both subjects use the same opt-in
`compiler` build profile, full fresh source-to-Object pipeline and 60 runtime
flags. Hardware: Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2.

| Measurement | Control range (4 runs) | Candidate range (4 runs) | Mean change |
| --- | --- | --- | ---: |
| Compiler, s | 21.033197–21.327281 | 20.256196–20.609357 | -3.75% |
| Whole process, s | 21.207516–21.415445 | 20.368044–20.776890 | -3.44% |
| Parse, s | 2.403830–2.422354 | 2.421427–2.453079 | +0.72% |
| Frontend, s | 2.511881–2.533072 | 2.496730–2.537529 | -0.41% |
| Native, s | 16.040961–16.299158 | 15.264908–15.546801 | -4.93% |
| Canonical lowering, s | 0.206173–0.208263 | 0.097368–0.098452 | -52.76% |
| Preparation, s | 3.543470–3.626422 | 2.990355–3.053991 | -15.83% |
| Numeric generation, s | 3.838597–3.883676 | 3.840729–3.869394 | -0.05% |
| Source CFG, s | 2.326045–2.412560 | 2.334347–2.412966 | -0.57% |
| Cleanup including compaction, s | 3.399211–3.531317 | 3.335753–3.472631 | -1.63% |
| Compaction, s | 0.166054–0.170065 | 0.170075–0.171064 | +1.89% |
| Retired instructions | 318,971,858,197–319,231,416,438 | 311,348,789,415–311,604,359,979 | -2.37% |
| Peak RSS, bytes | 5,910,298,624–5,912,788,992 | 5,910,904,832–5,912,281,088 | +0.01% |
| Peak footprint, bytes | 5,879,960,760–5,882,451,128 | 5,880,599,760–5,881,976,016 | +0.01% |

Runs are serial control/candidate/candidate/control then
candidate/control/control/candidate. Both blocks improve compiler time (3.51%
and 3.99%). Every candidate compiler, process, native, preparation and instruction
observation falls below every control. Parse time is an adverse 0.72% higher;
compaction is 1.89% higher and both memory means are slightly higher. All contrary
observations remain. These finite cohorts are descriptive, not population
estimates. Both subjects include retired SSA value compaction and its measured
cost. Native substage durations are nested, not additional whole-process costs.

Builds, tests, screens, fixture compiles, CPU sampling, hashing, copying and packet
work do not overlap primary timing. The observed 403.891-second compiler rebuild
uses existing Cargo caches, is excluded from Mech compilation timing and is not
a matched cold-bootstrap comparison. Both screens and all eight primary runs
reproduce every qualified output byte. Fourteen successful fixture pairs cover
six complete array/enum programs and eight checked scalar programs, also matching
all four output files. Generated program code and metadata are byte-identical;
no new compressor-runtime timing campaign is claimed.

The final source passes **152 engine native-artifact tests, including four new
graph tests**. Those tests compare original discovery/failure order and incoming
edges over seeded graphs, acyclic ready order, duplicate edges, cycles, high sparse
IDs, nested/repeated queries and invalid sites. An earlier direct run of the exact
production module's four tests duplicates this coverage and is not counted again.
The control copy initially lacked execution permission; its failed warmup exits
126 before compiler execution and remains outside primary timing. Repairing only
the file mode preserves its frozen bytes and hash. A packet-curation correction
for an unnecessary nonexistent source path also remains separately recorded.

A separate diagnostic control compile samples five requested one-second windows:
preparation, numeric helpers, source CFG, cleanup and main machine-code emission.
All output bytes match. The packet retains every command, raw/demangled sample,
self table and the existing demangler's source/pin. Main-thread waits are excluded
from hotspot tables. Samples are neither phase-exclusive elapsed measurements nor
exact source-line attribution. They identify repeated liveness, scope-table work,
helper traversal analysis, SSA lookup, phi removal and register allocation as
remaining work. A recorded next experiment targets the full-program omitted-edge
scan in `Scope::supported_prefix`; no saving from that future change is claimed.

The [compact result](../../benchmarks/mech-deflate/native-profile/scope-graph-compile-apple-m3-20261114.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/scope-graph-compile-apple-m3-20261114.archive.json)
pin the complete [benchmark evidence packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md).
It includes exact source/base snapshots and patches, flags, commands, raw timings,
all outputs, test logs, corrections and pure offline replay. Compiler and test
executables are excluded and hash-pinned. Python 3.11+ replay reconstructs the
summary without compilers, benchmarks, original scratch paths or external services.
