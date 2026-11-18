# Compiler-emitted aligned spill pairs

The AArch64 diagnostic compiler pairs consecutive integer spills or reloads
when the slots are adjacent, the stack offset fits and no stack or debug
observer needs the original edit locations. The object writer applies the
phase profile's tail padding after the verified machine buffer, keeping
instruction offsets, relocations and phase metadata intact. The matcher
loses 96 bytes of instructions and gains 32 bytes of unreachable NOP padding.
The resulting object preserves every native function's address modulo 64.

The rebuilt object, complete metadata and linked timing worker match the
[earlier placement diagnostic](../register-spill-pairs/README.md) byte for
byte. The register allocation checker produces those same identities.
Pairing disabled reproduces both the retained 72,520-byte encoder and the
71,960-byte smaller control, including their exact linked workers.
[Identity receipts](preflight-identities.json)

## Matched comparison on Apple M1

| Configuration | Encoder text | Encoder object | Linked executable file | Complete build |
| --- | ---: | ---: | ---: | ---: |
| Mech control | 71,960 B | 159,576 B | 956,056 B | 4.53667 s |
| Mech aligned pairs | 71,896 B | 159,512 B | 956,056 B | 4.53363 s |
| Roc | 57,696 B | 79,160 B | 230,600 B | 2.98708 s |
| libdeflate | 37,080 B | 52,128 B | 210,168 B | 0.48076 s |

The paired Mech encoder is **1.24612× Roc** and **1.93894× libdeflate** in
text size, and **1.51775× Roc** and **9.43012× libdeflate** in complete build
time. The comparison measures fresh source through final executable link
with the compilers, fixed runtimes and adapters prebuilt. The eight balanced
orders give a paired candidate/control build-time change of +0.045%, with a
90% interval of −0.088% to +0.179%. The build-time effect is effectively flat.
[Complete measurements](report.json) · [Raw build trials](source-to-link/timing.json.gz)

Parsing takes 2.456 s of the 2.783 s Mech frontend median. Native emission
takes 1.707 s and final linking takes 0.041 s. In separate instrumented trials,
Roc spends 1.648 s specializing and 0.810 s in LLVM optimization and emission.
Clang records 82 ms of frontend work and 448 ms of backend work for the C
encoder. These individual profiled trials have their own measurement
overhead. [Build profiles](source-to-link/profiles/)

## Runtime and decision

Both complete orders record **81/81 Roc** and **61/81 libdeflate**, with
all individual peer wins preserved and supported by the paired intervals.
Each full order has 982,656 timed calls. The linked worker passes 162 exact
calls, six checked-error origin probes and 616 extra shape checks. The root
frame is 3,744 B and measured peak stack is 5,296 B.

| Case | Candidate versus control | 90% interval | Policy result |
| --- | ---: | ---: | --- |
| `empty` | +1.704% | +0.933% to +2.549% | Regression |
| `seven-stored-prefix` | +1.085% | +0.592% to +1.550% | Regression |
| `prefix-natural-65535` | +0.201% | +0.138% to +0.264% | Bounded |
| `prefix-natural-262144` | +0.336% | +0.274% to +0.408% | Bounded |

The v1 timing decision is **Reject**. Seven other cases are unresolved.
Keep the existing default and use the variant for opt-in diagnostics. The
legacy timing adapter asserts exact compressed bytes and keeps its original
correctness log format. The log lacks the explicit per-call output digest
required for promotion. [Policy result](runtime-policy.json) · [Proof receipts](checks/)

The two complete orders use the original absolute-path fixture inventory.
The replay compares the input, prefix and expected-output bytes by SHA-256
for every case with the committed inventory. The acquisition receipts and
raw timing observations are preserved. [Fixture content bindings](fixture-content-bindings.json)

## Reproduce from the PR source

Run from the repository root on Apple ARM64.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-aligned-spills \
  --variant fresh-owner-counts-cold-tails-paired-spills \
  --compiler-opt-level 3 --preselect-root-inlining
```

The maintained recipe checks the exact canonical bytecode, object and
complete metadata. Its optional `--mimalloc-object` builds the recorded
worker and runs 162 exact calls. That worker has its own linked identity
and correctness receipt. The runtime counts above refer to the fixed
benchmark worker. [Maintained rebuild](maintained-rebuild/rebuild.json)

From this evidence directory, replay both fresh focused and complete orders.
The replay uses the archived v1 runtime procedures and the acquisition plan
pinned before the campaign.

```sh
python3 -B replay_runtime.py --plan-sha256 "$(cat plan.sha256)"
```
