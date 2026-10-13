# Direct library initialization route review

Source-only review of PackageCompiler2.4.3 [julia_init.c](https://github.com/JuliaLang/PackageCompiler.jl/blob/v2.4.3/src/julia_init.c)
and its header. The implementation forwards arguments through libuv and Julia's
option parser, finds the compiled program library, resolves its absolute path,
and initializes Julia using that image. Julia1.13 follows its branch with an
explicit bundle bin directory. Shutdown invokes Julia's exit hook. This C route
contains no Julia source evaluation; direct_driver.c supplies the five fixed
runtime options and then calls the exported native entry.

Initialization also appends bundle paths to inherited depot/load paths. The
future guard must sanitize inherited Julia paths and pin the generated library
name, compiled init object, bundle layout and actual resolved runtime/images.
Source intent cannot prove options survived initialization, initializer methods
were native, or hidden package/runtime work avoided fallback. Observe the exact
unchanged runtime before initialization through shutdown, including all threads,
and retain actual options/image/entry evidence and negative-canary rejection.

The header supports the declared direct ABI, conditional on actual exported
symbols and correct linked generated artifacts. An ordinary Julia fallback
launcher, default application eval wrapper or unreviewed alternative image is
not admitted. Binarytrees4's Main-targeting @everywhere module-context issue
remains separately unresolved; this init route does not repair its definitions.
No source-generation/build/native queue is authorized by this static review.

Tagged Julia1.13.1 bootstrap is [init.c](https://github.com/JuliaLang/julia/blob/v1.13.1/src/init.c#L543):
jl_init_ calls _finish_jl_init_. The supplied-image branch restores the image;
boot.jl loading belongs to the separate no-image branch. After runtime/thread/GC
setup, saved module initialization order is traversed. Each initializer is invoked
by [toplevel.c](https://github.com/JuliaLang/julia/blob/v1.13.1/src/toplevel.c#L54),
which resolves __init__ and applies it at the current world. Thus module startup
still executes Julia methods and needs actual native coverage/observation. The C
entry's lack of an eval wrapper does not qualify nested initializer behavior.
