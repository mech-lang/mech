# Reuse the checked continuation on layout refusal

The experimental LLVM-free encoder emits **68,136 B** from the same Mech
source and canonical bytecode. This is **704 B** smaller than the
collector-only packet build and **3,056 B (4.3%)** smaller than the
maintained 71,192 B encoder. The new size is **1.181× Roc** and
**1.838× libdeflate**. [Encoder and compiler identities](candidate.json)

The phases at canonical headers 362 and 2821 already have an ordinary
checked continuation. The root now uses it on geometry refusal before
writing any input or output packet or modifying backing storage. This
removes the 336 B and 368 B general bodies. The checked root keeps its
50,128 B extent, and every retained helper keeps its instructions and
relocations. All 24 controller windows and 16 continuation windows
mapped to the revised graph and passed the compiler's checks.
[Emission](emission.json) · [Controller map](controller-window-map.json) ·
[Continuation map](continuation-window-map.json)

The native worker passed **162 exact calls** and **six checked allocation
failures**. Its measured peak stack is **4,768 B**, compared with
4,624 B in the collector-only build and 5,280 B in the maintained build.
[Linked validation](linked-checks.json) · [Failure exits](checked-failures.json) ·
[Stack measurements](stack.json)

## Runtime

Each timing order used the same 96 balanced samples per case and
included compression through private heap cleanup.

| Order | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate |
| --- | ---: | ---: | ---: |
| Focused forward | 15/15 | 13/15 | 0.99488× |
| Focused reverse | 15/15 | 12/15 | 1.00159× |
| Complete forward | 81/81 | 61/81 | 1.00077× |
| Complete reverse | 81/81 | 61/81 | 1.00198× |

The complete comparison decision is **Reject**.
Both full orders preserve every win measured for the maintained control.
The release requirement remains 81/81 Roc and at least 62/81 libdeflate
in each order. This candidate is recorded as an experiment.
[Focused comparison](focus-screen.json) · [Complete comparison](full-screen.json)

## Build time and executable size

The table uses eight paired fresh-process M1 builds from the source of
each encoder through final link. Fixed deployment support was prebuilt.

| Build | Source through final link | Stripped executable |
| --- | ---: | ---: |
| Maintained Mech | 2.440 s | 292,568 B |
| Experimental Mech | 2.427 s | 276,056 B |
| Roc | 2.981 s | 191,912 B |
| libdeflate | 0.480 s | 155,752 B |

The candidate took **0.814× Roc** and **5.053× libdeflate** by
paired build time. In the experimental Mech build, parsing took
**0.785 s**, source lowering **0.220 s**, and native compilation
**1.348 s**. [Matched deployment data](deployment.json)

## Replay

Use the compiler built with the patch from the
[collector-only packet experiment](../scoped-phase-packets/README.md)
and its direct frame-copy rebuild. This updated recipe changes the
checked layout refusal choices and their mapped controller windows.
It checks the object, canonical bytecode, and metadata against the
recorded native worker.

```sh
python3 -B replay.py /tmp/mech-frame-copies-71192 /tmp/mech-layout-refusals
```

## Store only the phase results the caller reads

Selecting live output words shortens the encoder again to **67,512 B**.
Each phase keeps the existing offsets of the results its caller reads.
The checked failure and handoff layouts retain their separate contracts.
The smaller build passes 162 exact calls and six checked failure cases at
**4,672 B** measured peak stack. [Source replay](live-outputs/candidate.json) ·
[Linked validation](live-outputs/linked-checks.json)

| Focused order | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate |
| --- | ---: | ---: | ---: |
| forward | 15/15 | 10/15 | 1.00375× |
| reverse | 15/15 | 10/15 | 1.00426× |

Both focused orders lost prefix-random-1024 and random-1024 against libdeflate.
[Focused results](live-outputs/focus-screen.json)

The output-pruned build took **2.426 s** through final link in
the ten-pair comparison, **5.059× libdeflate**. Its stripped
executable is 276,056 B. [Build observations](single-body-preparation/summary.json)

## Prepare one body when the checked continuation owns the refusal

A separate compiler build prepares only the selected vector bodies
for these two phases. It obtains their entry lengths from that body
and retains the existing source continuation. The object, bytecode,
metadata and linked worker reproduce exactly. The ten-pair medians
were **2.423 s** for the control and **2.421 s**
with one-body preparation. The median paired saving was **2.51
ms**, with **7/10** pairs faster.
[Compiler source and observations](single-body-preparation/summary.json)

## Preserve the private phase output interface

Trimming only the non-private phases retains both sorter call interfaces
and the collector's machine code. The encoder emits **67,464 B**
with **4,768 B** measured peak stack. The exact replay,
compiler patch and result are pinned to the same supplied Mech source.

| Order | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate |
| --- | ---: | ---: | ---: |
| Focused forward | 15/15 | 13/15 | 0.99931× |
| Focused reverse | 15/15 | 12/15 | 1.00308× |
| Complete forward | 81/81 | 61/81 | 1.00129× |
| Complete reverse | 81/81 | 61/81 | 1.00542× |

Both full orders preserve every win measured for the maintained control.
The experiment remains one win short of the libdeflate floor in both orders.
The source-through-link median was **2.434 s**,
or **0.813× Roc** and **5.051× libdeflate**
by paired time. Its stripped executable is **276,056 B**.
[Compiler patch](non-private-outputs/compiler.patch.gz) ·
[Source replay](non-private-outputs/README.md) ·
[Complete results](non-private-outputs/full-screen.json) ·
[Matched builds](non-private-outputs/deployment.json)
