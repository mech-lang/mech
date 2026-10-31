# Borrowed record-selector experiment — not retained

Retained baseline: `b12f6c5afb2c859f6cde29ba566ddf09e70d2f79`.
Candidate implementation: `f9f8e4a50` (full revision in scorecard); optimized build source: `cc22c86ba5dc20ae5bba5a45a4bdb21dd302920b`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The candidate borrowed the current selector instead of creating its temporary owner, then reacquired its borrow after coexistence measurement. It preserved ordinary checks and debits. It did not establish an elapsed-time improvement: **4.2711875 → 4.280083 ms**, **14.882186× → 14.913181×** the common fresh 0.287 ms reference, **0.208268% slower**. One of four preparations and 25 of 64 blocks improved. The retained compiler remains unchanged.

The four matched pairs contain 3,968 measured calls (1,984/version), with warmup and profiling observations identified separately. All completed exact reference bytes, independent decompression, preserved input aliases/content, 13-byte output capacity, work 60,260,219,686 and zero live cleanup. The fixed eight-byte source, bytecode, algorithm, allocator and timing boundary are unchanged. No large-input timing is claimed.

Separate coherent diagnostics: instructions **48,597,760.5 → 48,546,303** (−0.105885%); cycles **13,646,173 → 13,657,909.5** (+0.086006%); allocation requests **13,994 → 13,994**; requested bytes **98,586,843 → 98,586,843**; additional peak heap **20,575,432 → 20,575,432**. Static generated code and stack are unchanged. Host frame action grows from 1,332 to 1,392 instructions. Hardware and sampling observations are outside elapsed timing. Baseline sampling failed to resolve worker symbols (1,244 weights); no before/after profile-category comparison is claimed. Candidate sampling resolved symbols.

Warnings-as-errors optimized build passed. The new focused borrow test, all 316 runtime tests and 32 work/memory/value tests passed. Initial test-adapter compilation and count-expectation errors were corrected and preserved in the receipts. Broader component, linked and architecture requalification was not run because this candidate is withdrawn; existing baseline qualification remains round 43. No CI run was triggered for this experiment.

`reconstruct.py` checks every archived file and reproduces both scorecards byte-for-byte. Commands, feature configurations, source patch, build identities, observations and failures are archived. The next unmeasured borrow refinement is separately closed in round 45. The next bounded production change is fully funded record-lookup prefix accounting.
