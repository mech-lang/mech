# Replay the selected variable definition

For a sealed one-shot parse, a winning variable-definition trial can save a
bounded event recipe. The final choice checks the source and parser context,
first raw origin, resource headroom and fresh identity ranges before replay.
The recipe reproduces the event and cache publications with freshly assigned
identities, including the allocation holes and logical work. The original
parser executes when the proof declines.
[Proof and measured fallback coverage](PROOF.md)

The hardened standalone parser took **1.30289 s** versus **1.87625 s**
in six balanced pairs. The capture-only build took **1.95935 s**.
The full-source comparison reproduced the raw events and cache-publication
digest, allocation cursors, tree and indices, diagnostics, work, fuel and
grants. It applied all 3,128 recipes on the pinned source. The finite,
segmented, and active-witness checks and 535 existing syntax tests pass.
[Standalone exact tests, six orders and prior variant](standalone-evidence.tar.gz)

| Eight matched Apple M1 builds | Mech control | Mech replay | Roc | libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Source through final deployment link | 3.70961 s | **3.13566 s** | 2.98380 s | 0.48054 s |
| Source through common strip | 3.72013 s | 3.14623 s | 2.99426 s | 0.49110 s |

The paired complete build improves by **15.39%** and is
**1.052× Roc** and **6.530× libdeflate** time. Both
Mech builds use the same prebuilt deployment support and the same native
compiler recipe. All 16 timed Mech builds reproduce the exact bytecode,
71,768-byte encoder, complete metadata and the same stripped deployment file.
The benchmark worker also reproduces and passes 162 exact calls.
[Source build plan](source-through-link-plan.json)
· [All observations](source-through-link-observations.json.gz)
· [Artifact and RSS summary](summary.json)
· [Fixed historical worker](worker.json)

An earlier attempt reused the control compiler because the copied source
timestamps preceded the control build. It was interrupted. The reported
runs use independently rebuilt compilers and verify their distinct hashes
before timing. [Excluded cached-control attempt](excluded-cached-control.json.gz)

| Mech compiler phase | Control | Replay |
| --- | ---: | ---: |
| parse | 1.88346 s | 1.31521 s |
| catalog | 0.05980 s | 0.05893 s |
| frontend | 0.22809 s | 0.22741 s |
| artifact | 0.00699 s | 0.00698 s |
| encode | 0.00261 s | 0.00263 s |
| native | 1.46348 s | 1.46392 s |
