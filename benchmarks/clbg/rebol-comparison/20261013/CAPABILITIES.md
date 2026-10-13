# Tagged Rebol3 source capabilities

Source-only documentary review for the proposed Oldes/Rebol3 3.22.1 Bulk
macOS ARM64 interpreter. These links identify upstream source text inspected
through the web tool; there is no retained downloaded runtime/archive, local
version/architecture/library receipt or executed API probe yet. The source-port
owner must keep qualification fields false and preserve one faithful baseline
with explicit equivalent compact/optimized aliases until actual results exist.

The [publisher asset page](https://github.com/Oldes/Rebol3/releases/expanded_assets/3.22.1)
lists `rebol3-bulk-macos-arm64.gz`, publisher SHA256
`6f078cfba45dcfc43502fee6795361180ce0f1ed6964521278a488613408311a`.
The [tagged README](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/README.md)
identifies an interpreter and explains extension registration. Current master
build metadata advertises a later development version; the concrete published
candidate and all ABI research here use the exact 3.22.1 tag. No development
source or old REBOL2 SDK may stand in for its runtime evidence.

## Numeric and collection primitives

The [tagged datatype help](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/boot/typespec.reb)
describes integer as64-bit, decimal as IEEE64-bit, binary as bytes, maps as hash
associative pairs and user functions as interpreted. The
[integer implementation](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/t-integer.c)
uses signed64-bit values. All18-base two-bit K keys fit36bits; unbounded Pi
state does not. RSA's internal bigint is not evidence of a public arbitrary
integer datatype. Use only reviewed general GMP operations or counted pure
Rebol limb arithmetic for Pi, with both actual quotient extractions in Rebol.

The maintainer's [3.22 release discussion](https://github.com/Oldes/Rebol3/discussions/179)
documents `integer-divide`, `//` integer division, `%` remainder and `%%` modulo.
Old REBOL2 examples using `//` for remainder must not be copied. Exact signed
division/zero/overflow behavior remains a bounded native primitive requirement.

[n-math.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/n-math.c)
has general square-root operations using C double `sqrt`,64-bit shifts, and
distinct truncated/Euclidean/floor modulus. `square-root` has a general numeric
conversion path; `sqrt` is a conditional decimal-only native, so the scored
source must bind the actual selected build/API. Keep the source-written
Nbody/Spectral/Mandelbrot/FASTA floating work and loop order explicit; generic
native arithmetic does not authorize a host benchmark kernel.

[t-map.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/t-map.c)
is the real growing associative-map implementation. K must create and update
seven separate maps through every window and record actual table/capacity work
later. [t-block.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/t-block.c)
`CT_Block` mode3 compares series pointer and index. The native `same?` calls
that identity mode via `Compare_Values` in n-math.c. A fresh `make block!` before
the depth branch, including leaves, provides a concrete tree identity strategy;
actual repeated-call freshness, two children and retained graph still need
runtime/allocation proof. A source literal returned from a function may be
shared and is unsuitable as an assumed fresh leaf.

## Byte IO and runtime resources

[p-console.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/p-console.c)
returns binary bytes by default from console reads, unless string/lines
refinements decode them. Its line mode removes a trailing LF, and a lone ESC
byte has special handling. It sends write values through `Prin_Value`.
[POSIX dev-stdio.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/os/posix/dev-stdio.c)
has explicit-length raw stdin reads and stdout writes; nonterminal input uses
the ordinary OS path. These implementation facts do not yet verify a complete
high-level raw stdin/stdout command. A conservative source proposal may use a
counted general native extension `read-all-stdin`/`write-stdout` handling EINTR,
short reads/writes, exact byte lengths and failures. It must do IO only; Rebol
parses records, packs PBM bytes and drives every algorithm. Do not assume text
`print` emits an arbitrary binary value unchanged.

The source-port review subsequently found a concrete file-path trap:
[POSIX dev-file.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/os/posix/dev-file.c)
full reads of a size0 file call get_virtual_file_size(), which reopens the path
and reads it to EOF before the original read. Applying that to a stdin pipe
through `/dev/stdin` can consume the pipe during size discovery. The
[file actor](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/p-file.c)
uses this route; /binary alone does not fix it. POSIX file output also performs
one write, so short-write behavior needs separate proof. Prefer the counted
exact-byte read-all/write-all primitive for a minimal proposed full suite.
These are source findings, not an actual failing native receipt.

[m-gc.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/m-gc.c)
implements root/series marking and sweeps series, gobs and handles, with memory
pools. [a-lib.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/a-lib.c)
enables GC at initialization and exposes handle registration/context allocation
and free callbacks. [native specs](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/boot/natives.reb)
include `recycle` with on/off/ballast/pools and `release` for native handles.
Preserve normal automatic GC; no unmeasured GC-disable policy. General GMP and
PCRE2 resources need explicit release/ownership and reviewed GC fallback. Tree
temporaries may become unreachable after each check while the original long
root remains referenced. Actual runtime memory flags and resources need saved
receipts before any metadata is true.

## Native extension boundary

The exact [reb-ext.h](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/include/reb-ext.h)
defines packed16-byte RXIARG values and RXIFRM dispatch frames with up to15
arguments, including int64/double/series/handle fields and access macros.
[strings.reb](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/boot/strings.reb)
names the `RX_Init`, `RX_Quit` and `RX_Call` entry symbols.
[f-extension.c](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/f-extension.c)
loads the library, passes `Extension_Lib()` to initialization, imports the
returned Rebol module source and dispatches command indices through RXIFRM.
This is an ordinary reusable extension route; it is not the old routine! FFI.
The maintainer's [small Easing entry example](https://raw.githubusercontent.com/Oldes/Rebol-Easing/master/src/easing-rebol-extension.c)
illustrates `RX_Init(int, RL_LIB*)`, version/alignment checks and
`RX_Call(int, RXIFRM*, void*)`; that separate master example is explanatory,
never evidence of exact3.22.1 ABI compatibility.

The [exact tagged extension test](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/tests/extension/test.c)
provides all three entry signatures, module header/Exports/command declarations,
integer result dispatch and alignment checking. Use this exact-tag example
instead of relying on the separate Easing project's master ABI.

The generated RL_LIB headers are not checked into the guessed tagged
`src/include/reb-lib.h` path. The exact
[make-reb-lib.reb generator](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/make/tools/make-reb-lib.reb)
scans tagged a-lib.c followed by f-extension.c in order to emit the complete
RL_LIB function table, access macros, declarations and corresponding runtime
table. Its outputs are include/reb-lib.h and include/reb-lib-lib.h; it defines
RL_MAKE_BINARY as RL_MAKE_STRING(size,FALSE). The official
[pre-make step6](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/make/pre-make.r3)
calls it. Capture/pin that exact generation route and all dependencies later
under the common mutex. No manually guessed partial RL_LIB table or header
from another version is admissible.

The [tagged types-ext.reb](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/boot/types-ext.reb)
lists binary's extension type offset40, integer5 and block32. The
[make-boot generator](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/make/tools/make-boot.reb)
emits their RXT enum and exact datatype conversion tables. Capture the generated
header and independent type/ABI probes; source declarations do not verify a
runtime dispatch frame. No header generation ran in this review.

For a handle-free one-shot facade, binary arguments carry series+index; tagged
RL_Series retrieves data/tail/width and RL_Set_Char appends bytes while updating
tail. A new byte result can use RL_MAKE_BINARY and the generated type/return
constants. A native match may return none or a two-integer block, with tagged
RL_Make_Block/RL_Set_Value or an RXR_BLOCK frame. Copy input bytes before any
Rebol allocation that could move their storage; validate index/tail/width,
size limits and exact type constants. Decimal-binary GMP operations free all
native temporary values/decimal buffers in each command. PCRE2 match frees
code/match-data after obtaining generic offsets. Source-written Rebol retains
both Pi quotient calls and controls every regex scan/substitution. These are
reviewable interface choices, not executed ABI or ownership qualification.

Proposed reusable helper commands are GMP value construction/release/set,
add/subtract/multiply/compare, integer quotient and decimal conversion; PCRE2
compile/match/substitute/release; and exact byte IO if needed. No Pi recurrence,
FASTA RNG/CDF, K window/count/sort, regex-redux pattern selection or benchmark
kernel goes into C. All authored C, loader/module text, ABI declarations and
Rebol wrappers count with each using profile. Vendor library images are pinned
and disclosed separately. Actual extension loading, ABI/type/alignment, failure
and ownership probes are later reviewed common-mutex work.

The [tagged host argument parser](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/os/host-args.c)
supplies `-q`, version options, script/argument handling and security options.
Use a later actual receipt to select the exact noninteractive command; keep
interpreter startup, source/glue loading and IO inside timed fresh processes.
No current live documentation, quarantine/signing recommendation or global
permission change is an executed or approved runtime configuration here.

[b-init.c Init_Main_Args](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/b-init.c)
stores system/options/args as a block of strings. The
[tagged startup script](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/mezz/sys-start.reb)
normally forwards it to system/script/args, with a separate --args conversion.
It also uses REBOL_HOME for private data/modules and may load runtime-adjacent
rebol.reb and data/user.reb. Later qualification must isolate/bind these startup
inputs and the exact environment so no user module silently changes candidate
bindings. A private REBOL_HOME and exclusive runtime copy directory are a
source proposal, not a global HOME/security configuration change.
