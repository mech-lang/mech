# Reuse visitation storage during loop-edge live-out analysis

Replacing repeatedly allocated tree sets with reusable membership marks reduces
compiler time **3.85%**, from **31.31–31.74 to 30.14–30.49 seconds**. Complete
process time falls **3.89%**, from **31.45–31.92 to 30.24–30.66 seconds**. Every
emitted source, MBC, native Object and metadata byte remains identical to the
previously qualified fast compressor.

Source `8ab9ce063e19fbf757f655808e54979098a2b915` changes only the visited-set
bookkeeping in `cfg_cleanup::threaded_edges`. The
[stage CPU profile](native-compiler-process-profile.md) identifies ordered-set
insertion in this exact live-out traversal. Each value still gets the same DFS,
definition-owner stop, dominance tests, range facts and repair decisions.

One block-indexed generation array belongs to the optimization invocation on an
immutable CFG. Each walk starts fresh; generation wrap clears all marks. The
old set was used only for membership, never ordered iteration. This reuses
storage, not semantic proof results. No predicate, ownership, admission or
language check changes.

## Matched complete compilation

Four serial ABBA observations use frozen compilers and the same **54 existing
runtime flags**, including `MECH_NORMALIZE_DEFAULTED_READ_SETUP`, with identical
host process-statistics access. Bootstrap, a separate screen, tests, fixtures and
archiving are excluded. No other builds, tests or profiles overlap the cohort.

| Measurement | Control | Candidate | Mean reduction |
| --- | ---: | ---: | ---: |
| Compiler source through Object | 31.310, 31.743 s | 30.138, 30.488 s | 3.85% |
| Complete process | 31.452, 31.915 s | 30.238, 30.663 s | 3.89% |
| Main cleanup | 4.818, 4.978 s | 3.996, 3.996 s | 18.41% |
| Retired instructions | 475.919, 474.796 billion | 464.975, 465.705 billion | 2.11% |

Candidate maximum RSS is **2.90% higher on average**, with both candidates above
both controls. This is a time saving, not a memory saving. The exact raw counters
remain in the replayed result. Four compiles are descriptive observations;
unchanged-stage variation is not attributed to visitation storage.

The preceding [SSA construction trial](native-ssa-forwarding-compile-trial.md)
was reverted after compressor runtime slowdowns. Control production source is
identical to this experiment's base. Both subjects here emit the qualified fast
54-flag Object `b125164d7e8ba9d4e98cb4b467b3194bb14a6d8cafe411600359c6f67943c703`.
Its main machine code, numeric helpers and complete metadata all match.

## Qualification and remaining work

Two focused tests pass. One compares exact DFS order with `BTreeSet` for
**1,048,576** four-block graph/root/owner combinations, including cycles and
duplicate pending entries. The other checks sparse block IDs and generation
overflow. All four outputs also match between compilers for six complete
array/enum programs and eight Bool/U64/I64/F64 scalar programs, including checked
division. The enum-state helper uses the existing complete fixture assembly.

No new compressor runtime latency campaign is claimed: generated program bytes
remain identical, unlike the rejected SSA trial. Direct scalar emission,
checked failures, finite admission, grammar and language semantics are unchanged.

Full compilation still exceeds ten seconds. Remaining measured targets include
repeated exit-liveness graph/definition construction, semantic dimension
environment cloning and canonicalization, temporary SSA production and parser
continuation/checkpoint work. Their savings require separate qualification.

The [compact result](../../benchmarks/mech-deflate/native-profile/thread-visits-compile-apple-m3-20261114.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/thread-visits-compile-apple-m3-20261114.archive.json)
pin all 339 files, commands, source patch, contrary counters and fixture outputs.
The archive is local pending public export authorization. Local extraction
verifies every hash/Git blob; pure offline replay reconstructs the result
byte-identically without a compiler or generated executable. Raw data remains
outside source Git, and source/evidence changes are separate commits.
