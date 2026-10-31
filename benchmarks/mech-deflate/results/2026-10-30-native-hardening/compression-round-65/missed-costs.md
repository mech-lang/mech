# Additional compiler/runtime costs reviewed

The accepted R63 and R64 checkpoints are 50794460eeffabb156e513f2dde01c08a23cb9f4 and 78614f175a932b37a73e9b8a12be1fd4e390c9f3. The R65 measured build is 78209b2aab2175f1fad92ee4daae3d6b38669d36. Every round retains its actual source/build identities and independently timed baseline.

1. Complete current shape checks were unnecessarily coupled to footprint-recipe eligibility. R63 removed 68 requests / 1,024 requested bytes, with 0.892357% pooled elapsed and 0.157790% instruction reductions. The complete-axis proof preserves nested-output overflow and conservative fallback; parent/candidate/ordinary-constructor tests establish ordering and recovery.
2. Two central projection-meter queries still built complete diagnostic results although they used only first/empty status. R64 switches those queries to Core's existing ordered first-violation checker. Matched elapsed improves 1.064733% and instructions 1.003238%; successful allocation counts remain unchanged. This is a separate missed application of a previously maintained primitive.
3. Actual current ARM64 host code retained an outlined checked-debit helper, constant zero comparison work and stack Result transport. R65 forces inlining of that existing body. The full current machine review and adverse observations are in binary-review.md. Source authority, arithmetic and TLS order remain identical.

Each primary comparison contains four independently prepared pairs and 3,968 timed observations. Instruction/cycle, heap/census and CPU-time sample cohorts are separate. Their gains are not added or multiplied. Only the complete pinned eight-byte level 12 call is measured.

## Queued, unapplied candidates

The queued-construction-costs packet includes exact R62 allocation stacks and source-only patches. These are opportunities, not measured savings:

- native_semantics::bind_call unconditionally clones BoundCall even when no second seal owner is needed. Two seed calls account for 72 attributed requests / 5,404 requested bytes. Move the existing descriptor directly on the no-candidate path; retain the certified path's required second owner and full validation. CandidateWitness's Copy semantics can be retained without cloning its metadata graph. Existing seal and binding tests must run before accepting the patch.
- A zero-count append comparison re-encodes/hashes the template output schema although the prepared call retains that exact output key. Sixteen current requests belong to this stack. Existing original/decoded zero-count identity, current capacity, finite funding and retry witnesses are the proposed acceptance controls.

## Initialization fidelity control

The initialization-audit packet checks all fourteen pinned reference source hashes and the actual Mech source 331e659e. Both programs fully zero 524,288 tree bytes and 6,002,052 cache bytes, and reserve 2,442,056 path bytes with initial length zero. No initialization mismatch was found. Extra requested path capacity is the previously identified ownership issue; it is not evidence of zeroing or copying the entire reservation. No omitted initialization or algorithm change is proposed.
