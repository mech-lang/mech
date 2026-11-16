# Qualify equal join arguments with branch cleanup

The matcher sends the same current cursor twice on each incoming edge to two joins. The compiler proves equality for each pair of argument slots, recursively through other joins and with a conservative refusal for unresolved cycles. The existing block-argument repair replaces the redundant slot. The constant branch fold then removes the last-match load path, and the header store uses a zero match count. The literal selection, cache bounds, writes, cursor increment, and countdown retain their original order. The proof runs in the existing compact private-vector pipeline under `MECH_COALESCE_EQUAL_JOIN_ARGUMENTS=1`.

The measured production base is `aa26a947138681dcc6a0d21434ae68b98ce248b0` plus the four-file patch. One of 351 Object bodies changes. The linked matcher shrinks from 10,284 to 10,188 bytes and keeps its 576-byte frame. The branch and unreachable load block disappear from the selected route. Its complete body matches the separate selected-read pilot by SHA-256. The root, ordered reductions, and all other bodies match the retained cursor image.

Original and decoded forms independently emit Object `43c4f7bd69d3d682177080e9599f3f98572032c5ea3d86f09cbb25ce86922755` and metadata `a30f636f59cfbd42e94e6a310af4ca84b2d9f2891713316cce37f0f4f32bd2de`. Both linked workers have SHA-256 `3a02b1311e66296b6516da01befb98a9b35f3a6c1d6c0e63cec3e27f2201c5b3`. The original source and bytecode hashes agree with the direct control. The same pinned workers for both peers participate in each balanced order.

| Full matched comparison | Cursor control | Cursor plus branch fold |
| --- | ---: | ---: |
| Faster than Roc | 47/81 (58.0%) | 47/81 (58.0%) |
| Faster than libdeflate | 12/81 (14.8%) | 17/81 (21.0%) |
| Faster than both | 3/81 | 8/81 |
| Faster than either | 56/81 (69.1%) | 56/81 (69.1%) |

| Input | Control/Roc | Candidate/Roc | Control/libdeflate | Candidate/libdeflate | Candidate/control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.172869× | 0.172597× | 2.209334× | 2.205854× | 0.998425× |
| Natural 8 KiB | 1.029711× | 1.030752× | 1.277651× | 1.278943× | 1.001011× |
| Natural 256 KiB | 1.168409× | 1.171875× | 0.947258× | 0.950068× | 1.002966× |
| Natural 1 MiB | 1.175519× | 1.178473× | 0.948835× | 0.951219× | 1.002513× |
| Periodic 1 MiB | 0.597205× | 0.586191× | 1.011154× | 0.992506× | 0.981557× |
| Repeated 32,767 bytes | 0.509697× | 0.501892× | 0.997362× | 0.982090× | 0.984687× |
| Repeated 64 KiB | 0.574281× | 0.563809× | 1.017495× | 0.998941× | 0.981765× |
| Repeated 128 KiB | 0.598778× | 0.590877× | 1.004404× | 0.991151× | 0.986805× |
| Repeated 256 KiB | 0.606049× | 0.597163× | 0.985909× | 0.971453× | 0.985338× |
| Repeated 1 MiB | 0.607335× | 0.597743× | 0.954373× | 0.939300× | 0.984206× |

The 81-case, 245,664-call campaign measures **0.994529× control**. Both 1.05 timing gates pass all 81 cases, and all six payload metrics agree. Fifty-four paired estimates improve and 27 increase. The largest increase is empty input at 1.015409× control. Periodic 1 MiB and repeated 32,768, 65,535, 65,536, and 131,072 bytes cross below libdeflate. The closest remaining libdeflate case is repeated 65,537 bytes at 1.004584×. Natural 8 KiB remains the closest Roc miss at 1.030752×.

The nine-case screen measures 0.992694× control and passes both 1.05 gates. Its separate counter cohort puts periodic 1 MiB at 0.983574× instructions and 0.982582× cycles. Repeated 1 MiB measures 0.983934× instructions and 0.983870× cycles. Natural 8 KiB spends 1.001930× cycles. The nine linked profiles identify sampled PCs in the exact timed candidate.

Control, original candidate, and decoded candidate each pass 162 exact byte, independent inflate, capacity, held alias, and cleanup checks. The maintained branch test passes 54 calls with warnings as errors and the register allocator checker. It verifies branch retirement on both agreeing routes and checks that a different current argument retains the branch. The earlier equal-argument suite passes 700 differential pairs. The fixed managed guard passes 88 exact outputs and measures 0.705521× and 0.705512× Rust in the respective post-JSON cohorts. Changed-file formatting, `git diff --check`, and the scoped unsafe audit pass. The repository-wide unsafe audit reports the inherited unlisted probe at `benchmarks/related-post-gen/check-measurement.rs`.

[Compiler patch, complete qualification, raw timing packets, and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-aa26a94713-equal-join-qualification.tar.gz). Archive SHA-256 `f7104bf3d75b0cc17b454e6fca8fae38e2fc847d0b33ac445dcbae90e6074f00`.
