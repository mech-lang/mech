# Current scratch allocation and retirement

**The selected build remains 63,804 B, 81/81 Roc and 63/81 libdeflate. This study establishes no 64th win.** It measures the setup/retirement question on the current selected object, using separate diagnostic hosts. The source, compiler, original fixed runtime, selected worker and default recipe are unchanged.

Natural 1 KiB makes **65 Mech payload requests**: 64 allocator-backed payloads and one lazy mapping. C requests one compressor context plus its output buffer. This exposes repeated small scratch lifetimes, alongside the existing checked-controller and Huffman work. Mapping/unmapping alone is too small in these observations to explain the whole remaining loss. On random 1 MiB, allocator/retirement event times are tiny compared with the approximately 5.8 ms whole-call gap; the matcher remains the substantial target.

## Scope and acquisition

Both source implementations are represented by their exact existing encoder objects. The selected Mech object is `6b7a6066980489e586890372344de6ec011e06ef2c2bbb5e546ebf5ba8e0ace1`; its full metadata and canonical bytecode are pinned too. The C encoder and support objects are those used in the latest matched complete-build cohort, from libdeflate `92e6a0db9fa848d742f9eb286c92afc60f2c3dda`. Both use the same pinned mimalloc object.

There are four diagnostic actors:

- `mech-control`: original fixed runtime and selected encoder, with coarse host clocks and task-event snapshots.
- `mech-counts`: a local runtime copy records allocation, release and final-transfer events, without per-operation clocks.
- `mech-clocks`: the same event stream also records runtime-method and payload-backing durations.
- `libdeflate`: the exact C encoder, fresh compressor allocation, output allocation, compression and compressor release, with coarse host clocks. Allocator callbacks count C context requests.

Each of nine cases runs in forward and reverse case order. Actor order rotates between cases and reverses between campaigns. Every batch warms for at least 200 ms using fresh contexts, retains its final ten warmup records, then saves 32 measured calls: **72 batches, 3,024 saved calls**. Actors run serially, with no builds, tests, profiles or compression during acquisition. Every call checks stable output bytes, independent inflation and unchanged borrowed inputs/output prefixes. Saved C streams match the pinned C output oracle. All nonreturned Mech private payloads retire before completion.

The preliminary one-sample smoke used only ten warmups and showed cold-host timing effects. Its raw records and original generated diagnostic sources are retained in the separate preliminary archive; it contributes no reported campaign median.

## Requested backing and observed faults

These counts are deterministic across both event modes, both orders and every saved call. The runtime verifier reconstructs live backing from actual pointer identities and checks total requests, bytes, zero-initialized extent, peak live extent, release/transfer matching and empty final ownership.

| Input | Mech payload requests | One mapped payload | Requested zero-initialized extent | Median Mech task faults |
| --- | ---: | ---: | ---: | ---: |
| Natural 1 KiB | 65 | 1,052,672 B | 1,604,676 B | 1 |
| Natural 4 KiB | 97 | 4,210,688 B | 4,773,188 B | 3 |
| Natural 8 KiB | 114 | 6,002,052 B | 6,570,440 B | 6 |
| Random 4 KiB | 60 | 4,210,688 B | 4,761,004 B | 2 |
| Random 16 KiB | 79 | 6,002,052 B | 6,558,460 B | 5 |
| Random 64 KiB | 78 | 6,002,052 B | 6,558,256 B | 17 |
| Random 1 MiB | 254 | 6,002,052 B | 6,615,760 B | 74 |
| Distribution change | 134 | 6,002,052 B | 6,580,016 B | 4 |
| Overlap, 320 B | 59 | 328,960 B | 880,300 B | 1 |

C requests **9,011,751 B** for its aligned compressor context on each case, plus a separately allocated output buffer. After warmup, its median task-fault delta is zero on these inputs. A fresh logical context can receive already committed storage from the allocator. The Mech mappings are created and released in every call.

