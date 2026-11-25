PolyBench/C native MINI regressions

The thirty Mech programs use the F64 MINI profile and matrix operators,
broadcasting, comprehensions, pattern matching and finite-state machines. The FSM ports specialize their closed
payload kinds to the upstream MINI dimensions. All thirty kernels are covered;
this fixture does not qualify larger datasets.

cases.json contains the C initialization inputs and all live-out values from its unmodified PolyBench/C 4.2.1 beta initializer and kernel. The C
adapters were checked independently against untouched upstream dump output.
The fixture records the upstream archive URL/SHA256, each C source path/SHA256,
the Mech source hashes, dataset, floating-point flags and canonical data layout.
The inputs and expected results were generated from C, not from Mech execution.
No C compiler, network connection or external benchmark checkout is needed to
run this regression. Expected values are finite; comparisons use rtol=1e-10
and atol=1e-9.

Matrices are canonical row-major snapshots. For doitgen and heat-3d, the first
two tensor axes are flattened into rows. C kernels use the supported
DATA_TYPE_IS_DOUBLE option, including
Deriche (whose default is F32) and the graph tables (whose default is integer).
Nucleotide bases remain byte-valued C inputs and use exactly representable
F64 cells in Mech. C identifiers containing underscores become Mech
names containing hyphens. The first trisolv row uses an explicit zero-prefix
case because the maintained range operation rejects an empty range. Deriche
expresses exp(x) using the maintained math/pow operation with Euler's constant;
its complete C outputs are checked with the same tolerance as the other kernels.

Run from the repository root:
cargo test -p mech-gpu --no-default-features --features canonical-native-program --test canonical_artifact_polybench

The test checks every live-out of original and decoded artifacts, emits their
Objects, preserves input aliases, verifies Mech source hashes and checks that
the shared managed-memory account is empty after each completed case. All
execution and library dimensions have explicit finite limits. The full-suite
work allowance is 10^15 conservative work units; matrix regression tests
separately retain tight admission-denial, rollback and recovery checks.

The polybench_qualification example accepts a source path, a JSON object of
named C initialization inputs, and an optional sample count. Matrices contain
shape:[rows,columns] and canonical row-major values; U64 parameters contain
u64:value, and scalar F64 inputs are JSON numbers. A zero sample count checks
one fresh native invocation without timing samples. A positive sample count
adds warmed, cache-flushed checked invocations. Parsing, compilation and input
import stay outside those intervals; resource checks and output creation stay
inside. The explicit resident-reference mode is a separate diagnostic path.
