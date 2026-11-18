# Prepare the admitted scorer before building other variants

The compiler prepares the checked no-growth scorer in a private declaration recorder. A proved dense body commits the recorded definitions and uses the existing checked source route when capacity is insufficient. An unadmitted attempt drops the recorded definitions and continues with ordinary compilation.

This fixture avoids **2,168 provisional blocks** and **5,358 IR instructions** by skipping the two unused growth-capable bodies. The encoder object, canonical bytecode, stripped executable and runtime worker reproduce the qualified build byte for byte.

The phase diagnostics now report the omitted bodies. Every metadata field the linked worker reads is equal, including the input and output word layouts. Linked checks cover 162 success observations and the same six checked failures.

[Avoided compiler work and identity checks](compile-work.json) · [Linked checks](linked-checks.json) · [Refused preparation identity checks](decline-comparison.json)

## Fresh source through final link

Apple M1 ran 8 balanced fresh builds per recipe with fixed toolchains and deployment support prebuilt.

| Build | Source to final link | Mech native phase |
| --- | ---: | ---: |
| Qualified compiler recipe | 2.300 s | 1.213 s |
| Prepare dense scorer first | 2.283 s | 1.191 s |
| Roc | 2.986 s |  |
| libdeflate | 0.480 s |  |

The paired median difference is **18.42 ms saved**, with 8/8 pairs faster. The new build took **0.764× Roc** and **4.757× libdeflate**.

[Phase medians and paired ratios](deployment.json) · [Raw builds](build-comparison.json.gz)

## Reproduce

Apply the compiler patch in an isolated checkout at 9b5de6f46819660b1e6b41ddd38003e8cb8041d9. Build with the existing frame-copy baseline commands, then run replay.py with the baseline rebuild and a fresh output directory.

[Compiler patch](compiler.patch.gz) · [Compiler build instructions](../shared-continuation-returns/README.md#replay) · [Verified source replay](verified-replay.json)
