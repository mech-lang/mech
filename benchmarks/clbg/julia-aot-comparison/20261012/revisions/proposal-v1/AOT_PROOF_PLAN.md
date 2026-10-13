# Julia method AOT proof candidate

Source-only plan and negative-canary/observation contract. No canary/image/test/
debugger/normal native process has run. A flag, empty trace, stripped-IR option or
successfully completed schema cannot admit Julia AOT. All eventual tools and
native cases require root's exact packet review and the common timing mutex.

Tagged runtime facts: method invocation can install interpreter dispatch when
native coverage is missing under compile=no/min. Image stripping retains some
IR according to module policy. The tagged experimental macro supports compile=min
and a separate force-compile marker; these are distinct from a complete native
image. Raw final artifacts must prove what was emitted/retained.

- [Julia1.13.1 method dispatch](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/gf.c)
- [Julia1.13.1 interpreter entries](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/interpreter.c)
- [Julia1.13.1 image serialization](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/src/staticdata.c)
- [Julia1.13.1 experimental compiler options](https://raw.githubusercontent.com/JuliaLang/julia/v1.13.1/base/experimental.jl)

The canary's native C ABI entry calls a deliberately uncovered noinline method in
a compile=min module with a runtime input and mutable payload. It performs no
CLBG work and is never a replacement cohort. The retained-IR canary may produce
the correct increment and exit0 through the interpreter with an empty JIT trace;
the qualification oracle must reject that observation. A separately preserved
missing-IR target variant must reject missing native code rather than silently
run. Image inspection must first bind the exact target method and show no usable
specialized or generic native invoke. If compile=all emits native coverage anyway,
the canary is invalid and the proof stops; no missing-code result is invented.

Use an unscored debugger observation of the unchanged scored native launcher and
image, with native address breakpoints on actual interpreter entrypoints
jl_fptr_interpret_call, jl_interpret_opaque_closure, jl_interpret_toplevel_thunk
and jl_interpret_toplevel_expr_in,
plus native code-emission paths jl_emit_codeinst_to_jit and
jl_generate_fptr_for_unspecialized, the runtime ABI code generator
jl_jit_abi_converter_impl, and the actual JuliaOJIT::addModule engine boundary.
RUNTIME_HOOK_REVIEW.md records the concrete routes that would bypass the initial
five-hook proposal. Resolve each symbol/address against the actual
loaded, pinned Julia runtime image; preserve raw symbol/address/loaded-image and
all-thread event logs. A broad compile_method/cache lookup is not itself proof of
new native code. Review the tagged runtime's complete generation/interpretation
paths before freezing the final hook inventory; these named hooks are a concrete
candidate, not a claim of exhaustive coverage already established.

Observer must be armed before runtime initialization, cover every thread/child
and remain attached through shutdown. A breakpoint that cannot resolve, delayed
attachment, event loss, failed canary or changed image stops qualification. It
must not mutate algorithm source, add finalizers or alter method allocation.
Debugger overhead is unscored; actual timing uses the same original pinned binary
without the debugger. Pin the ordinary native ownership/command/environment and
observer target separately. If the host cannot supply trustworthy complete hooks,
the AOT proof stays incomplete; flags/IR claims are not a fallback admission.

After canary calibration, observe varied bounded cases and each full canonical
input for every selected source/role, preserving complete output and cold startup.
Do not infer full-path absence only from small inputs: collection resizing/sort/
thread/IO paths can differ. Review final image method/native invoke and surviving
IR/module-policy artifacts alongside these actual observations. Normal timed
samples then use the same pins/compile-disabled command and fresh runtime state;
missing-code/output failures remain failed source evidence and independent forms
may continue only with the existing literal safety gates. No partial AOT medians.

Vendor PCRE2 pattern compilation/JIT remains ordinary disclosed regex machinery.
The observer classification concerns Julia-language methods and their Julia
runtime generation/interpreter entrypoints; a PCRE2 JIT event alone is not a
Julia AOT failure. Every authored C/Julia adapter and proof-specific helper is
identified separately; only actual compiled scored glue belongs in scored LOC.

proof_candidate.py supplies rejection/shape rules only and always leaves AOT/
chart admission false. Its five forged-record fixtures require empty-trace
interpreter rejection, missing-code/JIT rejection, exact armed hook inventory and
the absence of automatic admission. Actual native canary proof and source/state/
allocation/canonical-output/full-measurement evidence remain required.