Payload counts exclude allocator calls for owner descriptors and registry vectors. Their cost is included in the traced runtime methods, but their separate allocation count remains unmeasured. Requested initialized extent is **not physical clearing traffic**. Task faults are measured with Darwin `TASK_EVENTS_INFO`; they do not identify the faulting instruction, quantify cleared bytes or isolate first-touch elapsed time. The snapshot itself contributes a common Mach call. No page-ins or copy-on-write faults appear in the reported medians.

## Natural 1 KiB cost

| Coarse diagnostic | Forward | Reverse |
| --- | ---: | ---: |
| Mech original-runtime host, complete call | 70.480 µs | 70.084 µs |
| C complete call | 56.333 µs | 56.395 µs |
| C context construction | 4.125 µs | 4.125 µs |
| C compressor release | 0.084 µs | 0.084 µs |
| Mech event-clock host, complete call | 78.688 µs | 78.000 µs |

The original-runtime diagnostic gap is about **14 µs**. In the event-clock host, sums of the group medians are:

| Mech runtime activity | Calls | Forward | Reverse |
| --- | ---: | ---: | ---: |
| Ordinary allocation method, including owner registration | 64 | 7.606 µs | 7.397 µs |
| Ordinary release method, including owner retirement | 63 | 3.915 µs | 3.877 µs |
| Mapped allocation method | 1 | 0.542 µs | 0.500 µs |
| Mapped release method | 1 | 1.709 µs | 1.334 µs |
| Final output transfer | 1 | 0.041 µs | 0.042 µs |

The allocation/release method totals include per-operation clock overhead. The clock host is roughly 11% slower than its original-runtime diagnostic on this case, and about 39–59% slower on tiny overlap. These totals are **diagnostic indications, not removable time budgets**. No subtraction of timer overhead is used to predict a win. The count-only host remains much closer to control. Full raw observations include larger variation on distribution change; this panel is not the balanced full qualification campaign.

Across natural 1 KiB, the payload-backing portions of ordinary allocate/release total about 5.7 µs in the clock host; the rest includes owner creation/registry work, checks and clocks. This supports examining transient scratch/owner lifetimes and Huffman construction together. Prior halfword/word mapping-threshold experiments already lost required wins and are not promoted from these traces.

## Large random input

Random 1 MiB takes approximately **26.01–26.07 ms Mech** versus **20.18–20.29 ms C** in the original-runtime coarse hosts. The sums of traced Mech allocation/release/transfer group medians are **53–55 µs**, while C context construction and cleanup together are approximately **55–67 µs**. These boundaries differ and instrumentation perturbs them, so they are not a matched semantic-phase comparison. They provide no evidence that allocator-call reduction can recover the multi-millisecond matcher gap. First-touch elapsed time inside the encoder is still not isolated.

## Reproduce and verify

The proof archive contains the exact config, all commands, build logs, generated runtimes, diagnostic workers, original runtime sources/library and pinned encoder artifacts. The observation archive contains every campaign record, output stream, fixture content and oracle. Every archive member is hashed in [archives.json](archives.json).

On Apple AArch64, rebuild the dependencies and remap local paths in the archived config while preserving input hashes:

```sh
python3 build.py /path/to/config.json /tmp/mech-setup-diagnostics
python3 acquire.py /path/to/config.json /tmp/mech-setup-diagnostics /tmp/mech-setup-observations
```

Portable verification requires only Python's standard library:

```sh
python3 verify.py
```

The verifier checks source generation, unchanged runtime files, native artifact identities, all 72 batches, all 3,024 saved call records, **1,512 complete allocation-event streams**, reconstructed lifetimes, raw medians and independent inflation. [Verification receipt](verification.json). No production managed-entry, whole-GPU-suite or new error/stack qualification is claimed for diagnostic hosts.

The selected Mech source, emitted text and build recipe are unchanged: **63,804 B**, **2.219133 s** source through final executable link in the latest ten-trial matched cohort. A new compile-time campaign is unnecessary for these diagnostic-only additions. The next work is captured-input Huffman/collector attribution and a bounded reduction of transient scratch lifetimes, preserving all **81 Roc / 63 C** named wins. **64 C wins is next; 81 remains the goal.**
