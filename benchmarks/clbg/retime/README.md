# Full-cohort rerun with Low Power Mode disabled

The old chart measurements did not record power state. After observing a large
AC/battery timing change and finding AC Low Power Mode enabled, the user requested
fresh timings for all benchmarks. This reruns the complete existing chart cohort
and the audited compact-runtime additions, including Lua and LuaJIT: 575 builds
across eight benchmarks. The original reports remain available for audit.

Use the same pinned programs, compiled binaries, compiler options, workloads,
source sizes and algorithm labels. The Mech programs use the existing v0.4
integration Cranelift AOT binaries. This is a timing rerun; it adds no kernels or
compiler special cases. Executables, invoked sources and Java classfiles are
hashed before preparation and checked before each benchmark. Full output must
validate before three sequential fresh-process samples in randomized order.
Startup, JIT and regular-file output are included; builds, tests, output hashing
and comparisons are outside timing. The timeout is four hours per invocation;
failures are explicit exclusions, never extrapolated times.

The final audit checks every pinned command asset again and verifies that the
original reports remain unchanged before fresh results are used in the charts.

All timed batches require **AC power with Low Power Mode off**. A monitor checks
power source and active Low Power Mode at both sample boundaries and every ten
seconds. A detected change discards the entire timed benchmark batch, including
its control. Competing compiler jobs delay sampling; samples overlapping detected
compiler activity are discarded and retried. Ordinary desktop activity and
undetected brief changes remain limitations of these local measurements.

The new chart uses fresh raw medians normalized within each benchmark. It does
not use old low-power times or rescale them using a new control. A complete fresh
sweep replaces the entire plotted cohort together; partial fresh results do not
replace individual points in the old chart. New compact-only languages retain
square endpoints without inferred fastest dots or tradeoff connectors.

To reproduce, with the recorded runtimes and compiled binaries available:

```
python3 benchmarks/clbg/retime/run.py --prepare
caffeinate -i python3 -u benchmarks/clbg/retime/run.py
python3 benchmarks/clbg/retime/finish.py
python3 benchmarks/clbg/overview/build_gallery.py
python3 benchmarks/clbg/overview/plot_mega.py
```

`caffeinate` prevents idle sleep for the duration of the run. It does not alter
power-mode preferences. Do not run compilation, plotting or browser verification
alongside timing. The report saves atomically after each output check and sample
and can resume completed benchmarks without rerunning them.

See `compact/README.md` and its source manifest for smallest-source selection,
normalized gzip/LOC definitions, LuaJIT compatibility mode and allocation-policy
caveats. Eight benchmarks are covered, not the full ten-test CLBG suite.
