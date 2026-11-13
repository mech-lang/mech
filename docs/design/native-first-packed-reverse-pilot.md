# Read the first reverse element with its native stride

Advance the packed reverse read to a separate compiler trial. A one-function fixed-address pilot takes **0.994912×** paired control time across 12 cases. Natural 32 KiB, 256 KiB, 1 MiB and Reymont improve 0.84–0.87% in both order halves. The 8 KiB halves disagree and its paired result is 1.000925×. Both 1.05 gates pass on all 12 cases. Neither peer win count changes.

| Input | Control / Roc | Pilot / Roc | Control / libdeflate | Pilot / libdeflate | Pilot / control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.171821× | 0.172590× | 2.193008× | 2.202824× | 1.004476× |
| Natural 8 KiB | 1.003722× | 1.004650× | 1.251854× | 1.253013× | 1.000925× |
| Natural 32 KiB | 1.130323× | 1.120448× | 1.023011× | 1.014074× | 0.991264× |
| Natural 256 KiB | 1.146084× | 1.136435× | 0.926163× | 0.918366× | 0.991581× |
| Natural 1 MiB | 1.155596× | 1.145680× | 0.932675× | 0.924671× | 0.991419× |
| Held-out slides, 65,537 bytes | 1.158250× | 1.149262× | 1.012005× | 1.004152× | 0.992240× |
| Reymont, 32,769 bytes | 1.128485× | 1.119008× | 1.024759× | 1.016152× | 0.991602× |
| Repeated, 65,537 bytes | 0.562091× | 0.562847× | 0.988692× | 0.990021× | 1.001344× |

Libdeflate's scalar path keeps both indexed loads in its reduction and branches only on strict improvement. The first short Mech visit uses a four-byte cost out of an eight-byte reverse element. The narrow read cannot directly use ARM64's scale-eight register offset, so it builds the byte address. The pilot derives the same reverse ordinal from the already admitted affine pair, reads its full initialized U64 element, and immediately takes the low 32 bits. It computes the continuing cursor from the same ordinal. The 32-bit reverse-read control could not fold this address; the U64 load can. The resulting first visit loses one instruction and adds no recurring stack transfer in the inspected body. Continuing and vector routes retain their original operations and may be allocated to different registers.

The retained body re-emits byte for byte. The overlay changes one 1,328-byte function with a 224-byte frame. Its two table relocations and all linked addresses stay fixed, with all other code identical. The short-route read still executes after the original admission and after the forward read. It can inspect only bytes in a complete current initialized cell. The winning item, strict ties, existing failures and fallback are unchanged. This is evidence for a general compiler experiment, without a production source change or an all-case performance claim.

The exact trial worker passes 162 complete linked checks, including exact bytes, independent inflation, capacities, aliases and cleanup. The 7,200 timed calls have equal private payload metrics. Four separate profiles cover the exact linked worker; the changed body accounts for 1,876/5,251 running PCs on natural 8 KiB, 2,610/5,331 on natural 32 KiB and 2,529/5,365 on held-out slides. The first harness setup omitted the allocator receipt and recorded zero observations. This pilot does not claim fresh hardware counters or full compilation.

Measured base `8ede8f7c4e2f603e53a3a3deeb6838f751add3c2`; linked pilot `4d2f347eb84541cd8bcfb0eb15c2a1465c10abd0b5baff9710dbb0d6e633706a`.

[Exact CLIF, linked disassembly, timings, profiles and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-8ede8f7c4e-packed-first-reverse.tar.gz). SHA-256 `97d3b1c70167dcc987203d6339ec1e9b97eb663fa0c1f5c98aa56880b620e13b`.
