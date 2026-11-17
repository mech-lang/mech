# Skip unused compact failure returns

After the source site emits all paths, the compiler seals its private failure
destination. It emits the site origin and return if the block has incoming
edges. The six-order causal screen omits **5,236** otherwise unused returns,
reduces the native median from **1.42586 s to 1.41773 s**, and saves
**0.40%** paired source-through-link time. It reproduces the exact
reference bytecode, encoder object, metadata and stripped executable.
[Ablation and bindings](ablation-receipt.json) · [Six paired builds](ablation-observations.json.gz)

The maintained source uses the same admission and drops the diagnostic switch.
A prebuilt compiler and fixed deployment support produce these eight-order
results on Apple M1.

| Measurement | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.70063 s | 2.98958 s | 0.48058 s |
| Source through strip | 2.71108 s | 3.00001 s | 0.49107 s |
| Encoder text | 71,192 B | 57,696 B | 37,080 B |
| Stripped executable used in timing | 292,568 B | 191,912 B | 155,752 B |

The published Mech build is **0.903× Roc** and **5.621×
libdeflate** through final linking. The reference worker reproduced exactly and
passed 162 linked checks. [Build plan](matched-plan.json)
· [All matched builds](matched-observations.json.gz)
· [Frame-copy runtime qualification](../frame-copy-v2/README.md)
