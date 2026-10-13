# Julia official safe-trim build, first stage

Mutable source proposal, not executed or hashed. This three-file packet is the
concrete first build stage of `trim-aot-plan-v1/REVIEW.md`. It preserves the
historical PackageCompiler pilot and both closed LLDB calibration timeouts.
It runs no application, workload, timing, benchmark, linker, debugger or fixture.

`build.py` reuses the original pinned Julia 1.13.1 restoration and official
shipped `test/trimming/juliac-buildscript.jl`, ABI exporter, Base compatibility
patches and stdlib compatibility patches. The fixed four builder-source digests
and shipped Makefile digest are checked against both original bootstrap and
pilot receipts. Their complete saved pin closures are rechecked under the
common mutex, including original runtime images and compiler dependencies.
The original calibration root receipt is fixed at SHA
`5cdb703a95dd53c4fd7e5467287c993beb3b2d18b19e614d1e0a6889d1e43d73`;
its two exact target cleanups and zero helper executions are required. No LLDB
retry or new debugger path exists here.

The byte-exact original derived `ClbgNbodyPilot.jl`, `workload.jl`, `entry.jl`,
Project file, matched JIT loader and C driver are copied into a fresh build
directory. The saved adaptation receipt is fixed at
`7edf4e1baf8533c4cf3a1e88aa86b880f3691229a27cb2b9b4c29cf33a3df0f4`.
Original edit spans and untouched spans are rechecked; the workload text must
equal the saved derived source. No parser, counter, source adapter, app method
patch, alternate algorithm, C workload or input-specific precomputation is run.
`bodies` remains a typed but uninitialized binding until an eventual app call.
The original initializer, argv handling, two energy prints and all n-body
steps remain in Julia. The saved C driver is retained for a later explicitly
reviewed init/link stage; this worker does not link or execute it.

The first command uses the pinned restored Julia binary and original sysimage,
`--threads=8,0 -O3 -g2 --output-incremental=no --experimental --trim=safe
--strip-ir`, and simultaneous fresh object and optimized bitcode outputs.
It retains Julia source metadata and native debug information: neither
`--strip-metadata` nor native `strip` is selected. The official library protocol
is exact: source module, `--output-lib`, `true` to register all C-callable
roots, and ABI output. Its actual `clbg_entry(Cint, Ptr{Ptr{Cchar}})::Cint`
root includes original initializer/driver/kernel calls. Official task/runtime
roots and compatibility modifications are disclosed by the pinned builder.

`build_options.jl` is build-process instrumentation, separately retained rather
than treated as an application kernel. It asserts actual JLOptions safe=1
(the pinned header's `JL_TRIM_SAFE`), strip-IR=1, metadata-retained=0,
nonincremental=0, optimization=3, debug=2 and eight default threads. It captures
actual object/bitcode/input-image paths and exact library/root protocol in
TOML. After the unchanged official builder returns it asserts the app state is
uninitialized and builder args/load/depot paths are cleared. This receipt
precedes final verifier/code generation; it is never sufficient by itself to
claim success. No unsafe mode or automatic source/IO/error-handler rewrite is
available if the original code is rejected.

The actual command is owned and bounded for 1,800 seconds through unchanged
`c3-comparison/.../private-v3/guarded_prepare.py`'s existing `owned_command`.
The worker's real cwd is explicitly Julia's study root; its group ownership,
leader identity, cleanup/reap and stdout/stderr journals remain the established
service. The preparation module itself is loaded from SHA-checked raw source;
its existing guard/harness loaders and their original full pinned dependencies
remain unchanged. A fresh private owner lease and common inode 79859134 are
held throughout every input pin check, compiler child, output capture and
terminal journal. Fresh IDs only: neither output nor build directory may
already exist. Signals interrupt through the existing owned cleanup. Failed
or partial build bytes and the error journal are preserved, never overwritten.

The saved bootstrap environment is reused with only a fresh empty depot,
fresh temporary directory, explicit eight image compilation workers and
single-worker unrelated numeric-library settings. The app uses only Base;
this does not instantiate packages or import PackageCompiler. It consumes no
network credentials or user runtime environment beyond the already recorded
bootstrap values. All new output bytes are pinned in the terminal journal,
and original input pins are rechecked before completion even on failure.

After an actual normal compiler exit, the worker requires exact actual options,
nonempty object/bitcode/ABI/options artifacts and an unambiguous exported
`clbg_entry` with signed 32-bit argc/result and pointer-to-pointer signed
8-bit char argv. It records complete ABI metadata, artifact byte sizes,
digests and headers, plus full compiler diagnostics. It does not treat ABI
export or a successful wrapper as proof of kernel native ownership. Archive
member lineage, generated native image, exact C initialization, actual IR
absence, reachable fallback capability and a separate negative omission case
remain explicit subsequent gates. Root must read actual verifier diagnostics
and artifacts before authorizing those next stages. No runtime AOT or chart
admission is claimed by this first stage, even if safe compilation succeeds.

The official [tagged system-image documentation](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/doc/src/devdocs/sysimg.md)
describes concrete trim roots, safe rejection of unresolved targets and
independent strip-IR. The [tagged options implementation](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/jloptions.c)
confirms `safe` and both object/bitcode output switches. The shipped Makefile
uses the same nonincremental official library builder route. Those sources
were read; no build success is inferred from documentation.

Root next captures/freezes the exact three source files and original pilot
receipt under common, reviews the complete source and durable invocation,
then chooses a single fresh output ID. Run recipe (actual digest arguments
must come from that capture, not these placeholders):

```
bundled-python3.12 -I -S -B -u revisions/safe-trim-build-v1/build.py
  --revision official-safe-trim-nbody-v1
  --worker-sha256 <actual worker SHA>
  --options-sha256 <actual options wrapper SHA>
  --review-sha256 <actual review SHA>
  --pilot-sha256 <actual original pilot report SHA>
```

Use the existing durable launcher/private lease protocol. No direct launch,
freeze, source hash, native process or qualification happened during authoring.
