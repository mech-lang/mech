# Julia AOT cohort proposal v1

Read-only research/source candidate. No runtime execution, package resolution,
download, source hash, compile, precompile, fixture, native validation, timing or
queue has run. All later operations require the common timing mutex and root
review of the concrete immutable packet. Existing Julia JIT sources/reports/
samples remain unchanged, as do all six requested languages and V report scope.

Existing measured Julia is `/tmp/mech-clbg-julia/julia-1.13.1/bin/julia`, with
`--threads=8,0 --startup-file=no --history-file=no -O3` and standard libraries.
Current explorer endpoints cover all10 canonical workloads with20 fastest/
smallest roles but17 distinct source identities: nbody, spectralnorm and FASTA
share a source across their two roles. AOT must cover every role with explicit
identity/alias metadata; never invent three additional distinct source programs.
`source-inventory.jq` binds each role to the current exact displayed source and
preserved report metadata. This is a metadata candidate, not new hash evidence.

## Supported route and actual limit

PackageCompiler2.4.3 exposes `create_app`, a native launcher and precompiled
`julia_main()::Cint`; runtime application arguments are available as ARGS.
`incremental=false`, complete dependency inventory and explicit
`sysimage_build_args` allow an independent application image. The conservative
candidate is one app per selected source/role, built with Julia1.13.1, ordinary
stdlib/library code, exhaustive precompilation plus IR stripping, then launched
with `--compile=no`,8 compute threads and startup/history disabled. The stronger
entry candidate now uses `create_library` with an exported `Base.@ccallable`
entry and `direct_driver.c`; the default app wrapper evaluates runtime setup and
entry expressions, so it is not accepted unchanged. The library's documented
init/shutdown API initializes the runtime and directly calls compiled entry code.
All launcher/glue inputs remain counted and pinned. Compiler,
trace generation and image/link creation remain outside timed runtime startup.
[PackageCompiler app docs](https://julialang.github.io/PackageCompiler.jl/stable/apps.html)
and [tagged2.4.3 API](https://raw.githubusercontent.com/JuliaLang/PackageCompiler.jl/v2.4.3/src/PackageCompiler.jl),
[library ABI docs](https://julialang.github.io/PackageCompiler.jl/stable/libs.html),
[default app wrapper](https://raw.githubusercontent.com/JuliaLang/PackageCompiler.jl/v2.4.3/src/embedding_wrapper.c)
and [library initializer](https://raw.githubusercontent.com/JuliaLang/PackageCompiler.jl/v2.4.3/src/julia_init.c).

This route is supported, but all20 forms are not yet proven viable. Julia1.13.1
explicitly falls back to `jl_fptr_interpret_call` for missing native methods when
compilation is disabled. A default/custom sysimage, `--compile=no`, or an empty
JIT trace alone therefore cannot establish language AOT. Actual final image
method coverage/IR policy and interpreter/JIT absence on scored paths are gates.
Tagged image serialization can retain method IR in modules declared compile=no
or compile=min. Inspect actual final-image source/IR and those module policies;
the --strip-ir flag alone does not establish global interpreter absence. Missing
IR causes MissingCodeError in the interpreter path, providing a useful actual
negative-control candidate after build; full output with empty traces still
requires the reviewed image/runtime proof.
Safe trimming is an alternative only where the actual compiler proves every
reachable call; unsafe trim modes cannot qualify missing dynamic coverage.
[Tagged Julia method dispatch](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/gf.c)
and [Julia CLI](https://docs.julialang.org/en/v1/manual/command-line-interface/),
[tagged image stripping](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/staticdata.c)
and [tagged interpreter](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/interpreter.c).

Julia identifies native method instances from executed argument types and builds
code images separately; images restore compiler/runtime state. Precompile trace
generation therefore uses separate bounded processes and varied type/branch/IO/
worker cases. Image building never receives canonical full inputs, saved answers
or a fixed full n. A method missing native coverage fails qualification, leading
to a new preserved precompile/source revision rather than JIT/interpreter timing.
[Julia AOT internals](https://docs.julialang.org/en/v1/devdocs/aot/).

## Exact source and runtime state

Per-source static entry mapping is required before executable adapters. Existing
nbody-julia-5 initializes mutable global `bodies`, then executes its driver under
`!isinteractive()`. Existing fasta-julia-3 parses global `n`, and `perf_fasta(_n)`
reads that global n. Importing complete original scripts into an image could run
work at build time or serialize mutable/warmed state. Runtime include/eval of the
original scripts would reintroduce top-level interpretation. Neither qualifies.
`ENTRYPOINT_REVIEW.md` records every distinct source's driver/state and
source-sensitive proof obligations. No derived-module policy is treated as
resolved until its exact code and native artifact are reviewed.
`RUNTIME_STATE_STRATEGY.md` proposes typed uninitialized globals with compiled
runtime initialization, explicit const-binding changes and fresh matched JIT
where selected. `AOT_PROOF_PLAN.md` supplies the unchanged-binary hook/calibrated
missing-native canary candidate, including empty-trace interpreter rejection.

Preserve the exact original files. Add explicit, source-reviewed derived module
and `julia_main` adapters only where needed: retain original algorithm/function
bodies and sequencing; move runtime driver/state initialization into compiled
entry code; reset mutable global state per fresh process; take runtime ARGS and
stdin; keep regex object/pattern construction in its normal whole-program
boundary. Record every copied/extracted span and change. Count all authored
adapters and app glue in later qualified LOC, with byte-equal compiled-input
inventory, compiler/package/stdlib/native library/image/launcher pins. No domain
kernel, answer cache, constant full-n specialization, frozen input/stream/worker
result or serialized post-warmup state is permitted.

The immutable Node spelling in the existing tree sources does not establish
whether the scored compiler allocates a fresh leaf for every call. Review actual
unchanged scored compiler/native allocation evidence before accepting either
tree form. If a fresh-leaf source correction is necessary, preserve all old JIT
records and source bytes, author a distinct corrected-source revision, and obtain
fresh matched JIT measurements of that same revised source as well as AOT. An
AOT versus JIT comparison must bind the same algorithm/source and runtime worker
policy; a changed AOT tree cannot be compared as if the old JIT source were its
matched implementation. Existing distributed tree source must retain the actual
measured launch/worker policy rather than infer added processes from its macros.

Standard PCRE2 regex pattern compilation/JIT and ordinary GMP/GC/IO behavior
remain legitimate library reuse, explicitly disclosed. The no-JIT requirement
concerns Julia-language methods, including actual scored call/worker/IO paths,
not generic vendor regex machinery. Ordinary GC remains enabled. Complete input,
state construction, scheduler/runtime initialization, all work/output and exit
are timed in a fresh process. Precompile/compiler/link work is excluded.

## Qualification and full suite

Root first reviews all20 role mappings, exact original/derived/adapter sources,
runtime-state reset, package version/commit and actual image/launch commands.
After approved locked bootstrap, preserve raw package resolution and every
compiled input. Run compiler/IR/native-code coverage evidence, varied bounded
complete-output cases and essential tree/map/two-division work proofs. Record
actual runtime compilation policy, missing-code negative control, Julia JIT and
interpreter event evidence, and demonstrate no runtime include/eval/source load
or new package compilation. Final image coverage must fail closed for a missing
method; bool metadata without raw source/runtime/negative-control evidence is
insufficient. Global empty trace alone does not satisfy this gate.

Only then root approves ten independent full manifests, three shuffled fresh
samples per admitted AOT form with fresh unchanged Mech controls. Preserve all
old JIT samples, label a separate Julia AOT execution entry/family, include startup
and complete stdout to a regular file, leave counts null until qualified LOC.
Use existing exact process ownership/environment/power/cleanup and isolated
source-failure rules. No partial or estimated medians. If only some forms qualify,
retain truthful failed evidence and leave the full Julia AOT goal incomplete.
