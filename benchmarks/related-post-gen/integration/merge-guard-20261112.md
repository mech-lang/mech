# Retest of the fixed related-post benchmark

The managed Mech worker at c316e77a728f9614ae8ea34b68d36406649bb8f0 passes the fixed rust_con gate of 1.0× on the historical processing metric. The Rust worker was rebuilt from the original locked source at fc2982b0c929b1b701209d401a1dc30c773ef4ab.

| Cohort | Mech post-JSON | Rust processing | Mech/Rust | Paired median | Mech faster | Exact outputs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| primary | 1.839208 ms | 2.487958 ms | 0.739244× | 0.735533× | 30/31 | 62 |
| confirmation | 1.801750 ms | 2.448875 ms | 0.735746× | 0.738206× | 13/13 | 26 |

All 88 outputs on the 5,000-post fixture matched the original Rust output. The first cohort started with Mech and the confirmation started with Rust. Both alternated execution order in fresh processes and retained every pair. The sole slower primary pair was round 1, at 1.050024× Rust.

## Timing boundary

The Mech interval starts after JSON decoding and includes admission, setup, native execution, internal cleanup and result materialization. The Rust interval starts after pool initialization and JSON decoding and uses the original preinitialized eight-thread Rayon processing path. Whole-process wall time includes restoration, startup and JSON IO and is recorded separately.

| Cohort | Mech whole process | Rust whole process | Mech/Rust |
| --- | ---: | ---: | ---: |
| primary | 56.838 ms | 19.994 ms | 2.842811× |
| confirmation | 56.137 ms | 18.100 ms | 3.101482× |

## Managed artifact and wrapper errors

The emitted managed Object SHA-256 53d19064b79d4f57afc1688cc8d702a2cc46a48c72772a63435f9f005a83acbe matches the archived #990 Object byte for byte. The locked release build kept warnings as errors. The first link failed on five unused generated declarations. The linked runner build added narrow expected unused annotations to those declarations and kept the original runner and Object.

Both timing commands finished every invocation and wrote their raw reports. The primary wrapper then raised a missing category KeyError in summary generation. The confirmation wrapper then rejected the post-JSON summary reference as ineligible. Both exit-1 receipts and tracebacks are retained. Offline replay checks every output and reconstructs the historical processing metric from the raw records without rerunning either cohort.

This result applies to the original related-post workload. The separate 81-case compression recovery gate still uses R355.

## Run the fixed-target check

Pass the two fresh comparison reports and their executable and source pin receipt to the verifier. It checks all 31 primary and 13 confirmation pairs and exits unsuccessfully if either median ratio exceeds 1.00.

```sh
python3 benchmarks/related-post-gen/verify-fixed-target.py \
  --primary primary-report.json --confirmation confirmation-report.json \
  --pins benchmark-pins.json
```

[Raw observations and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/fixed-related-post-990-retest-edc068fea897.tar.gz).
