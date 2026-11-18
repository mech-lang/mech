# Forward empty checked-source jumps

The experimental compiler forwards **1,837** empty jumps before native SSA
construction and removes **1,568** pre-layout blocks. It forwards only
reachable forward jumps with no action, scalar effect, buffer result, owner
transfer, or retirement. Entrypoints used by generated routes and checked
controller boundaries keep their block identities. The emitter verified
all 24 controller windows and 16 continuation windows after remapping.
[Compiler patch](compiler.patch.gz) · [Source and patch pins](source.json) ·
[Block census](source-graph.json)

The branch uses the original MechDeflate source and emits identical
canonical bytecode. The checked root accounts for the entire text change.
The other 27 symbols retain identical bytes and relocations.
The forwarded encoder emits **67,656 B**,
**192 B** more than the control. Both produce a **276,056 B** stripped
deployment executable. Linked checks matched **162** successful
observations and **six** checked error exits. Measured peak stack drops to
**4,672 B** from the maintained encoder's **5,280 B**.
[Artifact identities](candidate.json) · [Linked checks](linked-checks.json) ·
[Stack measurements](stack.json)

## Compilation

Ten balanced orders on Apple M1, macOS 15.6.1, measured fresh source
through final executable link with fixed runtime support prebuilt. Stripping
was timed separately. Forwarding saved **40.6 ms** by median paired
difference and was faster in **10/10** paired trials. Median native
compilation moved from **1.344 s to 1.297 s**. The forwarded
encoder builds in **0.798× Roc** and **4.953× libdeflate** time.

| Build | Source through link | Encoder text | Stripped executable |
| --- | ---: | ---: | ---: |
| Mech forwarding disabled | 2.427 s | 67,464 B | 276,056 B |
| Mech forwarding enabled | 2.387 s | 67,656 B | 276,056 B |
| Mech free wrapper | 2.447 s | 67,992 B | 276,056 B |
| Roc | 2.990 s | 57,696 B | 191,912 B |
| libdeflate | 0.482 s | 37,080 B | 155,752 B |

[Build and phase medians](deployment.json) · [Pinned build plan](build-plan.json.gz) ·
[Every paired build receipt](build-comparison.json.gz)

## Compression runtime

The two complete orders each acquired all 81 fixture cases. The focused
orders each covered 15 cases around close peers. Natural 8 KiB is the
candidate's paired compression time divided by libdeflate's.
These results fall short of the runtime qualification policy.

| Scope and order | Roc wins | libdeflate wins | Natural 8 KiB |
| --- | ---: | ---: | ---: |
| Focused forward | 15/15 | 13/15 | 0.99882× |
| Focused reverse | 15/15 | 12/15 | 1.00649× |
| Complete forward | 81/81 | 61/81 | 1.00468× |
| Complete reverse | 81/81 | 61/81 | 1.00497× |
| Focused free wrapper forward | 15/15 | 13/15 | 0.99944× |
| Focused free wrapper reverse | 15/15 | 12/15 | 1.00520× |

[Complete decisions and loss sets](full-screen.json) ·
[Forward complete observations](full-forward-observations.tar) ·
[Reverse complete observations](full-reverse-observations.tar) ·
[Runtime policy](../../../qualification-policy-v3.json)

## Register-preserving release calls

A separate compiler experiment shares the checked release interface
with register-preserving wrappers. The free-only wrapper adds **528 B**
and lowers measured peak stack to **4,624 B**. Sharing all six wrappers
adds **3,968 B** and peaks at **4,672 B**. Each wrapper occupies **384 B**.
Both variants passed the linked checked exits. The focused results
and matched compile time show the trade-off for the free-only variant.
[Release compiler patch](release-calls/compiler.patch.gz) ·
[Size receipts](release-calls/size-results.json) ·
[Focused results](release-calls/free/focus-screen.json)

## Replay

Apply one compiler patch to its pinned parent in an isolated checkout.
Rebuild with the patched source from that checkout. Run the corresponding
replay script to verify emitted object, canonical bytecode and metadata.
The two patches are independent. The release-call recipes are in
`release-calls/free` and `release-calls/all`.

~~~sh
# From the repository root at 8f0ffe737b94229132fd4abe83eb16a60a7619d3,
# set study to the evidence directory from this commit.
gzip -dc "$study/compiler.patch.gz" | git apply
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-source-route-base --variant frame-copies --compiler-opt-level 3 \
  --direct-source --preselect-root-inlining --skip-refused-general-ssa \
  --target-dir /tmp/mech-source-route-target
python3 -B "$study/replay.py" /tmp/mech-source-route-base /tmp/mech-source-route-on
python3 -B "$study/control/replay.py" /tmp/mech-source-route-base /tmp/mech-source-route-off
~~~

[Verified output hashes](verified-replays.json) ·
[Compiler build receipt](compiler-build.json)
