# Optional native LLVM Object backend

The experimental backend now emits and executes genuine Mech native Objects.
It is **optional**, not the default compiler or a required toolchain for the
existing Cranelift path. LLVM opt and llc run only during LLVM compilation.
The emitted program does not load LLVM or invoke these tools. The maintained
scalar native path and selected IR242 Cranelift checkpoint remain unchanged.

Source implementation: b638320000f7cb33837df2ee73f09221551826a0.
Evidence: benchmarks/related-post-gen/trials/llvm-native-image-20261107/.
The older typed-SSA packet remains historical: it did not execute an LLVM Mech
program and must not be relabeled as this later result.

## Implementation and invariants

`emit_program_llvm_object_with_limits` records each owned, verified CLIF function
before Cranelift changes its Context, lowers typed SSA to LLVM IR, optimizes it,
emits native AArch64 machine code, and derives actual machine facts before
making metadata. Source, definition, node, input and output limits remain
independently enforced. Actual LLVM code/constants and stack frames define
machine admission; discarded Cranelift estimates do not.

Lowering preserves integer, floating, Boolean, SIMD and call semantics. There
are no fast-math, no-wrap, inbounds, alias, or call-purity promises. Definitions
stay noinline; unsupported operations fail explicitly. Raw-word Booleans,
modulo shifts, overflow results, traps, NaNs/signed zero, pair returns and
preserving calls retain their typed semantics.

LLVM uses image10 and a fixed private backend/options/toolchain/source-IR/machine
digest. Cranelift retains image9, no backend field, and byte-identical default
bundles. A backend stamp grants no source authority: independent loaders derive
the source/layout proof afresh, including after a self-consistent forged stamp.
Header-only LLVM reconstruction declines; image and compiled-module constructors
retain the complete identity.

The compiler's Rust Mach-O reader checks the closed definition set and exact
StackSize reports. Native linking canonicalizes literals, derives the machine
footprint, then combines fresh metadata in a separate readonly section. Final
native text/constants and symbol extents are audited before returning an Object.
The executable is checked against that same Object. The existing root-plus-
largest-helper stack policy remains; this is not a new universal call-graph
stack analysis.

`generate_linked_llvm_program_application_with_limits` uses the ordinary checked
application writer with explicit LLVM selection and a runtime-only launch shell.
An actual generated minimal Cargo project builds and runs offline with its
generated main unchanged.

## Qualification

Both benchmark sources execute correctly, including 31 Rust edge cases each.
Original and decoded LLVM Objects have all 13 bundle files byte-identical, as do
both default Cranelift source bundles against the preceding compiler.

Maintained native tests cover 240 LLVM/JIT program comparisons, 132 loader checks,
actual exact/one-under machine limits, source limits, aliases, finite errors and
publication behavior. Independent, Object-bound and full-catalog loaders produce
192 exact outputs, 48 exact finite denials and 12 exact successes. Runtime-only
runners produce 64 exact outputs. Generated applications check pin refusal,
same-Object linking, 9 successes and 6 denials. The actual Cargo application adds
3 exact successes and an exact overflow denial.

All 399 names from the native CI selection execute and pass. Full GPU results:
898 passes and two inherited failures, in the legacy matrix-multiply catalog
and unchanged k-nucleotide owned-map tests. Four ignored LLVM execution tests
are explicitly run and pass. The full GPU suite is not entirely green. All 133
scalar/DEFLATE control rows retain their exact non-timing fields and allocation
measurements. Efficient scalar native execution is not retired or replaced.

## Measured comparison

Two predeclared, independently seeded 31-round cohorts include all 20 retained
Rust controls, explicit Apple M3 Rust builds, fixed IR242 and an IR264 Cranelift
control with the same frontend, source, adapter and runtime closure as LLVM.
Each cohort retains 775 exact outputs, including all first/adverse observations.
Seven separate resource rounds retain 182 exact outputs. Builds, tests, profiles
and archives do not overlap these cohorts.

| Confirmation cohort | Post-JSON | Times fastest Rust | Launch through exit | Times fastest Rust |
|---|---:|---:|---:|---:|
| Fixed selected IR242 |2.807875ms|1.83656x|36.303375ms|1.50629x|
| Matched IR264 Cranelift |2.801834ms|1.83261x|36.171125ms|1.50080x|
| Optional LLVM reuse4 |2.335208ms|1.52740x|36.138833ms|1.49946x|

Fastest references are separate: processing Rust1.528875ms; whole process
Rust24.101167ms. Continuous post-JSON includes all input preparation, admission,
worker creation/run/join and materialization. Whole process includes restoration,
JSON IO and destruction. These are local fresh-process measurements on a shared
Apple M3, not a claim of universal Rust optimality.

LLVM reuse4 wins all 31 paired processing rounds in each cohort. Its confirmation
paired median improvement versus fixed IR242 is16.23%; the predeclared other-
boundary noninferiority upper bound is+0.488%, below 1%. Whole-process superiority
is not established. This passes the processing-investment gate; it does not
establish Rust parity or make LLVM mandatory.

Processing instructions fall from 54.826M in matched Cranelift to 39.990M in LLVM.
Both request 3758 allocations/8,012,356B; the fastest processing Rust control
requests 58 allocations. Restoration remains about 125.9M instructions and 26837
allocations. LLVM reuse4 has 554700B of code/constants and a 1568B reported stack
bound. Resource measurements are separate from latency.

## Existing-backend investigation

LLVM is a reference for missed optimizations. Current register profiles locate
the main query and serial preprocessing as native hotspots. In the query,
Cranelift has repeated Boolean conversions, register moves and stack traffic;
LLVM emits fewer machine instructions. Static counts and register observations
are not dynamic instruction counts or CPU-time percentages.

Subsequent IR265 Cranelift Boolean-chain branch distribution is unselected:
401 native CI names and all 133 controls pass, but it loses all 31 processing
comparisons to its parent, adds about 6% processing instructions and regresses
whole-process time. Do not repeat this experiment unchanged or infer that adding
branches recovers LLVM's conditional-instruction combining.

Continue with direct backend instruction combining, an executed proof/funding
census, and backend-independent compact metadata restoration. Exclusive
instruction diagnostics identify about 51M instructions in owned outer-image
deserialization; reader initialization is negligible. Preserve eager independent
validation and measure first evaluation separately. Moving restoration outside
the timer or into first use is not an optimization.
