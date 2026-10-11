# pidigits: source spigot and general arbitrary-precision arithmetic

The complete compact `pidigits.mec` is 39 LOC. `pidigits-gated.mec` is 41 LOC;
it adds an eligibility comparison before the extraction tests. The 42 LOC
`pidigits-advance.mec` also advances the state before each candidate test, using
the ordering in C and Chapel to avoid extra tests immediately after output.
These programs
contain the spigot loop, both digit extractions, production and consumption state
updates, digit output and partial-line padding in Mech. `Extract`, `Produce`,
`Consume` and `Step` are ordinary Mech functions compiled to native code. There
is no host pi generator or pidigits executor.

Both forms follow the [official workload](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/pidigits.html):
the sequential unbounded spigot algorithm, using both extract(3) and extract(4)
tests, with ten digits per output line and the running count. The gated form
uses the eligibility condition also present in pinned Rust, C and Chapel
programs; whenever eligible it still performs both extractions. The full
workload is 10,000 digits. Complete small and full output hashes are pinned in
`input-pins.json`.

The needed library gap was signed arbitrary-precision arithmetic with reusable
storage, backed here by GMP through Rug. The source type is `<bigint>`, a checked
handle to an entry-owned mutable integer cell. The operations are:

| Operation | General purpose |
| --- | --- |
| `bigint/new(value)` | Allocate an integer initialized from a u64 |
| `bigint/assign(dst, src)` | Copy the value while reusing destination capacity |
| `bigint/multiply-u64-into(dst, src, factor)` | Multiply by a scalar into reusable storage |
| `bigint/add-into(dst, left, right)` | Signed addition |
| `bigint/subtract-into(dst, left, right)` | Signed subtraction |
| `bigint/add-multiple-u64(dst, src, factor)` | Add a scalar multiple |
| `bigint/subtract-multiple-u64(dst, src, factor)` | Subtract a scalar multiple |
| `bigint/floor-u64(numerator, denominator)` | Floor division with a checked u64 result |
| `bigint/less-equal(left, right)` | Compare integer values |

These operations have ordinary mathematical aliasing semantics; a destination
can also be a source. Division rejects zero denominators and results that do
not fit u64. The compiler tracks mutation effects and the entry-owned arithmetic
resource, preserving call order and rejecting use from unsupported independent
workers. Handles are typed rather than interchangeable with ordinary scalar
integers. Storage is retired on normal completion and errors. No arbitrary
precision session is initialized for an older control program that does not
use these operations.

Reusable cells retain GMP limb capacity, and the add/subtract-multiple calls
reuse the same general GMP techniques used by peers. The optional policy
`MECH_BIGINT_POWER2_SHIFTS=1` replaces multiplication by powers of two with shifts;
it is measured separately and is not enabled by default without evidence of a
benefit. General floor division writes directly into the reusable quotient,
avoiding the former copy and GMP's numerator-aliasing temporary path. When
operand signs match, floor division uses quotient-only truncation, which gives
the identical result without the remainder computed internally by GMP 6.3's
general floor routine. Opposite signs retain floor rounding; checked output
conversion is preserved. This decision uses actual integer signs, not a program
or input identity. Existing record and match lowering from binarytrees reduces source
overhead here as well. Literal-only `io/print` now supports padding directly.

The library is admitted through the experimental Cranelift AOT frontend
backported onto the v0.4 integration. Interpreter and distributed support are
outside this implementation's current scope.

`remaining_conformance.py` checks all three complete programs at 0, 1, 2, 9, 10,
11, 27, 30, 97, 1000 and 10000 digits. It also compiles a source Fibonacci loop
that exceeds 100 bits and reuses integer cells, demonstrating a separate use of
the same APIs. Runtime tests cover signed results, destination/source aliases,
division errors, invalid handles and resource retirement. The standalone
`native-features` test crate exercises real compiler admission, effect inference
and worker rejection without expanding unrelated broad engine test fixtures.

Measurements and LOC follow the regex-redux protocol: matched full workloads,
AC normal power, sequential fresh processes, complete output checks and source
LOC only. All eligible listed native-cohort programs are attempted; compact
runtime candidates include the listed smallest and published-fastest complete
programs. Missing or incompatible peer libraries are explicit exclusions, not
rewritten benchmark algorithms. A library-backed peer does not establish a
smallest endpoint if it cannot pass the output check on the configured runtime.

## Reproduction

```sh
PYTHONPATH=/tmp/mech-clbg-compact-libs python3 benchmarks/clbg/run_remaining.py pidigits --prepare-only
PYTHONPATH=/tmp/mech-clbg-compact-libs python3 benchmarks/clbg/run_remaining.py pidigits --reuse-builds --samples 3
PYTHONPATH=/tmp/mech-clbg-compact-libs python3 benchmarks/clbg/remaining_conformance.py
PYTHONPATH=/tmp/mech-clbg-compact-libs python3 benchmarks/clbg/remaining_regressions.py
```

Compiler builds use the existing Homebrew GMP 6.3.0 through `system_gmp`.
The measured report pins sources, objects, executables, dependency locks,
toolchain versions and an exact compiler/runtime archive. Earlier tuple forms
and diagnostic trials remain under `experiments/` rather than being substituted
for the measured canonical forms.
