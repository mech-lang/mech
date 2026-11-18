# Ordered scoring vector cursors

The diagnostic AArch64 emitter carries two admitted read addresses and four
lane indices through the long part of an ordered reduction. The existing
entry proof admits the complete initialized read windows. Each group retains
the wrapping unsigned costs, first-winner order and strict-improvement rule.
The integer address for the exhausted edge is discarded. Checked errors,
ownership, and accounting still use the existing proof and entry paths.
[Compiler patch](candidate.patch)

| Apple M1 encoder | Control | Candidate |
|---|---:|---:|
| Encoder text | 71,768 B | 71,832 B |
| Encoder text / Roc | 1.24390× | 1.24501× |
| Encoder text / libdeflate | 1.93549× | 1.93722× |
| Complete encoder object | 159,384 B | 159,448 B |
| Linked executable text | 474,692 B | 474,756 B |
| Linked executable file | 956,056 B | 956,056 B |
| Root frame | 3,728 B | 3,728 B |
| Measured peak native stack | 5,280 B | 5,280 B |

On the 8 KiB natural-text case, the composed scorer executed **164,742 fewer
instructions**, with **43,020 additional stack loads** and **3,474 additional
stack stores**. A reload of a spilled bound in the surrounding loop
accounts for the additional loads. The candidate took **1.03486×** and
**1.04053×** libdeflate's time in the two orders. Its paired change against
the Mech control was **+0.042%**, with a 90% interval of **−0.123% to +0.210%**.
The 8 KiB random-prefix case improved by 1.55% while still taking about
1.61× libdeflate's time. The instrumented scorer had zero calls in that
case, so the results do not attribute that change to the cursor loop.
[Counts and artifact identities](report.json)

Each focused order preserved every individual peer win, with **15/15 Roc**
and **10/15 libdeflate**. The validator decision was **Reject**. Empty input
slowed by **2.01%**, with a 90% paired interval of **+0.90% to +3.02%**.
One-byte and random-64 timings were unresolved. Both screens used 96 balanced
blocks per case and preserved exact Mech bytes, capacities, and aliases in
1,038,336 timed calls across all four actors. The separate checked-exit and
stack probes also passed. The 81-case runs refer to the published control.
[Validator replay](focus-replay.json) · [Checked exits and stack](checks/)

## Reproduction

Replay the original observations against the v1 policy with
`python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/ordered-vector-cursors/replay.py`.
The recorded-source package includes the release profile and allocator
configuration. Exact native object reproduction was verified on the M1 with
Rust nightly 2026-03-03.

Run the following from a separate checkout of the PR containing this evidence.
The first command obtains the frozen bytecode before applying the patch.

```sh
packet=benchmarks/mech-deflate/native-profile/native-compact-20261116
python3 "$packet/rebuild.py" /tmp/ordered-cursors-input \
  --variant fresh-owner-counts-cold-tails-zero-branches --direct-source \
  --compiler-opt-level 3 --preselect-root-inlining
git apply "$packet/evidence/ordered-vector-cursors/candidate.patch"
CARGO_BUILD_JOBS=1 CARGO_PROFILE_RELEASE_OPT_LEVEL=3 \
  CARGO_PROFILE_RELEASE_DEBUG=false CARGO_PROFILE_RELEASE_LTO=false \
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
  cargo +nightly-2026-03-03 build --locked --release \
  --config 'profile.release.package.libmimalloc-sys.opt-level=1' \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --target-dir /tmp/ordered-cursors-compiler --bin emit_compact_size
python3 "$packet/evidence/ordered-vector-cursors/reemit.py" \
  /tmp/ordered-cursors-compiler/release/emit_compact_size \
  /tmp/ordered-cursors-input/frontend/program.mecb /tmp/ordered-cursors-reemit
```
