# Reuse the checked scorer growth continuation

The no-growth scorer already has a capacity check. This prototype uses
that check to resume at the existing checked continuation when capacity
is insufficient. It branches before writing the phase packet or changing
ownership. The new route makes the separate general scorer body
unreachable from the deployment entry. It uses the existing source Plan
and preserves both fast scorer bodies and the match collector's machine
instructions and relocations.

| Emission | Matched object text | Peak stack on linked checks |
| --- | ---: | ---: |
| Published Mech build | 71,192 B | 5,280 B |
| Unpadded control | 71,144 B | — |
| Checked growth continuation | **69,528 B** | 5,296 B |

The matched unpadded comparison saves **1,616 B**. It eliminates the
**1,584 B** general scorer and saves **32 B** in the checked entry's
symbol extent. The other 48 B in the comparison with the published
object is existing placement padding. The stripped deployment executable
is **276,056 B**. The encoder emits **1.875× libdeflate** and
**1.205× Roc**. [Byte attribution](growth-study-size.json) ·
[Deployment link receipt](linked-deployment.json)

The exact candidate passes 162 linked output and capacity observations
and six checked-failure cases. A separate test copy of the same source
requests smaller scratch capacity to exercise real growth. With the
option on and off, it produces 162 identical observations, including
allocation traffic. The original output and alias checks pass in both.
The smaller scratch changes the allocation count relative to the original
scratch in 140 calls. [Linked contracts](linked-contracts.json) ·
[Growth exercise](small-scratch.json)

| 15-case screen | Roc wins | libdeflate wins | Natural 8 KiB vs libdeflate |
| --- | ---: | ---: | ---: |
| Forward | 15/15 | 12/15 | 1.03357× |
| Reverse | 15/15 | 12/15 | 1.03905× |

The candidate retains every peer win from the matching control in both
screens. Peak stack is 16 B above the 5,280 B bound, and natural 8 KiB
still loses in both screens. The maintained build remains the 71,192 B
frame-copy configuration. The 15-case tests establish focused performance
only. [Screen and per-case ratios](screen.json) ·
[Runtime acceptance rules](../../../qualification-policy-v3.json)

## Replay

Apply [compiler.patch.gz](compiler.patch.gz) in a working copy of this PR.
Run the documented frame-copy rebuild using the patched sources. Its
option-off encoder, bytecode and metadata match the maintained recipe.
Pass that rebuild directory to `replay.py` along with a new output
directory.

```sh
python3 -B replay.py /tmp/mech-frame-copies-71192 /tmp/mech-shared-growth
```

The LLVM-free emitter still prepares the general scorer before the
object's reachability pass omits it. An earlier selection decision would
also remove that compilation work.
