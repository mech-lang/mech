# Fixed-key loops lowered to native code

The source loop, key selection and U64 replacements now run in Cranelift. The actual 5000-post case has one native loop, 28 native writes and zero per-key selection/insertion helpers; genuine loans, finite fuel, current shared-memory checks, errors and fallback remain.

| Confirmation, same cohort | Post-JSON | x fastest eligible Rust |
|---|---:|---:|
| Rust ordered mask tables256 | 1.535750 ms | 1.000000 |
| Established IR305 reference | 1.809667 ms | 1.178360 |
| IR306 borrowed bank | 1.829334 ms | 1.191167 |
| IR307 native key loop | 1.778292 ms | 1.157931 |
| IR307 callbacks-only control | 1.794875 ms | 1.168729 |

The candidate point estimate improves by31.375us over same-cohort IR305. The first paired interval crosses zero and the same-runtime control intervals cross zero in both cohorts. Keep the native implementation as the next compiler foundation, with IR305 retained as the established timing reference; no unchanged favorable reruns. The historical1.151190x figure came from a different cohort.

Confirmation instructions fall from33,267,688 to32,784,473. Allocations749 to722; requested bytes increase1,657 and additional heap peak432 bytes. Native code grows4,088 bytes, largest native frame shrinks32 bytes, executable grows55,040 bytes. Whole process/restoration regressions are recorded and do not gate this work.

All480 selected compiler contracts,168 Engine contracts,133 scalar/DEFLATE records unchanged outside timing,64 linked output cases,2170 latency outputs,504 resource outputs and64 aligned profile outputs pass.

Fresh phase probes point to serial triple construction (347us vs Rust186us), then preparation (701us vs589us) and quartets (259us vs194us). Queries win wall time (476us vs565us) while consuming more aggregate instructions. Probe medians are diagnostic, not additive savings or authoritative timings. See next-compiler-work.txt for the required producer-element/consumer-extent proof before removing native checks.

Source: 1bb7bfd2febf6b076720c76aec3a875976f28a63. Production backend: Cranelift; no LLVM dependency added. Source and four query workers are unchanged.
