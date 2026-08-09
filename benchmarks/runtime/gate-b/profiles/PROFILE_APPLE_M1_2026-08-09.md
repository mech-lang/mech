# Resident EKF turn profile and optimization: Apple M1

This profile isolates the one-instance, 4,096-turn Gate B resident EKF on an
Apple M1. Fixture construction, correctness validation, allocation reporting,
and Criterion analysis are excluded from the layer timings and flame graph.
All profiled steady-state lanes report zero allocations. The sampling profile
and first layer table describe the implementation before the follow-up
optimization below.

## Layer subtraction

The profiling-only `mech-resident-prepared` lane adds candidate summary and
state hashing to the scheduled turn, then publishes without preparing or
appending a retained receipt.

| Cumulative layer | Episode | Per turn | Increment |
| --- | ---: | ---: | ---: |
| Resident kernel | 0.5303 ms | 129.5 ns | 129.5 ns |
| Scheduled and published | 0.6702 ms | 163.6 ns | 34.1 ns |
| Summary/hash and published | 1.1300 ms | 275.9 ns | 112.3 ns |
| Complete receipt/ledger turn | 1.5069 ms | 367.9 ns | 92.0 ns |

The absolute times are slower than the earlier Gate B run because this profile
was collected under Time Profiler and a different thermal state. The
incremental attribution is the useful result: candidate summary/hash consumes
about 55% and receipt/ledger work about 45% of the 204 ns turn shell above
scheduled execution.

## Sampling profile

The Time Profiler recording sampled a sustained complete-turn run at 1 kHz.
The SVG keeps only the Criterion hot-turn closure and removes fixture setup and
post-timing validation stacks.

Prominent inclusive frames:

| Frame | Samples |
| --- | ---: |
| `prepare_scheduled_turn` | 56.8% |
| EKF scheduled kernel | 36.7% |
| `candidate_state_hash` | 14.0% |
| `prepare_commit` | 34.9% |
| `accepted_record` | 21.3% |
| ledger `prepare` | 6.6% |
| retained-ledger `append` | 5.2% |

Percentages are inclusive and therefore overlap. The capacity controller's
mutex-backed `bind` and `commit_prepared` transitions appear beneath ledger
prepare/append. They are real costs, but record construction/validation and
state hashing are larger targets.

## Finding

There is an obvious performance-mode boundary. A successful turn currently
hashes all 96 candidate bytes, constructs and validates a 64-byte receipt, and
appends that receipt through the retained ledger even though the computation
and scheduler have already completed. A fail-stop performance mode that omits
the diagnostic hash and per-turn retained receipt should approach the 164 ns
scheduled/published result from this run, below the measured 248 ns Julia EKF
turn.

Open [`resident-ekf-turn-apple-m1.svg`](resident-ekf-turn-apple-m1.svg) in a
browser for the interactive flame graph. Hover for inclusive sample counts,
click a frame to zoom, and use the search control to highlight functions.

## Follow-up optimization

The profile identified two assumptions that the frozen resident executor can
prove ahead of time:

- The state fingerprint is diagnostic, so it can mix the twelve `f64` bit
  patterns directly instead of running FNV over all 96 serialized bytes.
- The turn coordinator has one writer, a fixed receipt type, and a bounded
  admission window. It can reserve the entire receipt array during activation
  and append without the generic ledger's mutex-backed capacity controller.

The specialized recorder still prepares the receipt before publication,
aborts the candidate if preparation fails, retains every accepted or rejected
receipt, and performs no steady-state allocations. The generic retained ledger
is unchanged for concurrent and dynamically sized workloads.

Criterion measurements from the same Apple M1 after the change:

| Cumulative layer | Episode | Per turn | Increment |
| --- | ---: | ---: | ---: |
| Resident kernel | 0.5387 ms | 131.5 ns | 131.5 ns |
| Scheduled and published | 0.6696 ms | 163.5 ns | 32.0 ns |
| Summary/hash and published | 0.7181 ms | 175.3 ns | 11.8 ns |
| Complete receipt/ledger turn | 0.8027 ms | 196.0 ns | 20.7 ns |

