# Runtime hook inventory refinement

Source-only review of tagged Julia1.13.1 runtime implementation, not actual symbol
resolution or an exhaustive absence proof. No runtime observer has run. All
loaded-image addresses, C/C++ linkage names, symbol availability and complete
scored control paths still require pinned native inspection under the common
mutex before any observation or AOT qualification.

[interpreter.c](https://github.com/JuliaLang/julia/blob/v1.13.1/src/interpreter.c)
has four explicit entry routes: ordinary method invocation, opaque closures,
toplevel thunks and deprecated toplevel expression evaluation. The last route
enters the expression evaluator directly; watching only the other three would
leave a gap. The candidate oracle now includes jl_interpret_toplevel_expr_in.
This rejects source evaluation on scored paths along with method fallback.

[jitlayers.cpp](https://github.com/JuliaLang/julia/blob/v1.13.1/src/jitlayers.cpp)
contains a runtime ABI adapter generator that creates its own LLVM module and
submits it to the JIT engine. Watching only ordinary method-emission hooks would
miss that route. The candidate adds jl_jit_abi_converter_impl and the actual
JuliaOJIT::addModule boundary. These describe Julia runtime code generation, not
vendor PCRE2 regex code generation. The latter remains allowed and disclosed.

Existing candidate method-emission/unspecialized-generation hooks remain useful
corroboration. The unspecialized generator may return immediately for an already
available native pointer, so its event is conservatively unqualified pending
exact source/argument semantics review. Do not silently discard that event or
infer generation solely from trace-compile output.

Resolve the C++ engine hook by its real loaded-image symbol/actual address; a
human-readable name or an unresolved/pending debugger breakpoint is insufficient.
If the released runtime strips or inlines a needed hook, qualification stops until
an unchanged-runtime observation method is independently reviewed. A rebuilt or
instrumented libjulia is not a substitute for the scored runtime's evidence.
Pins include libjulia/libjulia-codegen and every child/worker image. Observation
must be armed before runtime initialization on all threads through shutdown.

proof_candidate.py now expects eight semantic hooks and explicit actual linkage
name/source-route artifacts. Its five existing forged-record fixtures reject
each interpreter/JIT category with an empty trace and preserve the missing-native
canary requirement. They are still unexecuted; even a passing synthetic oracle
returns runtime_aot_proven=false and chart_admission=false. Root must establish
actual route completeness before fixing a native observation manifest.
