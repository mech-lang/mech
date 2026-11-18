# Checked-root zero comparisons

The AArch64 backend emits one compare-and-branch instruction for the supported
native-width integer zero comparisons in the compact checked function.
The measured root contains 123 → 25 adjacent compare-zero/equality-branch
pairs. Its compiled body falls from 51,448 to 51,240 bytes. A measured
16-byte tail pad keeps the address residues modulo 64 of all 1,060 linked
text symbols. The pad follows the object's normal alignment fill and leaves
compiled relocations and checked-origin metadata intact.

| Apple M1 | Encoder text | Encoder object | Executable text | Executable file | Source through final link |
| --- | ---: | ---: | ---: | ---: | ---: |
| mech-control | 71,960 B | 159,576 B | 474,884 B | 956,056 B | 4.01350 s |
| mech-experiment | 71,768 B | 159,384 B | 474,692 B | 956,056 B | 4.00611 s |
| roc | 57,696 B | 79,160 B | 150,416 B | 230,600 B | 2.97829 s |
| libdeflate | 37,080 B | 52,128 B | 129,900 B | 210,168 B | 0.47937 s |

The source-emitted aligned recipe saves **192 text bytes**. Its encoder
text is **1.24390× Roc and 1.93549× libdeflate**. The complete build takes
**1.345× Roc and 8.357× libdeflate** time. Eight balanced orders hold the
compiler, source, fixed runtimes, allocator, and final links constant.
Median peak compiler RSS is 401.5 → 401.5 MB.
[Build trials](matched/timing.json.gz) · [Build phases and ratios](report.json)

## Runtime and checked exits

| Full order | Control Roc | Aligned Roc | Control libdeflate | Aligned libdeflate |
| --- | ---: | ---: | ---: | ---: |
| forward | 81/81 | 81/81 | 61/81 | 61/81 |
| reverse | 81/81 | 81/81 | 61/81 | 61/81 |

All 81 cases meet the 0.5% per-case resolution after the three-case duplicate-control follow-up. The package-level timing replay remains Unresolved because the earlier focused and duplicate-control receipts lack environment and contract hashes. Both full orders and the second duplicate-control study carry those hashes. The source variant stays opt-in for review.
The unaligned 71,752-byte build has an established slowdown on `empty`
in the initial paired screen. The layout follow-up measures `empty` at
+0.70% with a
[+0.14%, +1.27%] interval.
Both experiments retain their raw observations.
[Unaligned replay](focus-replay.json) · [Aligned full replay](aligned-full-replay.json)

The aligned worker passes six checked-error cases, 616 shaped and alias
calls and 162 exact encoder calls. The root frame moves from 3,744 to
3,728 bytes, and measured peak native stack moves from 5,296 to **5,280 bytes**.
The alternate compiler emits identical canonical bytecode and source origins.
**209 Cranelift tests pass**, including native-width, sub-register and bit-test
cases. The settings snapshot now includes the existing spill-pair default.
[Checked exits, shapes and stack](checks/) · [Tests](logs/backend-tests-after.log.gz)

## Rebuild the exact object

Run from the repository root on Apple ARM64.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-native-zero-branches --compiler-opt-level 3 --direct-source \
  --variant fresh-owner-counts-cold-tails-zero-branches --preselect-root-inlining
```

Both direct and split rebuilds produce the same object and complete metadata.
Separate links with the original adapter and runtime reproduce the fully timed
worker byte for byte. The rebuild rejects a stale checked-root tail profile.
[Reproduction receipts and commands](source-reproduction.json)
