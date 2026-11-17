# Cold error-origin tables

The prototype replaces 291 independent AArch64 origin stubs with three
packed origin tables and local decoders. It selects separately entered cold
instructions that return through status-only code into an epilogue that
restores the saved link register. Every original origin and return target
was checked against the emitted object. The object prototype also preserves
all 31 function entry addresses modulo 64.
[Origin map and exact identities](link-proof.json) · [Backend design](SOURCE-PLAN.md)

| Apple M1 | Control | Prototype |
|---|---:|---:|
| Encoder text | 71,768 B | 71,256 B |
| Encoder object | 159,384 B | 158,872 B |
| Linked executable text | 474,692 B | 474,180 B |
| Linked executable file | 956,056 B | 956,056 B |
| Measured peak native stack | 5,280 B | 5,280 B |

The prototype's encoder text, including its tables, is **1.23502× Roc**
and **1.92168× libdeflate**. The checked failures preserve their status,
source origin, and allocation observations. All 162 successful smoke calls
also pass with the three decoder entries replaced by traps. Those inputs
reach their successful results without entering a decoder.
[Correctness and stack checks](checks/) · [Decoder entry probe](cold-entry-proof.json)

Each focused order preserves all prior wins in its 16 cases, with
**16/16 Roc and 11/16 libdeflate**.

| Link experiment | Empty-input estimate | 90% paired interval | Validator |
|---|---:|---:|---|
| Standard matched link | +1.215% | +0.663% to +1.763% | Reject |
| Common stub section alignment | +0.893% | +0.248% to +1.516% | Unresolved |

The second comparison specifies the same 4 KiB stub section alignment on
both links. The stub, helper-stub, constant and string sections start at the
same addresses in both workers. The checked root entry also has the same
address. The empty-input interval still crosses the 0.5% policy resolution.
One-stored, seven-stored-prefix and the 4 KiB random-prefix case are also
unresolved. Both link experiments are focused screens. The maintained
source recipe and full timing evidence are in the checked-root branch report.
[Linked address measurements](aligned-link-placements.json) ·
[Original screen](standard-link/focus-replay.json) ·
[Section alignment screen](aligned-sections/focus-replay.json) ·
[Full suite control](../branch-on-zero/README.md)

## Reproduction

The prototype command takes the directory containing the exact source-emitted
`program.o` and `metadata.json`, then writes the 71,256-byte Mach-O text
object to a new directory. The script checks both object hashes. The input
comes from the PR's `fresh-owner-counts-cold-tails-zero-branches` rebuild.
This diagnostic uses Darwin's object tools.

```sh
evidence=benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/cold-origin-tables
python3 -B "$evidence/prototype/pack-origins.py" /tmp/mech-frame-pool/native /tmp/mech-cold-origins
python3 -B "$evidence/replay.py" standard-link
python3 -B "$evidence/replay.py" aligned-sections
```
