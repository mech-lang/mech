# Skip unused general numeric SSA

The checked private phase already omits its general machine body. The
updated compiler derives that phase's entry bounds from the same asserted
floors and checked child handoffs. It keeps the original import and function
declarations. The retained vector builds and verifies its original SSA and
must reproduce those bounds. The source proof declines cases requiring the
general body or its diagnostics.
[Maintained source and phase timings](summary.json)
· [Nine differential scenarios](result.json)

The maintained recipe reproduces the exact 71,768-byte reference object
and complete metadata. All measured builds reproduce the same bytecode,
objects, metadata and comparable executable. With the fixed benchmark link,
the exact previously timed worker passes 162 checks. The historical full
orders for that worker are 81/81 Roc and 61/81 libdeflate.
[Source rebuild](rebuild.json) · [Reference timing orders](../bound-71768-runtime/README.md)

| Matched Apple M1 source through final link | Time | Encoder text |
| --- | ---: | ---: |
| Mech control | 3.77362 s | 71,768 B |
| Mech with the checked source shortcut | 3.73777 s | 71,768 B |
| Roc | 2.98844 s | 57,696 B |
| libdeflate | 0.48067 s | 37,080 B |

The maintained comparison reduces paired native time **2.25%**
and complete build time **0.87%**. The result is **1.251×
Roc** and **7.776× libdeflate** in complete build time. Both cohorts use
fixed runtimes, adapters and allocators with one prebuilt compiler each.
The initial experimental cohort also saves 1.54% native and 0.62% complete
build time. Its separate owner-exchange flag was disabled for both actors.
[Initial raw builds](initial/timing.json.gz)
· [Maintained raw builds](maintained/timing.json.gz)

The diagnostic rebuild from the repository root uses this option.

```sh
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-refused-ssa --variant fresh-owner-counts-cold-tails-zero-branches \
  --compiler-opt-level 3 --direct-source --preselect-root-inlining --skip-refused-general-ssa
```