Compared with the immediately preceding Criterion baseline, summary/hash
improved from 275.6 ns to 175.3 ns per turn (36.4%), and the complete retained
turn improved from 367.3 ns to 196.0 ns (46.6%). With 100,000 retained history
records the complete turn initially measured 206.6 ns. That gap came from
constructing history after the resident working set and thereby cooling the
working set immediately before timing. With the fixed receipt slots initialized
first, empty and 100,000-record history measured 196.0 ns and 195.9 ns
respectively.

## Follow-up sampling profile

A final Time Profiler recording captured 13,958 samples inside the optimized
Criterion hot-turn closure:

| Frame | Before | After |
| --- | ---: | ---: |
| Scheduled EKF kernel | 36.7% | 69.1% |
| `candidate_state_hash` | 14.0% | no distinct samples |
| `prepare_commit` | 34.9% | 6.9% |
| `prepare_accepted_append` | 32.0% | 1.0% |
| Mutex frames | present | none |

The after profile confirms the layer subtraction: the numerical kernel now
dominates the hot turn, the byte-wise hash is gone, and the specialized
single-writer receipt path no longer enters the generic capacity mutex.

![Annotated before and after resident EKF profile](resident-ekf-turn-before-after-apple-m1.svg)

## Same-run Rust-to-Mech gap accounting

The five exact one-instance lanes were rerun serially from the same optimized
binary with 60 Criterion samples per lane. Median episode estimates are divided
by 4,096 turns. This removes the thermal and harness mismatch between the raw
Rust timeline result and the earlier resident layer table.

| Cumulative layer | Episode | Per turn | Increment | Share of 80.63 ns gap |
| --- | ---: | ---: | ---: | ---: |
| Raw Rust EKF | 0.4676 ms | 114.16 ns | - | - |
| Resident candidate/evaluator/publish | 0.5309 ms | 129.62 ns | 15.46 ns | 19.2% |
| Scheduled dirty tracking | 0.6781 ms | 165.54 ns | 35.92 ns | 44.5% |
| Summary and state fingerprint | 0.7248 ms | 176.96 ns | 11.41 ns | 14.2% |
| Complete retained receipt | 0.7979 ms | 194.79 ns | 17.84 ns | 22.1% |

The cumulative subtraction has no unexplained residual: the four increments
sum to the measured 80.63 ns difference. The dominant cost is scheduler dirty
tracking: root seeding, per-node dirty checks, downstream propagation, execution
marks, and the dirty/executed node vectors. Summary hashing and receipt retention
together account for 29.25 ns, or 36.3% of the gap.

A fail-stop performance lane that bypasses dirty scheduling, summary hashing,
and receipt retention would target the 129.62 ns resident-candidate result:
7.71 MHz, 13.5% slower than raw Rust's 8.76 MHz. Keeping the scheduler while
dropping only hashing and receipts targets 165.54 ns, or 6.04 MHz.

Two increments are still composite and require narrower diagnostic lanes before
changing implementation:

- The first 15.46 ns combines activated-plan traversal and fifteen enum kernel
  dispatches with candidate epoch selection, double-buffer access, output-slot
  bookkeeping, and the release-store publication.
- The last 17.84 ns combines admission-permit consumption, fixed receipt
  construction, prepare-before-publish ordering, and the infallible receipt-slot
  append.

The raw Rust and resident lanes both return successful `Result` values and run
the integrity checks. This subtraction therefore does not identify the
successful `Result` check itself as a material standalone cost. The resident
prototype also uses enum dispatch rather than a virtual trait call, so virtual
dispatch is not hidden in this 80.63 ns result.

## Narrow diagnostic lanes

Three benchmark-only prototypes were run serially from one optimized binary
with the same 4,096-turn trace, 60 Criterion samples, three-second warmup, and
five-second measurement window. Every lane produced the reference trajectory,
performed zero timed allocations, and retained candidate validation.

