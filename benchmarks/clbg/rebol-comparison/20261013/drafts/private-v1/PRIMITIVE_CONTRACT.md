# Proposed general native facade

This interface is authored application design, not an observed runtime export,
installed package or qualified extension. Its loader/native implementation is
now authored against the tagged3.22.1 entry signatures and full generated SDK
macros. Actual complete SDK generation/provenance, ABI, build and loader
qualification remain prerequisites before any preparation/run. No handwritten
partial RL_LIB/header or routine! FFI is assumed.

The two arithmetic/regex main programs are source-written against this explicit
contract. All ten programs use generic IO from the same extension. Each bundle
counts its Rebol facades, loader and full authored C extension, and discloses
both linked vendor libraries. The native implementation contains only general
operations; no workload state, constants, recurrence, scan orchestration,
counting, replacements or output assembly may be hidden behind them.

`clbg_big.reb` returns a context with these methods:

- `from-int(value)` serializes a native signed integer as a decimal binary.
- `to-int(decimal)` converts a proven small decimal result to a native integer.
- `add(a,b)`, `subtract(a,b)`, `multiply(a,b)` accept signed ASCII decimal
  binaries and return canonical signed decimal binaries with unbounded GMP
  arithmetic. No64-bit truncation or decimal float conversion is permitted.
  The accepted spelling is optional minus followed by one or more digits;
  plus signs, whitespace, embedded NUL and empty values are rejected.
- `floor-divide(a,b)` calls the general mathematical floor quotient on signed
  unbounded operands and rejects a zero divisor. This must use the actual full
  GMP numerator/divisor; no leading-digit estimate or Pi-specific shortcut.
- `compare(a,b)` returns -1,0,1 for general signed values.

Each arithmetic call owns local GMP values, parses complete binary inputs,
releases every value/output temporary on its ordinary success/failure paths,
and returns an owned Rebol binary through the proposed extension series ABI.
The result reserve is bounded from operand lengths and allocated before any
native decimal/GMP resource. Input views are reacquired after that allocation;
GMP values are cleared before filling the already reserved result. The fill
checks actual remaining capacity and never requests expanding VM allocation.
For every appended byte, it checks width1, tail==index and rest>=index+2,
including the terminator. Tagged
[Expand_Series](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/m-series.c)
allocates only when (tail+1+delta)*width exceeds rest*width; with delta1,
the checked append takes its existing-capacity branch. This is an explicit
source proof, not an assumption that RL_Set_Char cannot invoke expansion.
There are no persistent opaque handles or native Pi state to preserve.

`clbg_regex.reb` returns a context with `find(pattern,subject,start)`:

- Inputs are binary byte series and a1-based search offset in1..length+1.
- It compiles a generic PCRE2-8 expression with ordinary case-sensitive byte
  matching, explicit LF newline semantics and JIT disabled, finds the next complete match, and returns `none`
  or an owned two-integer Rebol block `[start,end]` with1-based inclusive start
  and exclusive end. Native offsets must be range/type checked before return.
- Pattern compile and runtime matching errors are failures, not absent matches.
  Native byte length and starting series index must both be respected.
- Every call frees its compiled expression and match data on each ordinary path;
  no handles, subject copy or native callback is retained after return. The
  subject is borrowed only while native matching runs, without any Rebol
  allocation/callback. Native resources are freed before RXR_BLOCK dispatch
  allocates the ordinary result pair.

Repeated pattern compilation is inside this proposed baseline's measured
boundary. It is an ordinary general operation, not a claim of optimization.
All matching/count loops and all five sequential replacement assemblies remain
in the authored Rebol main. Its selected expressions are nonempty; an empty
returned match stops the selected program instead of looping or fabricating a
count. Generic primitive fixtures must independently exercise empty/absent
matches, bounds, byte indices, errors, signed arithmetic and zero division.

The selected native vendor GMP and PCRE2 libraries, header/source provenance,
ABI, complete image closure and default allocator behavior need actual common
mutex receipts. No library build, parser, fixture or execution occurs here.

The same module supplies `read-stdin-chunk()` and `write-stdout(binary)`. The
counted Rebol IO facade collects each32768-byte-or-shorter native read until
EOF, enforcing the signed input bound before each append. Native reads reserve
their Rebol result before consuming any input and use only a fixed stack byte
buffer; the former native read-all heap buffer has been removed. EINTR and
short-read/write handling preserves exact byte lengths. Record parsing,
sequence selection, reversal, PBM packing and every output assembly remain
Rebol. Native writes borrow bytes without VM calls or native heap resources.

The tagged Make_Series cap is MAX_I32 and its OOM path traps nonlocally, rather
than returning NULL. The proposed guard is deliberately below that signed cap:
INT32_MAX-64 bytes for input/views/results, leaving terminator/alignment room.
Every result reserve is checked before invoking the VM allocator. VM allocation
occurs before native GMP/decimal ownership, or with only stack storage for IO
and formatting; PCRE2 resources are released before RXR_BLOCK's VM allocation.
No native heap ownership is carried across a requested VM allocation. These
source ordering/capacity facts require later actual ABI/failure receipts.
Allocation traps and fatal vendor OOM remain failed fresh-process outcomes;
they are not claimed as caught errors or fully recovered native executions.
No successful qualification may infer cleanup from the source proposal alone.

`format-fixed(decimal,digits)` calls the general C library `%.*f` formatter,
returns the exact resulting ASCII binary and accepts finite doubles with
precision0..18. It does no scaled rounding or benchmark arithmetic. The future
child environment must bind LC_ALL=C and normal FPU rounding, with actual
0.0625/3,2.675/2,negative values and full9-digit output fixtures. This replaces
the incorrect initial half-away scaled formatter before any native run.
Its bounded512-byte text buffer is on the stack across VM result allocation,
and truncated formatting is an explicit failure; no malloc text survives a
nonlocal VM trap.

The loader requires an absolute child-local extension path and verifies the
returned module exposes all nine typed commands. RX_Init checks alignment and
exact3.22.1 through the generated library table. This is a proposed guard,
not an observed ABI check. Root must generate/pin the exact complete headers and
their generator/input dependencies under common before compiling anything.

The general quotient choice follows the official
[GMP integer division API](https://gmplib.org/manual/Integer-Division): fdiv
selects the mathematical floor quotient. Matching/free/offset operations follow
the official [PCRE2 API](https://www.pcre.org/current/doc/html/pcre2api.html).
The exact Rebol entry and series ABI sources are linked in the parent
CAPABILITIES.md. These documentary references do not replace the actual
selected vendor headers, runtime images and primitive receipts.
