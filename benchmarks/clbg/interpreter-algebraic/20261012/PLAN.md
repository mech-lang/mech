This additive experiment is unbuilt and unqualified. It changes no runtime,
published source, compiler minimum, control binary, live owner, input pin,
report or score. All builds, source/pin checks, semantic probes, assembly/IR
inspection and timing must own `/tmp/mech-clbg-timing.lock`, with inherited
ownership through each compiler/sampler/native process group and full cleanup.

The [Rust 1.98 release](https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/)
stabilizes explicit algebraic floating operations. They permit reassociation
and other optimizations, potentially enabling loop vectorization. The
[official API](https://doc.rust-lang.org/stable/core/primitive.f32.html#algebraic-operators)
does not guarantee precision, deterministic results, signed-zero behavior or
normal NaN/infinity behavior. Finite-input checks alone therefore cannot turn
these methods into a strict-equivalent optimization. Whole-array workload,
output and numeric qualification remain mandatory, and existing scores cannot
be reused for changed arithmetic.

Local inspection found an existing route that avoids a compiler upgrade:
`~/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/num/f64.rs`
already declares `algebraic_add/sub/mul/div` behind `float_algebraic`. The
workspace's root/engine/core/gpu manifests declare Rust 1.92 and edition 2024;
CI and the sealed source-mode compiler use that exact nightly. The experiment
will use its pinned compiler and add `#![feature(float_algebraic)]` only to an
isolated copied GPU crate. Production Cargo files and `.cargo/config.toml`
remain unchanged, including warnings-as-errors. Stable 1.98 comparison is an
optional separate compiler experiment, not needed for this first test.

The concrete copies retain the existing generic interpreter in
`hosts/gpu/src/sequence/resident.rs`. `ScalarTape` interprets typed source
coefficients; `ScalarTile` evaluates each source instruction over at most 32
independent calls after its existing domain proof. No coefficient formula,
benchmark name, host workload kernel, extra pruning or JIT code is introduced.
The existing transpose, tail/domain checks, checked integer fallback, Rayon
partitions, eight-worker policy, source callables, allocation and output path
remain intact.

| Copy | Change to `ScalarTile::row` and `scalar_row` |
| --- | --- |
| `resident_strict.rs` | Exact unchanged source copy, positive-zero seed and original left-to-right weighted reduction. |
| `resident_reduction.rs` | Only accumulation becomes `sum.algebraic_add(term)`; multiplication, division and coefficient DAG stay strict. |
| `resident_broad.rs` | Accumulation plus weighted `x.algebraic_mul(coefficient)` / `x.algebraic_div(coefficient)`; coefficient DAG remains strict. |

These are the first two experimental variants requested for the actual
generic machinery. The separate `prototype.rs` shows reusable slice helpers
and explicit monomorphized policies for tiny conformance/IR experiments. It
is not an integrated spectral-norm executor. Its caller supplies the original
seed: local core `iter/traits/accum.rs` uses negative zero for floating `Sum`,
whereas these resident rows use positive zero. Strict fixtures must compare
bits with the correct seed, including empty arrays.

Other local opportunities are dense engine `numeric/mod.rs` sum_rows/columns
(12572/12602), matrix_multiply (16968), dot (17329), and snapshot dot (19608),
which currently use strict sums or scalar accumulators. The same file's
binary_f64 (11316) already exposes independent elementwise/broadcast slices.
Fused f64 resident scalar dispatch (83) and paired wide::f64x2 operations
(127) execute Nbody/Mandel graphs; separate register writes inhibit whole-DAG
reassociation, so per-op changes there have weaker grounds than reductions.
Complex/rational/integer kernels, shape/index/resource checks and bitwise
change detection are outside this first experiment.

Preparation will snapshot/pin all dependency sources and the existing compiler
and standard/shared libraries under the mutex. Each variant gets its own
copied crate/workspace and CARGO_TARGET_DIR, identical profile/CPU/worker flags,
and a fresh measured executable. Nothing may overwrite frozen inputs. Before
timing, compare the exact source diffs and compile the three copies with the
same pinned nightly. Keep LLVM IR and ARM assembly for the actual row loop:
look for relaxed fadd only in reduction-only, retained strict fmul/fdiv there,
relaxed weighted operations only in broad, and real vector accumulation or
reciprocal changes. Lack of relevant code-generation gain is a stopping result,
not a speed claim.

`guarded_prepare.py` is the unexecuted concrete preparation draft. It first
seals original compiler/helper/SDK/library/source pins and all experiment files
under the mutex. It borrows the unchanged, already qualified ownership/probe
functions from frozen Nim v2 and image-inspection functions from frozen Pony
v1; it never changes their globals or inputs. A later exact-digest `--prepare`
creates three separate copied workspaces and targets. Each runs the original
integration driver release/offline/locked build, retaining optimized bitcode
with `-C save-temps`; pinned LLVM 22 `llvm-dis` converts that actual bitcode,
and `llvm-objdump` records ARM disassembly from the exact production binary.
This preserves Cargo's required emission modes and uses identical codegen
settings in all variants.

A separate opt-in `algebraic-conformance` feature/binary calls the actual
private tape/row code with constant and varying arbitrary coefficient graphs.
The unscored fixture records all numeric classifications and verifies original
strict bits, axes, 32-lane tails, checked-domain fallback and overflow errors.
It changes no executed production CLI path. That separate feature build must
leave the already pinned production executable and original IR unchanged.
Then eleven real unchanged-source Mech cases (N1/2/3/7/8/9/31/32/33/65/129)
run under the existing sampler/native PGID/resource/status/pin guards with
complete independent Python reference output. No native queue has launched.

The bounded conformance matrix must include zero/one length, 31/32/33 tile
tails, non-power-of-two lengths, transpose and non-square layouts, positive
inputs, mixed signs, catastrophic cancellation, signed zeros, subnormals,
overflow/underflow, division-by-zero, NaN and infinities. Strict copies must
retain source results and errors exactly. Algebraic results are classified
with bits, finite/nonfinite status, absolute/relative error and ULP distance;
unexpected values are retained, not disguised as strict conformance. The
unchanged source coefficient tape must still execute every admitted call and
retain all integer errors/tails. Repeated identical algebraic calls may differ
and must not be used as a soundness premise.

Then run actual unchanged Mech spectral-norm sources through the copied
resident interpreter at small/adversarial shapes with complete output and
independent work/count proof. Only a demonstrated gain with accepted canonical
output justifies actual full N5500 processes, using unchanged eight workers and
three fresh samples per selected variant plus matched strict controls. Keep
whole-process and prepared execution measures distinct and preserve every
failed attempt. No extrapolation, existing-mode timing aliases or chart
admission is authorized by this plan. Nbody/Mandel deserve separate exact
energy/trajectory/escape/PBM gates before any later broader experiment.
