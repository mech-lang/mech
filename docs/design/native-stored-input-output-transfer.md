# Stored-input output transfer pilot

**Keep the current source while isolating the remaining wrapper cost.**

This screen uses the same retained R355 generated Object in four workers. Two workers use matched runtime-only builds that differ by one inline attribute on output transfer. Linked disassembly confirms that the attribute eliminates the call and its view staging. The full historical and merged-runtime workers remain controls.

| Input | Inline/R355 | Inline/merged runtime | Inline/matched attribute-off |
| --- | ---: | ---: | ---: |
| empty | 1.150531× | 0.937092× | 0.981542× |
| one-stored | 1.112024× | 0.959854× | 0.971468× |
| seven-stored-prefix | 1.158506× | 0.971922× | 0.988115× |
| eight-literals | 1.016192× | 1.003455× | 0.999069× |
| prefix-natural-262144 | 1.006187× | 1.004554× | 1.003856× |

Inlining helps the three short stored-input paths in the matched attribute experiment. The candidate remains 11.2–15.9% slower than R355 on those inputs, and both larger controls move slightly slower than the full merged-runtime worker. The remaining allocator shim and heap-state transport are separate leads.

The screen recorded 153,696 complete calls in 24 rotated actor orders. It checked exact bytes, independent inflation, aliases, output capacity, heap counters and cleanup. The archive includes the linked timed spans and the original zero timer observations.

[Raw observations, disassembly and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/stored-input-output-transfer-a3f040279fd0.tar.gz).
