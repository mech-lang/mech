# Separate Uiua parallel Mandelbrot experiment

Two additional candidates apply ordinary Uiua `spawn`/`wait` to the retained
11 LOC whole-row and 14 LOC active-pixel programs. All original sources,
qualified results, sealed metadata and durable-coordinator reports remain
untouched. Expected source counts are 16 and 19 LOC, including every helper.

The application creates `min(8,N)` groups of row indices using their remainder
modulo the worker count. Each group is passed to one native OS thread; there
are no nested spawns and no automatic thread pool. Each worker performs the
unchanged Uiua recurrence, mask/select/scatter and packing operations and
returns a boxed matrix of packed rows. Workers never write output. The parent
waits for every returned block, joins the blocks with ordinary array operations,
sorts their original row indices, restores exact order with native selection,
and writes the complete binary payload. The parent writes the two PBM header
lines before waiting. Thread startup, completion, collection, ordered
output and process shutdown belong to any later full timed execution.

Strided groups spread geometrically different rows across workers. This is a
general subdivision and ordering technique; it uses no analytic interior
test, input cache, native workload kernel, floating-point approximation or
pairwise membership matrix. Holding complete packed output uses linear space
in its size; transient numeric storage is part of the measured RSS.

All scripted preparation, native runtime probes, heavy hashing, independent
references and validation acquire `/tmp/mech-clbg-timing.lock`. Worker
observations distinguish total OS threads from compute workers. Diagnostic
gates hold all workers on ordinary channel receives and compare macOS native
task thread counts to an otherwise identical parent-only gate; these diagnostic
sources are never timed. Worker traces are returned as data and printed only
by the parent, avoiding unordered concurrent output.

Uiua's ordinary Rise index sort uses Rayon internally. The additional
candidates explicitly set `RAYON_NUM_THREADS=1` to cap that generic sorting
pool. The worker gate and parent gate initialize the same one-thread pool
before observing counts. This separates runtime sorting overhead from the
`min(8,N)` Mandelbrot workers; no automatic Spawn pool is used. Fresh
sequential Uiua comparison runs use this same explicit environment.

The complete output suite includes widths 1, 2, 7, 8, 9, 13, 31, 63, 64, 65,
129, 200 and 257. Three parallel returned-state traces per family verify every
required recurrence in binary64, including the escape-causing iteration.
Seven synchronized worker gates per family verify the actual worker count.
Every validation attempt retains its own directory, including failures, while
`results/validation.json` points to the most recent attempt's report.

The proposed full study must be reviewed separately before execution. It will
write only this directory's experimental report, use the frozen standard
`N/run.py.execute` and `PinGuard`, retain compiler/power/RSS/process-group
monitoring, and validate exact full `N=16000` output. Three fresh full samples
of each Uiua form and three interleaved unchanged Mech fast controls are
planned. Experimental results remain ineligible for the chart until separate
approved admission; preparation does not refreeze or alter the active cohort.
