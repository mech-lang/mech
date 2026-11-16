# Immutable parser packet reuse trial

The composed parser reloads words from its immutable invocation-local input packet when it enters the empty run and when it returns to the general recurrence. Each word has an identical dominating entry load. The packet pointer is used only for aligned reads within its admitted extent. The pilot forwards those values, leaving the source reads, writes and checks in order.

| Parser body | Code bytes | Full frame bytes | Static stack references |
| --- | ---: | ---: | ---: |
| Control | 1,216 | 224 | 16 |
| Forward at empty-run exit | 1,152 | 224 | 12 |
| Forward at entry and exit | 1,136 | 208 | 11 |
| Forward at entry only | 1,296 | 256 | 34 |

The entry-only variant exceeded the existing linked slot. The other two use the same function address and keep both lookup relocations attached to the same targets. All other named addresses and linked instruction bytes agree. The control re-emits the exact retained body. The initial audits exposed a descriptor key mismatch, a legitimate register assignment change and a linker GOT load relaxation. The final audits verify register operands and relocation targets independently.

| Input or cohort | Exit / control | Entry and exit / control |
| --- | ---: | ---: |
| 18-case geometric mean | 1.011689× | 1.006979× |
| Distribution change, 16 KiB | 1.007937× | 1.007426× |
| Natural 16 KiB | 1.021110× | 1.009031× |
| Reymont 16 KiB | 1.018060× | 1.005353× |
| Natural 256 KiB | 1.026315× | 1.018582× |
| Natural 1 MiB | 1.028101× | 1.014362× |
| Random 64 KiB | 0.989362× | 0.996607× |
| Eight literals | 0.999281× | 0.992880× |

| Natural 16 KiB cohort | Control / Roc | Candidate / Roc | Control / libdeflate | Candidate / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Exit | 1.045067× | 1.067128× | 1.058111× | 1.080448× |
| Entry and exit | 1.055299× | 1.064829× | 1.062219× | 1.071812× |

Keep the qualified prefetch compiler. Both pilots passed the two 1.05 gates on all 18 cases and each retained 14 wins against at least one peer. The exit variant gained one libdeflate win on a case already below Roc. The 28 entry words are unchanged. Forwarding extends some live ranges through the fast loop, and the savings above count static instructions at transitions. The natural 16 KiB parser accounts for 2,805 of 5,258 mapped running samples in the exit variant and 2,819 of 5,304 with both sites forwarded. The retained executable previously recorded 2,735 of 5,319. Samples include validation and cannot partition elapsed changes or retired instructions.

The two screens include 15,552 timed calls, 324 linked checks and eight independent exact-worker profiles. The replay checks reference bytes, independent inflation, capacities, tracked allocations, aliases and cleanup. The baseline worker is `1bfd044d1218b68549bfbfd4e3a6b25d150762108dfdbb0ca3d680b9fed903f8`, produced by compiler `241e01883a6000d9b78f407be94d50d0ed860ee7`. The test changes one emitted parser body. The pinned source, bytecode, allocator and full-call timer are shared.

Run `python3 -B verify-round.py` after extraction. The archive contains 129 checksummed files. SHA-256 `b381646157bbf36a269e39f63dddbb92bc90bcff90d83ab0aea49d136319f528`. [Paired screens, profiles and disassembly](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-cb8dfa790d-parser-packet-reuse-trial.tar.gz).
