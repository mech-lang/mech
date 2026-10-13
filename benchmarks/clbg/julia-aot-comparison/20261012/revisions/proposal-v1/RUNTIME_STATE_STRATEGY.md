# Runtime state adapter candidate

Source-only strategy; no Julia parser/compiler/runtime has evaluated these
snippets. Original files/JIT records remain immutable. The candidate uses typed
uninitialized globals where an original const heap binding cannot be populated
at runtime without carrying a build-time object. Julia's documented typed global
declaration supplies a stable type without an initial value. Constant redefinition
can invalidate code, so Core.declare_const/const rebinding is not an AOT shortcut.
[Julia scope/typed globals](https://docs.julialang.org/en/v1/manual/variables-and-scoping/)
and [module runtime initialization](https://docs.julialang.org/en/v1/manual/modules/).

This changes binding semantics/performance from const to typed global. Record it
as explicit derived source, not byte-equal old source. Prepare fresh matched JIT
measurements of the same derived source/entry/state adapter whenever this route
is selected, preserving original JIT rows. A later comparison must state whether
it uses an original JIT implementation or an exact matched derived implementation.
All loop/recurrence/map/division/IO function bodies and sequencing stay original;
typed bindings and fresh initialization are independently counted app glue.

The compiled C ABI entry copies actual runtime arguments, then calls
initialize_runtime_state(), then run_original_driver(). Image building imports
definitions only and invokes neither entry nor state/driver functions. Type-only
precompile statements are separate from bounded trace processes; those processes
never produce the final image. Inspect serialized globals before final image
acceptance: required state globals must be unassigned, with no open streams,
Regex handles, input bytes, warmed arrays/random state or generated answers.

Concrete nbody adapter after the unchanged Body type/algorithm definitions:

```julia
global bodies::Vector{Body}
function initialize_runtime_state()
    global bodies = make_initial_bodies()
    push!(bodies, init_sun(bodies))
end
```

make_initial_bodies() contains the exact original four-Body literal, extracted
with recorded source spans and ordinary runtime construction. Its constructor
must execute on every fresh native process, not serialize initialized bodies or
reuse a precomputed sun. Driver preserves the two energy prints around nbody!.
The const-to-typed binding change requires matched derived JIT comparison.

Concrete FASTA adapter declares uninitialized state::Base.RefValue{Int32},
iub2/homosapiens2::Vector{Float64}, and n::Int. Original b"..." literals have
read-only Base.CodeUnits{UInt8,String} type, not Vector{UInt8}; preserve those
immutable symbol constants and the original immutable alu string. The official
[byte literal reference](https://docs.julialang.org/en/v1/manual/strings/#Byte-Array-Literals)
documents that distinction. Initialization constructs the exact original mutable
probability arrays and Ref(Int32(42)); driver
assigns global n from ARGS before perf_fasta(n), since perf_fasta(_n) reads global
n. Immutable scalar/string literal data may remain code constants, but no
generated sequence or mutable initialized random state enters the image. Do not
introduce a byte-vector conversion or guessed type into the original symbol data.

Concrete revcomp adapter declares INIO::IO, OUTIO::IO and COMPS::Vector{UInt8}
uninitialized, assigns actual runtime stdin/stdout, allocates the original256
table and executes the original zip/fill loop before perf_revcomp(). Abstract IO
binding is an explicit source/performance change; actual stdio subtypes and
native coverage must be observed with regular-file stdin/stdout. No guessed
stream type or build-time pointer/handle is admitted.

Concrete regex adapter declares variants/subs with each original collection
shape (vector for source1, tuple for source3), then constructs them before the
runtime driver. Original r"..." macro expansion itself must be inspected: a
quoted preconstructed Regex object is not fresh runtime regex initialization.
If needed, derive explicit Regex(raw"...") calls with exactly the original
pattern/flags, preserving all nine patterns and five substitutions. Tagged
[Julia1.13.1 regex implementation](https://github.com/JuliaLang/julia/blob/v1.13.1/base/regex.jl)
constructs the r literal object during macro expansion and its default Regex
constructor uses the same default compile/match options. The explicit constructor
must therefore replace every selected literal, including the input-cleaning
expression inside perf_regex_dna, so those handles are constructed at runtime.
Raw literal
pattern strings are data; PCRE2 pattern compilation/JIT occurs normally at
runtime and is disclosed. The PCRE.exec method override remains exact compiled
source and is isolated in its own app.

Pi4 separately declares/assigns runtime n because its trailing-line label uses
that global. K maps, Pi BigInts, permutations, Mandelbrot coordinates/output and
spectral working arrays are already allocated inside original functions and
remain there. Trees need actual unchanged scored allocation evidence first; any
leaf correction gets a distinct source revision and matched new JIT measurement.
The distributed tree definitions need a reviewed image-safe definition route
that preserves its actual no-added-worker launch; runtime eval is not an adapter.

Before accepting any derived packet, bind its complete original/derived/glue
compiled input inventory and span-by-span adaptation review, exact native method
coverage including initializer/error/worker/IO paths, serialized-state inspection,
fresh-process output/work proofs and matched source provenance. Any unresolved
adapter or missing native coverage leaves that form unqualified; full Julia AOT
completion still requires every canonical workload/form.
