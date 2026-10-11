# Fast scripting-language endpoints

This cohort completes the speed endpoints that the earlier compact-program audit
did not measure. It uses the unchanged full workloads from the native comparison:
50 million n-body steps, Mandelbrot size 16,000, FASTA size 25 million (also the
input to k-nucleotide and reverse-complement), spectral-norm size 5,500,
fannkuch-redux size 12, and binary-trees depth 21.

For Python, Ruby, Perl, Lua, PHP, and Node.js, select the fastest published time
from the already pinned October 8 tables, retaining every tie, including ties
caused by times rounded to minutes. If an entry cannot validate, try the next
published tier. LuaJIT tests every listed Lua source unchanged. All previously
measured compact programs remain candidates, so a smaller source can still win
on speed on this host. Julia's fast variants were already measured in the native
cohort. Regex-redux and pidigits already include published-fastest scripting
variants in their separate October 9 reports.

**Fastest means fastest measured among these selected candidates.** This is not
an exhaustive local speed sweep of every published scripting variant. Failed
sources, diagnostics, source hashes, complete physical LOC, runtime versions,
commands, and sample output validation are retained in the report. The source
axis is nonblank, noncomment physical LOC only.

Previously completed compact timings are reused only after checking their report
hash, source hashes, runtime hashes, at least three validated full-output samples,
and AC power with Low Power Mode off. New programs run sequentially in three
fresh processes, with randomized order within each benchmark. Startup, JIT
compilation, child workers, and complete output to a regular file are timed.
Output hashing is outside timing. A fresh native control is sampled three times
in each new batch and its drift from the preserved median is reported without
rescaling any result. The existing compiler-load and power-change guards discard
affected samples. Each accepted timed execution must validate the full output.

The listed programs assume a Linux execution environment. New Python parallel
invocations use a general launcher selecting `multiprocessing`'s `fork` policy,
preserving inherited descriptors and globals. New Ruby and Perl invocations use
a general preload redirecting `/proc/cpuinfo` to a pinned CPU enumeration fixture
for the host's eight logical CPUs. These launchers contain no benchmark work and
are included in process timing. Benchmark source files remain unchanged. PHP
uses its standard short-tag setting, tracing JIT, and unlimited language-level
memory setting; Ruby uses YJIT, and Node and LuaJIT use their default JITs.

Preparation and the resumable full run:

```sh
python3 benchmarks/clbg/fast-scripting/run.py --prepare
python3 benchmarks/clbg/fast-scripting/run.py
python3 benchmarks/clbg/fast-scripting/check_policies.py --prepare
python3 benchmarks/clbg/fast-scripting/check_policies.py
python3 benchmarks/clbg/fast-scripting/finish.py
```

A separate policy report checks the fastest Ruby fannkuch source with a four-worker
budget, avoiding fractional partition rounding at size 7, and retries the fragile
PHP binary-trees entries with JIT disabled. Every source remains unchanged; failed
attempts remain recorded. These policy runs start only after the primary sweep
finishes and validate several small executions before full timing.
The next listed parallel Python Mandelbrot candidate is also measured because the
64-worker variant spent substantial time merging output lists. PHP's parallel
Mandelbrot candidate is retried if the host's System V shared-memory limit has
been raised to accommodate its 32 MB output segment.

When the report is complete, `overview/plot_ten.py` merges its medians into the
white PNG mega chart. Every covered language gets fastest and fewest-LOC
endpoints, with one geometric-mean segment per language and faint benchmark
inputs. Lua still has no ten-benchmark mean because its pidigits program failed
under Lua 5.5; LuaJIT covers all ten.
