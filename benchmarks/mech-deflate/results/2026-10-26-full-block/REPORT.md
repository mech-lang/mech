# Complete block-collector review checkpoint

This draft carries the actual first-block match-collection loop, including
sampling, consecutive search/skip calls, cache headers, observation and frequency
merges, near-end limits, block decisions, rewind and carryover. It follows
compiler head `faadb8539df0b05c84001709b86893967738e5fb` and keeps the reference
algorithm's 262144 U16 tree entries, 1500513 U32 cache words, depth 300 and nice
length 258. The real tree skip still has no cache argument or recording path.

The original six-position qualification and component expected values and
allowances are unchanged. New first-block source functions initialize their own
tree, cache and counters inside the compiled computation. The host test supplies
only the input and compares the returned complete state after execution.

## Executed result

The explicitly larger-budget diagnostic permits 16384 lowered blocks and one
trillion compute/execution and comparison work units. Managed memory remains
256 MiB and call temporary storage remains 128 MiB. These are diagnostic
allowances, not a default-policy or performance claim. The lowered program has
4916 blocks; its native compilation took 4.709022042 seconds. The Rust host uses
the unoptimized test profile and the generated native code uses optimization
level `speed`. Source parsing/lowering and reference comparison are outside the
reported execution interval. Execution includes collector initialization and
result publication; it is not the eventual compression timing boundary.

| Original-artifact input | Outcome | Execution seconds | Allocation requests | Cumulative requested bytes |
| --- | --- | ---: | ---: | ---: |
| Eight distinct literals | Completed, exact reference agreement | 14.000032916 | 3066805 | 1012910577 |
| Eight repeated bytes | Managed account admission rejected | 10.286718750 | 304221 | 384717606 |

The completed literal block reports 32,189,830,448 compute/execution work units,
513,951,319 comparison units, and 12,543 visits. Every scalar field and every
word of all six buffers agrees, including all zero-filled remainder cells and
the complete tree and cache. Retained account bytes are 42,780 before evaluation
and 6,615,228 after publication; dropping inputs, aliases and outputs releases
the entire account. Allocation totals are cumulative requests, not peak memory,
copy-byte measurements, or process RSS.

The overlapping case rejects at native origin node 0, block 442, local 0:

```text
Library { error: Memory(BudgetExceeded {
  operation: None, requested: 274943859, limit: 268435456
}) }
```

No output is published on rejection and the managed account returns to its
42,780-byte pre-evaluation usage. The enclosing diagnostic test is therefore
**0 passed, 1 failed, 270 filtered**, with no successful decoded run. The decoded
cases and the separate 320-byte diagnostic have not executed. The literal
subcase's completion must not be reported as a passing diagnostic suite.

The previous 10-billion-unit first-block witness remains present and fails at
`Value(Base(WorkLimit { phase: Setup, required: 10007266517,
limit: 10000000000 }))`. It is a selected budget rejection, not an algorithmic
incorrectness finding. The earlier 4096-block default rejection and the
4916-block compilation receipt are also retained. None establishes an inherent
compiler capacity boundary.

## Independent reference

`generate_block_reference.py` checks the pinned original package revision and
copies the original collector fragment through the first block boundary before
parsing/writing. It executes that reference and scans every cell of each state
buffer, storing all nonzero entries plus exact extents, scalar state and input.
No compressor implementation is called by Mech. The source revision is
`671077457ff7ef3a3d44caf715a3fc595addf379`, the reference compiler revision is
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, and the copied fragment SHA256 is
`8cbbdc954cdcdb092a85d6e88c5148207fd555f80078428b1c4885bc32b402eb`.
Reference generation completed with zero errors and zero warnings for the
eight-literal, eight-overlap and 320-overlap cases. The checked-in test compares
all words and their exact U16/U32 kinds, not a prefix or checksum alone.

## Commands and limits of coverage

```sh
cargo +nightly-2026-03-03 test --locked --offline -j1 -p mech-gpu \
  --no-default-features --features canonical-native-program \
  --test canonical_artifact_program -- \
  full_block_qualification::real_buffer_complete_eight_byte_blocks_diagnostic \
  --ignored --exact --nocapture --test-threads=1
```

The larger diagnostics are explicitly ignored during ordinary test selection.
The fixed-budget eight-literal completion test is not ignored and is currently
red. This review snapshot is intentionally draft; no full-suite or completed-CI
qualification is claimed. Formatting and whitespace checks pass. Earlier
component and compiler receipts are pinned in the preceding review branch.

Only the literal end-of-input path has completed through the full block loop.
Sampling of larger alphabets, window slides, distribution-triggered block
decisions, rewinding and carryover still require execution qualification. Their
presence in source is not evidence that those paths work. Linked execution of
this full loop is also unqualified.

## Next bounded work

Identify the selected operation at block 442, distinguish its admission bound
from actual retained backing, and reproduce that exact buffer operation in a
minimal ordinary-source witness before changing compiler behavior. Repeat the
same eight-byte overlap case after any narrow fix. A separately recorded larger
memory allowance can diagnose completion cost without changing the existing
component qualification.

Still required for `compress_into`: retaining-capacity reserve/append/clear for
U64 path nodes and U8 output; near-optimal path selection; the actual-size
Huffman integration; block and bit writing; compression orchestration and
repetition with the original allocation/timing boundaries; exact complete raw
byte agreement; independent decompression; and the full input run. There is no
complete compressor or complete-compression timing result at this checkpoint.
