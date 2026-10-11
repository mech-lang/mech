# Canonical nbody initial-data correction

The original locally authored APL and Uiua sources transcribed Jupiter's z
coordinate two binary64 ULPs away from the published initial value. These
additive copies use the exact canonical decimal `-0.103622044471123109`.
Every other source byte, algorithm operation and execution flag is unchanged;
the normalized LOC remains APL30 and Uiua21. APL's two roles remain an exact
execution alias. No original source has a full nbody timing attempt, and none
is described as a full failure.

From this directory:

```sh
python3 prepare.py
python3 validate.py
```

Preparation preserves the original source hashes, old/new binary64 encodings
and a reversible one-literal diff. Validation takes the common timing mutex
and compares complete two-line energy output for 0,1,2,10,100,1000 steps against
an independent canonical binary64 pairwise reference. The parent will admit
these additive manifests only at an inactive stage boundary, retaining all
original sources and their small-validation evidence. Three canonical50M-step
trials and fresh Mech controls are still required for performance admission.
