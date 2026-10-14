# Saved Julia archive members and official link route

This concrete three-file packet is `inspect.py`, `direct_driver.c` and this
note. It is source-only, mutable, unexecuted and unhashed. After root/peer full
source review, root can freeze/capture its three source digests plus the actual
three fixed LLVM22 tool-image digests, then launch one fresh archive observation
job. The worker executes only six bounded read/extract commands against the
two existing archives. It never launches Julia, compiles, links, executes the
workload, uses LLDB, reruns fixtures or writes the chart. A concrete next link
and image observation argv is saved as a proposal for separate root review.

The actual completed metadata seal is `jobs/official-static-io-safe-build-seal-v1/`
with result90b03c66, parsed options7ce0e355, outcome08484dd6,
durable7d910d43 and requestfd111d09. Root's 9,839 resolved-pin review is
`continuation/jobs/julia-saved-safe-build-seal-closed-review-v1/receipt.json`
`1ebe64084ffe2996ad4be7e0f5814d200bb61721a4a2bcae0cd3ab9ca9c19d84`.
Worker55243/target55249 normal0 and their exact request/start/terminal command,
cwd, identity and all saved byte pins are required. The frozen seal4eafdd9b
and its original build helper are cold-loaded only for constants and pure
load/read/bind/verify/save; neither seal/build/run/main is called.

The successful compiler53013 normal0 is retained, never repeated. Its object
archive is `nbody-safe-o.a` e25e9ace,9,444,136 bytes; optimized bitcode archive
is `nbody-safe.bc` fbf05e04,9,672,560 bytes. Both actual headers are ar, despite
the second file's suffix. Exact15 typed options and sole `clbg_entry` C ABI
have passed; native owner, actual IR removal, fallback closure and runtime AOT
remain unqualified. Old native success, both failed controllers, raw Dict
options and metadata outputs stay intact.

The exact new commands use caller-pinned, resolved LLVM22 `llvm-ar`, `llvm-nm`
and `llvm-readobj` from the three fixed Homebrew paths. They are observations
of existing native output, not another LLVM compilation of the Julia kernel.
For each archive, `llvm-ar t` lists all member names, then `llvm-ar --output=... x`
extracts into its own fresh directory. The original archive is read-only in
this route. Member names must be the official producer's contiguous text
partition family followed by metadata and sysimg; duplicates, additions,
missing members, mismatched partition counts, paths and output links reject.
The actual partition count is discovered, not assumed from the requested
thread count. Source basis is Julia v1.13.1 `src/aotcompile.cpp`'s
WRITE_ARCHIVE/getDefaultForHost implementation, which writes Darwin archives
for both object and optimized bitcode outputs.

Every object member must have an actual little-endian arm64 Mach-O OBJECT
header; every bitcode member must have the raw bitcode or LLVM wrapper magic.
All member bytes/headers/SHAs remain evidence. No bitcode reader or decoder
is invoked, and recognizable bitcode container magic is not IR-removal proof.
`llvm-nm --format=just-symbols --defined-only --extern-only` must observe exactly
one defined `_clbg_entry`. `llvm-readobj --file-header --symbols` retains all
actual native member headers and symbol records for root review. No automatic
whole-kernel/native owner verdict is derived from finding the entry name.

The minimal new C glue calls only the exact recorded signed32 argc/result and
pointer-to-pointer argv ABI. It excludes the executable name, preserving the
existing Julia entry's ARGS boundary. It adds no PackageCompiler
init_julia/shutdown_julia, Julia source evaluation, package loading or workload
implementation. The shipped official trimming `capplication.c`75a32483 calls
its exported library functions directly, providing this route's source basis.
The new driver is an explicit new counted authored source for any future
qualified application bundle. The old PackageCompiler driver's byte identity
is preserved as historical evidence; it is not silently treated as this glue.

The shipped Makefile26c1f1f1 uses whole-archive loading, julia-config flags,
libm and libjulia-internal. Its installed `share/julia/Make.inc` is absent, so
the worker records a direct Darwin equivalent instead of invoking an incomplete
make template. The independently retained `julia-config.jl`b167885d gives
gnu11/PIC/include, public/private library search, both rpaths and libjulia;
the Makefile supplies libm/libjulia-internal. The whole-archive choice uses
Darwin force_load for this one explicit archive, following Julia's v1.13.1
Make.inc Darwin whole_archive definition. Native archive contents and the
actual linker result still require review; this source statement is not a
successful initialization or executable behavior claim.

The concrete next-link proposal uses the already pinned Clang22 image and
fresh `official-static-io-safe-link-v1/nbody-safe`, this exact driver and the
original e25e object archive, with no compiler flags that request Julia JIT or
another Julia build. It records subsequent lipo architecture, otool linked
images/load commands and native defined-symbol observations. Root must first
review the actual archive output, then authorize/freeze a separate link/image
worker with these actual archive/owner pins and the image tools' exact bytes.
Those commands are not reachable in this archive worker. A link success and
image map alone will still leave runtime output, negative/missing-native,
fallback/IR closure and AOT timing admission pending. No absence-of-codegen
or no-fallback claim is made here.

The common inode79859134 plus completed seal/compiler private leases and a
new packet owner lease cover prerequisite reads, all six commands and final
byte reconciliation. Commands use the unchanged owned servicef1290ab1 with
its fixed cwd/DEVNULL/new-PGID/timeout/cleanup and inherited common FD. Fresh
exclusive outputs/builds reject retries/overwrites;600s alarm begins after
common acquisition. Genuine partial outputs and failed outcomes remain pinned.

Root's eventual bundled Python3.12 `-I -S -B -u inspect.py` arguments are:

```text
--revision official-static-io-safe-archives-v1
--worker-sha256 <actual frozen inspect.py SHA>
--driver-sha256 <actual frozen direct_driver.c SHA>
--review-sha256 <actual frozen REVIEW.md SHA>
--ar-sha256 <actual /opt/homebrew/opt/llvm@22/bin/llvm-ar image SHA>
--nm-sha256 <actual /opt/homebrew/opt/llvm@22/bin/llvm-nm image SHA>
--readobj-sha256 <actual /opt/homebrew/opt/llvm@22/bin/llvm-readobj image SHA>
```

Tool/source digests and successful archive observations are not invented by
this note. No execution is claimed; existing build and seal are the only
actual completed stages referenced above.
