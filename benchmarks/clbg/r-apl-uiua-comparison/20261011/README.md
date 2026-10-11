# R, APL and Uiua comparison

This cohort retains complete compact and optimized application programs for all ten tests, the original/reference sources and provenance, small and adversarial checks, ordinary runtime/library installation recipes, immutable source audits, and resumable full measurements. Locally authored ports are identified in each language manifest. They are a candidate pool, not a claim about an absolute language limit.

Canonical full workloads are nbody 50,000,000 steps; Mandelbrot 16,000×16,000 pixels; spectralnorm n=5,500 with forty on-demand products; fannkuchredux n=12 with every permutation; binarytrees maximum depth 21 with every actual node allocation and traversal; FASTA n=25,000,000 with 2n/3n/5n record sizes; k-nucleotide and reverse-complement on that complete FASTA file (the THREE record has 125,000,000 bases); regexredux on FASTA n=5,000,000; and pidigits 10,000 digits with both exact quotient extracts. No reduced workload or extrapolated time is chart eligible.

Every timed sample starts a fresh process and includes startup, interpreter/package loading, worker creation, complete input, all algorithm work, complete stdout to a regular file, and shutdown. Source preparation, compilation, output validation and hashing are excluded. Three output-validated full samples and three fresh Mech AOT control samples are required before a comparison is admitted. AC power and Low Power Mode off are required. Concurrent compiler load, changed power, failed monitoring, source/runtime/input changes, timeout or a process-group RSS violation prevents admission.

All cohorts use the shared advisory lock `/tmp/mech-clbg-timing.lock` for each full process. This runner includes planning pilots and input generation under that lock. It completes all three samples per short benchmark before starting the longest cases. Performance failures and every discarded attempt remain archived. Process sessions and their workers are killed on timeout/interruption; valid earlier samples can be resumed. The default timeout ceiling is 14,400 seconds per attempt; optional small planning pilots choose a recorded shorter ceiling. Those pilots are never measurements in the chart.

From this directory, reproduce ordinary runtime/support installations:

```sh
python3 install_r_runtime.py
./prepare-apl-runtime.sh
python3 install_uiua_runtime.py
```

R uses native ARM64 GNU R 4.6.1, its standard bytecode JIT, and ordinary CRAN GMP for Pi integers. APL uses GNU APL 2.0 and its ordinary array/PCRE operations; Pi uses a general GMP adapter, and regex v2 uses a general PCRE2 literal-substitution adapter. Uiua uses the official ARM64 Uiua 0.19.1 distribution and a general GMP adapter. Platform dependencies and exact pin/build details are in the manifests/install recipes. All locally authored adapter source is retained under `support`; ignored `.build` contains installed runtimes, package sources, caches, native support binaries and generated data. There are no benchmark host kernels.

Prepare and validate application sources (all are already retained):

```sh
python3 prepare_r_sources.py
python3 validate_r.py
python3 validate_r_adversarial.py
R_LIBS_USER="$PWD/.build/toolchains/r-library" .build/toolchains/R-4.6.1/bin/Rscript --vanilla audit_r_semantics.R "$PWD" > results/r/semantics-fixtures.tsv
R_LIBS_USER="$PWD/.build/toolchains/r-library" .build/toolchains/R-4.6.1/bin/Rscript --vanilla audit_r_pidigits.R "$PWD" > results/r/pidigits-work-fixtures.tsv
python3 prepare_r_manifest.py
python3 validate_apl.py
python3 prepare_uiua.py
python3 validate_uiua.py
python3 validate_uiua_adversarial.py
python3 prepare_uiua_manifest.py
python3 prepare_inputs.py
```

`prepare_inputs.py` regenerates missing ignored full stdin files with the visible optimized R FASTA source and checks the established canonical SHA256 pins. The existing Mech AOT controls are pinned from the prior source audit, with the current nbody scheduling artifact selected from its separate replication report. Their compiler/build recipes remain in those prior cohorts; do not rebuild or change them during a frozen run.

After all independent work reviews and source corrections are complete, freeze sources and run:

```sh
python3 prepare.py --languages r,apl,uiua
python3 run.py --languages r,apl,uiua --pilot --samples 3
python3 summarize.py
```

The same run command resumes existing valid samples. A subset may be measured first, for example `--benchmarks regexredux,revcomp,pidigits`; later commands resume the remaining benchmarks. `--pilot-only` performs scheduling probes only. `--timeout` and `--max-rss-mib` record explicit attempt ceilings; their changes must not silently alter workloads. Freezing must not be repeated after source edits while reusing old timing evidence.

`source-audit-{r,apl,uiua}.json` pins each immutable preparation separately. `results/timings-{language}.json` retains all attempts, validations, samples, aliases, failures and pending work. `results/summary.{json,csv,md}` admits valid partial coverage without hiding failures. Seconds are medians. Time×Mech divides by the freshly measured fastest Mech AOT control; fastest LOC and smallest LOC compare with their respective Mech source endpoints. Values below 1× mean faster or shorter than Mech.

