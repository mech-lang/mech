# Native AOT route after the closed attach calibration

Source-only plan; no build, fixture, native workload, permission change or new
debugger attempt is authorized by this note. Current PackageCompiler pilot and
all failed attach evidence remain unchanged and unqualified.

The two-cell generic attach job completed normally, but both LLDB commands
hit their 45-second bounds before a returned-attach receipt/event loop. They
executed no target helper and reaped both exact owned children with no remaining
groups. The current instrumentation cannot distinguish blocking in the attach
call from its immediate post-return state/module queries, which run before the
returned receipt is saved. The closed under-common 348-pin review is at
`continuation/jobs/julia-calibration-v2-closed-review-v1/review.json`, SHA256
`5cdb703a95dd53c4fd7e5467287c993beb3b2d18b19e614d1e0a6889d1e43d73`.

## Proposed configuration: safe whole-program trimming with stripped IR

Use the existing pinned Julia 1.13.1 compiler and official shipped trim builder,
with the actual exported `clbg_entry(Cint, Ptr{Ptr{Cchar}})` as a concrete native
entry root. Build a NEW image using `--experimental --trim=safe --strip-ir`,
explicit nonincremental object/bitcode output and the existing optimization,
architecture/thread policy. Capture every verifier diagnostic. Any unresolved
reachable dynamic call is a build failure; `unsafe`/`unsafe-warn` are forbidden.
The tagged [system-image documentation](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/doc/src/devdocs/sysimg.md)
describes tracing concrete entry roots, rejecting unresolved calls and removing
IR from fully compiled trimmed images. This supplies the closed program graph
rather than demanding an inventory of every unused runtime function.

The actual original n-body algorithm, typed fresh `bodies` initialization,
argument handling and original driver stay in Julia. The existing C launcher
only starts the runtime, calls the exported entry and shuts down. Its compiled
wrapper alone proves no workload coverage. Any adapter typing/IO change needed
by the verifier must be explicit, source-derived, separately counted and used
by a fresh matched JIT control; never substitute a C implementation of the loop.
The official trim compatibility patches are a new disclosed runtime configuration,
not a retroactive proof or replacement of the current pilot's historical image.

The already recorded/pin-verified builder sources exist under
`.build/packagecompiler-bootstrap-v2/restored-runtime/julia-1.13.1/share/julia/test/trimming/`:

- `juliac-buildscript.jl`: `392b91e0810f7d46b019afd7c481381b90156833cb1d27df0e05467d0fba64df`.
- `juliac-trim-base.jl`: `3dd7c0621df19510d7eab8b88b96f5ec32d9cd93d4aa4019ca908666478e4967`.
- `juliac-trim-stdlib.jl`: `3740d83feed070e9f00f5ffbecdbaea2e2dd6a1c71ce23fa33e9c47e15b0b440`.
- `abi_export.jl`: `4527857e6c7846eff54ffe8ac3406bb843c525c584c68323eeb9c92d7e038128`.

These exact pins were consumed under common from the original pilot report.
The [tagged builder](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/test/trimming/juliac-buildscript.jl)
registers native task/runtime roots, optionally adds all C-callable entrypoints,
applies the shipped trim compatibility sources, exports ABI metadata, and clears
build-time arguments/load/depot paths. Its input protocol is source path,
output kind (`--output-lib`), C-callable-export boolean (`true`), and optional
ABI metadata output. A root-reviewed worker can use these shipped sources
without installing a new JuliaC package; [JuliaC](https://github.com/JuliaLang/JuliaC.jl)
is the supported compile/link/bundle frontend and confirms the same entry route.
No concrete build invocation is released until actual builder dependency,
compiler/linker/bundle/image pins and full source entry have been reviewed.

## Acceptance evidence that makes the configuration fail closed

1. Reuse exact source adaptation and immutable original spans, but create fresh
   output IDs. Pin compiler, complete builder/compatibility dependency closure,
   exported entry roots and ABI metadata, object archive/bitcode members,
   generated native image, C glue/link receipts and every loaded runtime/library.
   Require safe verification to finish with zero errors; retain its full raw
   diagnostics and actual option values. Preserve native debug/symbol information
   for owner review rather than stripping it away merely for smaller artifacts.
2. Bind native entry addresses/function tables and object/disassembly lineage to
   the authored initializer/driver/n-body kernels. Reject a wrapper-only image,
   a mismatched source/signature, ambiguous native owner or unregistered callback.
   Verify the actual final image has no usable Julia method/inferred/opaque-closure
   IR on the reachable program path. Require the C init wrapper to select this
   exact image and retain compile-disabled, source-loading-disabled execution.
3. Close fallback capability for the reachable program. Safe graph verification
   plus verified IR removal must leave an omitted language method unable to run:
   it must fail for missing native code, never interpret or emit runtime code.
   Review the tagged image-loader/method-dispatch paths and actual artifact fields;
   record a separate negative omission/unresolved-dispatch case before admission.
   A verifier-rejected dynamic target validates the static gate; if a runtime
   missing-code case is used, prove its native coverage/IR absence beforehand.
   Neither a flag nor an empty trace is sufficient. If any unclosed fallback
   capability remains, stop this configuration rather than permit it silently.
4. Only after the configuration gate passes, run original varied bounded cases
   and canonical inputs with exact source, fresh state, work/output/allocation
   and owned process/image evidence. Then take three fresh timing processes per
   selected form under the common mutex, with matched Mech controls. No AOT timing
   is admitted while native coverage or fallback closure remains pending.

The [tagged dispatcher](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/gf.c)
can select interpreter dispatch when compilation is off, while the
[codegen fallback](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/codegen-stubs.c)
can still install interpretation without the compiler plugin. Thus simply
removing libjulia-codegen is insufficient. The
[tagged serializer](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/staticdata.c)
distinguishes trimming from ordinary strip-IR and may retain method IR under
module policies when only stripping. This is why the plan combines safe roots,
actual stripped-image inspection and failure on absent native coverage.

If the official trimmed configuration cannot close a reachable fallback, a
separate private runtime with all interpreter/code-emission entries replaced by
unconditional failure is another possible configuration, requiring a complete
source-route review and new image qualification. It would not prove the existing
scored runtime retrospectively. The official safe-trim route is the first option.
