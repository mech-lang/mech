# Native LLVM Object backend work

Status: implementation has started; no LLVM program is qualified or selected.
The selected compiler remains IR242. The working branch starts from the qualified,
unselected IR264 compiler, which includes the latest fetched production DEFLATE
compiler code. The newer upstream commit changes experiment/design records only.

The current post-JSON all-thread profile observes native preprocessing and query
code most frequently, with repeated allowance/Boolean checks and stack traffic.
Those are statistical register observations, not CPU-time shares. The stronger
Rust control reaches1.526792ms while unchanged selected Mech measures2.715875ms
in the same cohort (1.778811x). Mech retires55,012,810 processing instructions
versus17,876,348 (3.077x). This motivates a broader machine-backend improvement.

The old LLVM diagnostic is not a production starting point: its descriptor does
not describe LLVM's code, stack or backend identity, and it lacks complete scalar
and finite-resource qualification. Do not substitute its text under a Cranelift
Object or quote its historical speedup as a current qualified improvement.

## Implemented foundation

Object metadata finalization is separate from the native definition pass. Its
same-Object identity, image, table metadata and compiled-module exports are made
only after code/stack facts are supplied. The default Cranelift path emits all13
bundle files byte-for-byte identically for both benchmark sources, including the
entire Object and descriptor. No IR/ABI/work version or runtime algorithm changes.

`llvm_backend/machine_facts.py` is a compile-time metadata reader. It reads actual
AArch64 Mach-O text/constants and LLVM prolog-epilog StackSize records, including
zero-byte frames. The sealed compiler definition set must match both symbols and
stack reports exactly. Missing/repeated reports, unexpected functions, aliases,
unknown allocated sections, wrong target/subtype and malformed extents decline.
Per-function code extents conservatively include following alignment padding;
total code includes machine constants and reports unwind/link metadata separately.
The reader grants no source/layout authority. The real probe has0/0/544-byte
frames, and malformed-report cases are refused.

## Remaining implementation and acceptance

1. Record the complete sealed CLIF definition graph and module declarations,
   using an owned compilation scope. Preserve source preparation and all source
   bounds, errors, counters, COW, aliases and publication/retry effects.
2. Lower Bool/U64/I64/F64 and used vector operations correctly. No fast-math,
   integer no-wrap/inbounds promises or pure-call attributes may be invented.
   Preserve raw Boolean words, shift masking, float NaNs/signed zero, checked
   conversions and cold error edges. Unsupported translation fails closed.
3. Compile machine code before metadata binding. Derive actual code/constants,
   per-function frames and the existing root-plus-largest-helper stack policy.
   Enforce original machine limits on LLVM output, not on a discarded Cranelift
   estimate. Keep the source/template/definition limits independently enforced.
4. Give the selected backend, exact toolchain and target/options their own
   fingerprint in the descriptor/image identity. Update wire version/readers as
   needed. Independent loaders rederive source proof; a backend label grants none.
5. Assemble genuine data and code into one Object; verify the closed symbol set,
   final text/constants and footprint after assembly. Never keep an old descriptor
   under replaced code. Keep compile-time tools out of executable runtime paths.
6. Qualify ordinary/native/original/decoded/restored execution, all scalar kinds,
   source errors, every finite boundary, output publication/recovery, loader and
   old-Object/forgery refusals, scalar-only/runtime-only/generated applications,
   Rust edge cases and all scalar/DEFLATE control fields. Preserve the maintained
   efficient scalar entry; do not retire it or route primitives through a runtime.
7. Only then run a quiet randomized comparison against the fastest eligible Rust
   controls at both continuous post-JSON and fresh launch-through-exit boundaries,
   with separate instructions/cycles/allocation/heap/RSS/size measurements. Both
   scopes and full adoption qualification remain necessary for selection/parity.

The compiler's frozen source inventory must include any embedded backend scripts
before a compiler embeds/executes them. The current machine-facts reader is not
yet invoked by production compilation. Its presence alone is not an LLVM backend.
