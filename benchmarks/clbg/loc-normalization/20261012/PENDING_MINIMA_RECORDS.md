The new adapter `pending_minima_records.validate_study` is additive and unhooked.
It matches `pending_minima.reconcile`'s explicit callback contract and returns
the two exact producer rows plus a separate list of discarded environment
records. Producer LOC remains null; canonical source review is still required.

The four allowed manifest digests are the exact manifests independently
reviewed for the parent-authorized studies. Canonical workloads are N16000,
N5500, N21, and N12. Mandelbrot's full 32,000,015-byte PBM and binarytrees'
455-byte output hashes come from the frozen `compact/run.py` HASHES. Spectral
norm and fannkuch's literal complete output bytes come from the frozen
`r-apl-uiua-comparison/20261011/run.py` validator. No producer reference field
is invented or copied into its manifest.

Qualification requires all six actual full records in their reviewed alternating
order, exact source/runtime/binary commands and fingerprints, saved raw-record
equality, normal sampler/native exit, inherited mutex ownership, whole-output
hash and byte equality, unchanged pins, no children or monitor/resource errors,
and the original AC power profile. Actual environment-only exclusions remain
archived and unscored. Missing, scaled, failed, partial, or fabricated records
cannot contribute a candidate. Derived medians and standard deviations must
equal both the original samples and producer summaries.

The proposed fixture phase has 16 methods, including four accepted canonical
oracle subcases, real environmental exclusion, raw-record tampering, missing
and extra samples, malformed process ownership, monitor/wrapper failure,
source/runtime flags, LOC mutation, and full-versus-probe rejection. These
fixtures have not been executed. A common-mutex fixture receipt and actual
completed-report reconciliation are required before the LOC pool can supply
this callback; no builder hook, data admission, or native timing is performed
by this proposal.