Aliases are allowed only for byte-identical source with the same runtime, command shape, working directory, environment and support pins. Their evidence points explicitly to the measured source. Compact/optimized roles with different worker flags are separate executions. LOC counts visible application code, with every independent statement/binding counted separately; a long literal or nested expression remains one expression. General standard/library operations are excluded uniformly. Compressed source size is not used.

`evidence/r-pidigits-extra-work-normalization` preserves original R Pi sources, generator, manifest and checks from a conservative work-normalization experiment. `r-pidigits-work-policy.json` records the correction: the established cross-language policy permits the published Rust #4 impossible-candidate q>r guard, but requires both exact quotients whenever a candidate is actually tested. These older R source hashes are not rejected under that policy. Compact R performs two extracts even on impossible recurrence attempts; optimized R uses the allowed guard. The dynamic fixture reports recurrence attempts, tested candidates and exact quotient operations separately; it rejects a one-extract/remainder substitute. No experiment source has admitted full performance samples.

The current R fannkuch shortlist uses bounded factoradic permutation matrices, vectorized prefix gathers, and all canonical flips: the serial compact source is 30 LOC and the eight-worker optimized source is 37 LOC. The original 35/50 LOC loop implementations and their passing checks remain archived as validated candidates. Their small planning results do not enter chart data.

APL regex v1 full attempts failed the runtime workspace limit. Those source hashes and failed attempts remain in the manifests/evidence. The additive v2 candidates retain all nine APL regex counts and all five actual substitutions, using a general PCRE2 binding to avoid nested copies of the large subject and full-length boxed replacement temporaries. The GNU APL runtime and memory guard are unchanged. Reproduce its support build and preparation before freezing:

```sh
python3 - <<'PY_BUILD'
import json, subprocess
from pathlib import Path
report = json.loads(Path('results/apl/regex-v2-build.json').read_text())
subprocess.run(report['command'], check=True)
PY_BUILD
python3 validate_apl_regex_v2.py
python3 prepare_apl_regex_v2.py
python3 - <<'PY_MERGE'
import json
from pathlib import Path
path = Path('apl-manifest.json')
base = json.loads(path.read_text())
extra = json.loads(Path('apl-regex-v2-manifest.json').read_text())
rows = {row['key']: row for row in base['programs']}
rows.update({row['key']: row for row in extra['programs']})
base['programs'] = list(rows.values())
path.write_text(json.dumps(base, indent=2) + '\n')
PY_MERGE
```

The controlled runner repair is recorded in `evidence/harness-v1-partial` and `evidence/harness-repair-staging`. It strengthened process-group cleanup, interruption classification, and logical/resolved dynamic-library pins; no previously measured application/runtime binary changed. `resume_after_guard_repair.py` preserves only execution-identical source/runtime/command/environment rows, records retired candidates and archive hashes, and reindexes every accepted attempt. It is an explicit audited migration, not an ordinary refreeze after an application edit. Current interrupted attempts remain visible and never become successful samples.

The next approved additive preparation uses `apl-regex-v3/manifest.json`,
`apl-revcomp-v2/manifest.json` and `uiua-spectral-v3/manifest.json`. Their own
READMEs retain build/source/validation recipes. APL regex v3 adds an arbitrary
PCRE2 count operation, retaining all nine application-driven counts and five
ordered substitutions. APL reverse complement v2 keeps record parsing,
complementing, reversing and line wrapping in bounded application arrays, using
ordinary seekable scratch-file I/O. Uiua spectral v3 performs every coefficient
division freshly with native outer-addition/array primitives; the selected
compact and bounded forms are 9 and 17 LOC. The validated readable 13 LOC form
is retained but not selected for full timing.

The original scalar Uiua compact spectral program completed one exact canonical
trial in 2307.811771667 seconds. That real attempt and source are retained; the
approved additive shortlist replaces scalar coefficient composition rather than
repeating the same avoidable interpreter overhead. The new sources require
their own three full validated trials before admission. This decision does not
claim a measured speed for the replacement programs.

Only after all active benchmark processes exit and the preparation review is
approved, inspect and apply the explicit migration:

```sh
python3 refreeze_additive.py
python3 refreeze_additive.py --apply
python3 run.py --languages r,apl,uiua --benchmarks regexredux,revcomp,pidigits,spectralnorm --samples 3
python3 run.py --languages r,apl,uiua --benchmarks mandelbrot,fasta,knucleotide --samples 3
python3 plan.py --languages apl,uiua
python3 run.py --languages r,apl,uiua --benchmarks binarytrees,nbody,fannkuchredux --samples 3
python3 summarize.py
```

The migration takes the common mutex, archives old reports/audits/manifests and
the unchanged common harness, and freezes only APL and Uiua. It preserves the
normal preparer's pre-revision output and pins the revision script itself. It
assigns new keys to the unchanged Mech controls for affected regex, reverse
complement and spectral comparisons, requiring three newly interleaved control
samples. Old control trials remain historical evidence. Execution-identical
application samples are preserved; new sources start with no samples. It never
retroactively adds newly recorded fields to old attempts.
