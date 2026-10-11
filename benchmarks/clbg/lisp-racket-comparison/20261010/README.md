# Common Lisp and Racket comparison

The complete published Benchmarks Game pool is preserved: 33 Common Lisp sources
and 26 Racket sources. `manifest.json` pins original sources, HTML pages, source
links and the BSD license. Published Linux timings are discovery information;
the comparison uses local Apple M3 measurements.

Nineteen separately named `-faithful` sources repair platform compatibility,
output ordering or differences from this study's shared required workload.
Originals remain in `upstream/` and `sources/`; superseded originals are excluded
before selection. `corrections.json` describes each change. Corrected algorithms,
helpers, arithmetic macros and application FFI declarations all count toward LOC.
General GMP arithmetic and CL-PPCRE regex matching are library support, with no
native benchmark executor.

`results/timings.json` is the resumable measurement record. Only a `complete`
record is accepted by `summarize.py` and the chart importer. Every one of the 59
eligible sources and ten unchanged Mech AOT controls gets three shuffled,
sequential, fresh-process full-workload runs. Full output is validated against the
shared cohort; N-body energies use the existing numeric tolerance. Correct output
is supplemented by the work review in `source-audit.json`.

## Execution

SBCL 2.6.9 compiles applications into AOT saved cores. Racket 9.3 CS uses native
Apple Silicon modules precompiled by `raco make`. Original source optimization
declarations and worker partitions are retained, with default heap and garbage
collection settings. Build/runtime commands, sources, cores/modules and
dependencies are pinned. The Racket installer checksum matches the official
release checksum. No `PLT_CS_*` override was set for this cohort.

The timed interval includes runtime/core/module loading, complete input,
computation, full output to a regular file and shutdown. Any compilation performed
by the application itself remains in the interval. Initial SBCL compile/save and
Racket module precompilation, validation and source audits are excluded. No
startup time is subtracted. Runs require AC power, Low Power Mode off and no
competing compiler jobs.

## Work and LOC checks

Shared requirements include floating-CDF FASTA, all seven growable nucleotide
maps, both pi-digit extracts on every attempted step, complete matrix products,
every required tree allocation and lifetime, and full-input regex/output work.
Published optimizations are retained when consistent with those requirements.
No period cache, indexed nucleotide histogram, selective-query shortcut, skipped
matrix remainder, reused tree or native workload function is admitted.

Six tree candidates have allocation evidence: two SBCL disassemblies and four
Racket identity probes. Constructors allocate distinct leaves and interior nodes.
Trees are constructed before traversal and long-lived roots persist across
temporary work. The original Racket struct source retains its extra item field
in addition to the two required child fields.

`loc.py` is the initial reader pinned by the measurement record. `loc_final.py`
makes two later metric corrections: Racket syntax-template ellipses are structural
repetition markers, and packed independent Common Lisp DO initializers count
separately, like LET initializers. No source or binary
changes. `audit_loc_final.py` verifies initial counts, recomputes final counts,
records differences, and runs 18 reader/statement fixtures. Packed independent
body expressions, initialized bindings and assignments count separately;
parameters, argument separators and literal data do not. No compressed metric.

After the full record completes:

```sh
python3 audit_loc_final.py
python3 summarize.py
```

`results/summary.json` selects each language's measured fastest and fewest-LOC
source for every workload. It records ratios to both fresh controls and existing
chart Mech AOT timings. Integration preserves previous source bytes, timing
samples and endpoint choices.

## Machine-state audit

`results/machine-state-audit.json` records the requested read-only process audit.
No MATLAB engine or stale benchmark job was found. Idle MathWorks services,
Opera, Codex and browser automation helpers remained open. Memory pressure was
low; the accompanying two-second `top` check showed no swapping. Animated wallpaper
and WindowServer used a small fraction of one core. The active Lisp/Racket test
was the substantial CPU consumer. No app was terminated during this audit.
