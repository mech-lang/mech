# Fuse masked equality branches on ARM64

Keep the [retained branch-fed selection](native-branch-fed-selection-pilot.md). This independent matcher trial takes **1.001666×** paired control across 16 cases. Both 1.05 gates pass on all 16 and the peer win sets agree: eight against Roc, eight against libdeflate and five against both. Natural 256 KiB improves slightly; the natural 8 KiB frontier and natural 1 MiB move the other way.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 1.000667× | 0.171835× | 0.171950× | 2.187945× | 2.189404× |
| Natural 8 KiB | 1.002776× | 1.002010× | 1.004792× | 1.245444× | 1.248902× |
| Held-out random 65,537 bytes | 0.998889× | 1.035034× | 1.033883× | 1.755065× | 1.753114× |
| Natural 256 KiB | 0.996872× | 1.147820× | 1.144229× | 0.927085× | 0.924185× |
| Natural 1 MiB | 1.005989× | 1.156475× | 1.163401× | 0.932978× | 0.938566× |
| Repeated 65,537 bytes | 1.003235× | 0.559508× | 0.561318× | 0.986656× | 0.989848× |
| Periodic 1 MiB | 1.004797× | 0.569635× | 0.572367× | 0.966927× | 0.971566× |

For a branch comparing two values masked by the same literal, equality is equivalent to all selected bits of their XOR being zero. The lowering accepts only an encodable ARM64 logical immediate, an I32 or I64 comparison, equality or inequality, and unique comparison and mask values in the optimized IR. Its two fused prefix comparisons emit `eor; tst; b.cond`. Existing loads, bounds, effects and branch arguments stay in place. The pinned peer matcher excerpts use two masks and a compare. There is no source change.

Disabling the rule reproduces all 10,188 bytes of the retained vector matcher. Enabling it produces 10,180 bytes. The trial relinks only that body at the original address, with two unreachable padding instructions. All other linked bytes except UUID and signature agree. The same body is verified in the hardware counter worker. The complete test uses 56,640 timed calls and 162 linked checks. The JIT witnesses pass 187,690 calls across equality, inequality, real shared SSA masks, different masks and nonencodable masks. Earlier shared-mask witnesses were rematerialized by the optimizer; the final witness keeps the mask live in a same-block store and verifies both the stored value and branch result. All six private payload counters agree in the timing cohort.

In the separate hardware cohort, natural 8 KiB, 256 KiB and 1 MiB retire 0.999557×, 0.999615× and 0.999517× control instructions. Random 1 MiB retires 0.998909× and repeated 65,537 bytes 1.002724×. The latter spends 0.996375× cycles while slowing slightly in primary elapsed. The exact candidate profile assigns 1,370/4,918 running timer samples to the matcher on natural 8 KiB and 3,590/5,297 on held-out random 65,537 bytes. Full-matrix timing and normal compiler emission were skipped for this unretained trial.

Measured parent `ab4c306eba81c6e474faf3b15c71efa6fb4150a8`; compiler patch SHA-256 `a211fb3f42cf0f4e2c59af766f937cee214eca4165dbcd85398edcaa22d864e4`; worker `06967baa4e3b6534e63b3302149793a387d6c97cd3d94aad5bd04e53f8b3a9c8`. The backend and witnesses use Nightly 2026-03-03 and warnings as errors.

[Compiler patch, paired measurements, linked code and verifier](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-ab4c306eba-masked-equality.tar.gz). SHA-256 `f008a59747bf2836197d49de18ce25d9f3fee48003065f0c4d18c9d5bc8f9a1f`. Run `python3 -B verify-round.py` after unpacking.
