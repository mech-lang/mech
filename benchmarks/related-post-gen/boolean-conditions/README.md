# Native Boolean proof and bit-branch follow-up

The compiler proves that native SSA values are zero or one across every join
and loop edge before simplifying Boolean equality branches. It trusts actual
native producers, not declared source kinds. Unknown inputs, loads, calls and
noncanonical backedges invalidate dependent proofs. Worklist bounds retain the
original path when the analysis cannot prove a result.

Numeric range proofs run first so Boolean rewriting preserves their existing
guard eliminations. The final predicate exposes bit zero to the backend's
native bit-test branch lowering. All source visits/debits, checked failures,
ownership/resource admission and the efficient native scalar entry remain.

Measured compiler: `9436e51d22b2955055f9581f2534064ee203e461`,
IR224/work114/ABI40/image8. Parent: IR223 at
`5925a2b3a4fd4dfcca7ada541e37a69d514171f8`.
The existing compiler review is [PR #990](https://github.com/mech-lang/mech/pull/990),
which targets the branch behind #973. This follow-up branch keeps that review
checkpoint stable.

## Result

Same Apple M3, 5,000 posts, 15-round confirmation:

| | Parent IR223 | Bit-test candidate | Fastest eligible Rust |
| --- | ---: | ---: | ---: |
| Continuous post-JSON, including setup/admission/workers/results | 3.142834 ms | **3.117125 ms** | **1.586375 ms** |
| Complete fresh launch through exit | 38.820250 ms | **36.647000 ms** | **23.705417 ms** |

The candidate is **1.9649x Rust** for processing and **1.5459x Rust** for
complete time. The processing reference is checked U8-ID/U32-rank Rust; the
complete-process reference is checked U32-rank Rust.

Paired confirmation wins against the parent are 12/15 for processing and 10/15
for complete time. Against the selected IR208 checkpoint, processing wins
15/15, but complete time wins only 6/15, with a 1.485 ms median paired
regression. **IR208 remains selected. Rust parity is unmet.**

The separate resource cohort records 64,446,990 to 64,186,370 processing
instructions (-0.40%), with allocation requests/bytes unchanged at
3,858/8,043,926. Peak physical footprint rises from 16,236,904 to 16,335,208 B.
The query changes 29,196 to 29,024 code bytes, keeps a 688-byte stack frame and
uses six ARM bit-test branches. Static instructions fall from 7,299 to 7,256.
The direct-branch prototype reduced more static instructions but increased
stack size and total executed instructions; its report is retained separately.

Both original and compact sources were measured with four and eight numeric
workers. Four workers retain the best processing median. No latency samples
were discarded, and profiler/resource timings do not replace headline latency.

## Qualification and evidence

The final candidate passes 691 GPU tests with the same two inherited failures,
both new Boolean proof/ordering tests, 34 numeric-tree checks, 124 differential
fixture checks, 1,437 exact cohort outputs and 2,000 exact profile invocations.
All 133 scalar/DEFLATE controls retain every nontiming field, including
allocation counts/bytes. The 11 unchanged production dependency libraries are
reused by verified source/hash equality.

The additional full loader, finite-budget, runtime-only and old-Object matrix
has not been rerun for this unselected candidate. Adoption requires that wider
qualification; this branch publishes a screened compiler change for review.

[The final report](evidence/compiler-native-boolean-bit-tests-screen-20261106.json)
links all retained cohorts, sizes and fresh profiles in `evidence/`.
`qualification/` contains the test and control receipts. File digests are in
`evidence-sha256.json`. Native binaries, raw sampler data and complete source
archives remain in the separately indexed local evidence archive; this packet
does not claim those were remotely uploaded.

The native query remains the leading sampled application-code hotspot.
Observed windows still contain increment checks, stack traffic, register
copies and conditional selections. Samples are not CPU percentages, and no
inherited source mapping is applied to changed functions. Further work should
target proved loop funding bounds and immutable producer-derived element
ranges; this Boolean path has produced only small timing gains.
