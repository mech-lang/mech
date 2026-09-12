# Resident EKF fixture

This directory owns the deterministic input trace and numerical oracle used by
resident EKF tests and benchmarks. It is product-behavior test data, not a
release-phase gate.

Verify the committed bytes with:

```text
python3 scripts/generate-resident-ekf-fixture.py --check
```

Historical benchmark reports that consumed this fixture are retained under
`benchmarks/archive/runtime-gate-b`.
