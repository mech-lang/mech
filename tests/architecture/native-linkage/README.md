# Native linkage evidence

`coverage.json` records the historical catalog before `logic/all` was added.
Its counts and digests have not been regenerated or presented as measurements
of the current catalog.

The verifier checks the exact `logic/all` factory ID, Boolean reduction
signature, owner, installer, Cargo feature set, contract kind, and alias policy.
It then compares every historical factory and its metadata with the committed
baseline. Missing, renamed, unexpected, and changed entries still fail.

Newly generated reports contain the actual complete catalog count and digest.
They also label a `historical_baseline` comparison, reconstructed after removing
only the independently validated addition. Validation can therefore use the
committed historical evidence without relabeling its fingerprints as current.
An explicit `check-native-linkage-coverage.py report` run records fresh complete
coverage evidence when the full native factory inventory has been regenerated.