| Lane | Episode median | Per turn | Rate | Relevant delta |
| --- | ---: | ---: | ---: | ---: |
| Raw Rust control | 0.4682 ms | 114.31 ns | 8.75 MHz | - |
| Resident candidate | 0.5317 ms | 129.81 ns | 7.70 MHz | - |
| Fused resident candidate | 0.4119 ms | 100.56 ns | 9.94 MHz | -29.25 ns vs candidate |
| Scheduled resident | 0.6709 ms | 163.80 ns | 6.10 MHz | - |
| Scheduled, scalar counts only | 0.6293 ms | 153.63 ns | 6.51 MHz | -10.17 ns vs scheduled |
| Prepared summary and fingerprint | 0.7156 ms | 174.69 ns | 5.72 MHz | - |
| Complete retained receipt | 0.7913 ms | 193.19 ns | 5.18 MHz | - |
| Complete, receipt prepared in slot | 0.7577 ms | 184.98 ns | 5.41 MHz | -8.21 ns vs complete |

The count-only lane preserves epoch marks and scalar touched, changed, dirty,
and executed totals, but does not populate the per-turn node and slot vectors.
Those vectors account for 10.17 ns per turn, or 29.9% of the measured scheduler
increment. Dirty checks and dependency propagation still cost about 23.82 ns
per turn after list materialization is removed.

The in-slot receipt prototype writes the accepted record into its reserved but
invisible ledger slot during preparation. Candidate publication still occurs
before advancing the visible ledger length. A contract test verifies that the
slot remains invisible before commit and contains the same receipt afterward.
This removes 8.21 ns per turn, 44.4% of the current receipt-layer increment.

The fused lane replaces activated-plan traversal and fifteen runtime enum
matches with fifteen explicit constant kernel calls. Candidate epoch selection,
the two state buffers, integrity validation, node execution marks, output-slot
tracking, and release-store publication remain. It saves 29.25 ns per turn and
reaches 100.56 ns per turn. This is also 12.0% faster than the existing raw Rust
control, but that control deliberately uses generic matrix helpers with runtime
dimensions; it is not a maximally specialized Rust implementation. The relevant
dispatch-fusion comparison is resident candidate versus fused resident candidate.

These results support three implementation directions, in descending order:

1. Compile dense activated regions into fixed straight-line kernel blocks.
2. Keep scalar scheduler totals by default and materialize node and slot lists
   only when an observer requests them.
3. Prepare retained receipts directly in reserved invisible slots, exposing them
   only after candidate publication.

## Fixed-shape cross-runtime controls

The earlier raw Rust control was portable and generic, so it was not a fair
lower bound for the fused resident candidate. A new control now calls the exact
same fixed EKF arithmetic as fused Mech, with the same two candidate buffers and
integrity validation, while omitting the Mech-specific shell. The
ordered 60-sample timeline independently warms every native lane for at least
250 ms before measurement.

| Lane | Per turn | Rate | Difference from fixed Rust |
| --- | ---: | ---: | ---: |
| Rust fixed fused | 69.47 ns | 14.40 MHz | - |
| Mech resident fused | 100.57 ns | 9.94 MHz | +31.10 ns |
| Rust generic matrix helpers | 112.17 ns | 8.92 MHz | +42.70 ns |
| Julia fixed StaticArrays | 130.53 ns | 7.66 MHz | +61.07 ns |
| Mech complete retained turn | 196.14 ns | 5.10 MHz | +126.67 ns |

This corrects the misleading observation that fused Mech beat raw Rust. It
beats the generic Rust helper control, but it is 44.8% slower than Rust around
the identical shared kernel. That 31.10 ns is the fused Mech shell above the
matched Rust work: epoch bookkeeping, reactive node/output marks, workspace
coordination, and atomic release publication. Fixed-shape Julia is 29.8%
slower than fused Mech.
Keeping the full Mech retained transaction adds another 95.57 ns over the
fused path and leaves it 50.3% slower than fixed-shape Julia.

The other runtimes also retain their original generic controls alongside new
preallocated fixed-shape controls. On this run, fixed-shape Julia improved
48.2%, LuaJIT 57.8%, Lua 36.6%, and pure Python 21.5%. The distinction matters:
these figures compare implementation strategies within each runtime, while
the fixed Rust versus fused Mech pair isolates the Mech shell around identical
arithmetic.

## Sustained fused-shell profile

The matched fixed Rust and fused Mech lanes were captured independently with
Apple Time Profiler at 1 kHz for ten sustained seconds. A benchmark-only mode
keeps one lane hot per process; this avoids the one-millisecond hot closure that
Criterion's profile mode produced for the custom-timed benchmark. Each corrected
recording contains about 10,200 hot-loop samples.

Distinct exclusive frames in the fused Mech recording:

| Frame | Samples | Share of fused Mech |
| --- | ---: | ---: |
| Fused candidate executor, including inlined arithmetic and validation | 7,938 | 77.9% |
| `atan2` | 927 | 9.1% |
| `begin_candidate` | 326 | 3.2% |
| `record_candidate_outputs` | 253 | 2.5% |
| `sincos` | 144 | 1.4% |
| Successful `Result` branch | 88 | 0.9% |
| Remaining fused-turn shell | 80 | 0.8% |

The inlined executor combines the common EKF arithmetic with node marks and the
post-kernel integrity scan, so two narrower constant-specialized prototypes were
measured in the ordinary optimized profile. Times below are medians from 60
Criterion samples with three seconds of warmup and five seconds of measurement.

| Lane | Episode | Per turn | Delta from fused Mech |
| --- | ---: | ---: | ---: |
| Fixed Rust, same buffering and validation | 0.29673 ms | 72.44 ns | -27.72 ns |
| Fused Mech | 0.41024 ms | 100.16 ns | - |
| Fused Mech, no internal node marks | 0.41042 ms | 100.20 ns | no measurable win |
| Fused Mech, no reactive output tracking | 0.39904 ms | 97.42 ns | -2.74 ns |
| Fused Mech fail-stop | 0.36401 ms | 88.87 ns | -11.29 ns |

The fail-stop lane keeps candidate double buffering, epoch selection, the
successful `Result` path, the two arithmetic-domain checks, and atomic release
publication. It omits reactive node/output tracking and the post-kernel scan for
finite values, positive covariance diagonal, and covariance symmetry. Correctness
is still checked after the timed episode. This is a plausible compiler-selected
performance policy, not a safe default.

Removing output tracking accounts for 2.78 ns when comparing the two lanes that
both omit node marks. Removing the integrity scan after that accounts for another
8.55 ns. Internal node marks are dead or effectively free in this fixed fixture,
so changing their representation cannot close the gap. The remaining
fail-stop-to-Rust difference is 16.43 ns per turn and is concentrated in resident
arena/epoch access and the differently optimized fused call site.

Two attempted code-generation changes were rejected:

- Moving arithmetic and validation into one shared out-of-line function widened
  the gap: fixed Rust measured 71.7 ns and fused Mech measured 105.5 ns.
- Explicitly unrolling the 3-by-3 multiply changed both lanes by about +0.2% and
  did not improve the compiler's existing SIMD mix.

Assembly inspection of the clean optimized Apple Silicon fused resident
prototype found partial LLVM auto-vectorization: 40 `fmul.2d`, 45 `fadd.2d`, and
3 `fsub.2d` instructions, each operating on two `f64` lanes. The same function
still contains 264 scalar add, subtract, multiply, divide, and square-root
instructions, in addition to scalar comparisons, absolute differences, and
maximums. This SIMD comes from compiling eligible native Rust loops; interpreted
and bytecode Mech do not currently perform graph-level vectorization.

More useful SIMD requires a Mech-aware structure-of-arrays batch layout that
vectorizes across independent EKF instances. That explicit lowering can preserve
one transactional boundary around the packed pure kernel and reduce lane failures
to a mask checked before publication.
