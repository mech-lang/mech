# Broad register-frequency experiment and layout repair

**The selected build remains 63,468 B, 81/81 Roc and 63/81 libdeflate.** Five
corrected-frequency/layout candidates reduce matcher instructions and reloads,
but do not improve broad elapsed time. They remain available for later
combinations; the selected recipe and its exact worker are unchanged. No new C
win or experimental full81 qualification is claimed.

## What changed

The older profile derivation mapped a VCode label to the nearest *visited*
machine head. A machine block that was not visited could therefore inherit the
count of an earlier hot block. On the current nine-case capture this affects
402–717 zero-visit VCode labels per case.

The new independent generator decodes every static AArch64 head from the actual
kernel, respects the metadata's code extent rather than trailing symbol padding,
and preserves zero visits before applying the required frequency floor. Its
complete head set matches the independently archived static shape exactly.
The three profiles use equal case shares, linear scaling to 4,096 or 65,536,
and square-root scaling to 65,536. All derive from unrounded counts. The compiler
still validates the complete VCode input, register types, constraints and clobbers.

Two repair arms retain corrected register allocation and guide machine layout
using the previous layout's ranks for identical pre-allocation VCode blocks.
This is a heuristic, not ordered-machine-graph equivalence. Every actual new head
is included once; native emission validates the profile. Capturing each new raw
VCode leaves its object, full metadata and bytecode identical. The portable
verifier independently regenerates both guides.

Original **2,719-line Mech source and canonical bytecode are unchanged**. The
compiler is the source-built version from the preceding diagnostic integration;
this study adds Python derivation and profiles, without another Rust change,
LLVM dependency, source rewrite, shipping byte patch or generic runtime dispatch.

## Results across inputs

Both 21-case discovery orders use all 24 four-actor permutations. Ratios below
are time relative to the same-run selected control; smaller is better. Every
observed Roc/C point win survives. Every slowdown and all individual zero-duration
observations remain included. Confidence and full81 qualification are not claimed.

| Candidate | Text B | Panel geometric mean, forward / reverse | Random 1 MiB, forward / reverse | Distribution change, forward / reverse |
| --- | ---: | ---: | ---: | ---: |
| Linear 4,096 | 63,612 | 1.006106 / 1.007140 | 1.006328 / 1.003945 | 1.040312 / 1.035512 |
| Linear 65,536 | 63,612 | 1.003771 / 1.005650 | 1.002356 / 1.006457 | 1.031079 / 1.032506 |
| Square root 65,536 | 63,628 | 1.005426 / 1.004606 | 1.008666 / 1.009884 | 1.007868 / 1.010841 |
| Linear, previous-layout guide | 63,612 | 1.002565 / 1.001472 | 1.002924 / 1.001723 | 1.029709 / 1.037044 |
| Square root, previous-layout guide | 63,628 | 1.002926 / 1.001768 | 1.011769 / 1.011286 | 1.002405 / 1.002757 |

Natural 4 KiB improves by up to about 1.5% in individual orders, but the intended
broader gain is absent. This is not a reason to discard the corrected counts or
partial gains. It means reload/instruction totals alone are insufficient to
select a build. Critical dependency timing and layout remain hypotheses requiring
further measurement. No hardware cycle attribution is claimed.

Actual final source-built matcher bodies, random 1 MiB, 35 calls each:

| Candidate | Instructions | Branches | Stack loads | Stack stores |
| --- | ---: | ---: | ---: | ---: |
| Selected control | 195,425,468 | 31,907,630 | 18,128,229 | 1,696,938 |
| Linear 4,096 | 188,680,624 | 31,905,634 | 12,265,612 | 1,296,017 |
| Linear 65,536 | 187,227,416 | 31,905,634 | 11,219,316 | 1,296,017 |
| Square root 65,536 | 186,764,141 | 31,905,634 | 9,871,529 | 1,699,067 |
| Linear, previous-layout guide | 187,229,416 | 31,907,634 | 11,219,316 | 1,296,017 |
| Square root, previous-layout guide | 186,766,141 | 31,907,634 | 9,871,529 | 1,699,067 |

These are untimed diagnostic block counts bound to each actual final object,
not CPU hardware counters. Coordinate-normalized objects have identical kernel
bytes and link to identical workers; address translation is checked explicitly.
All six bodies have fresh nine-case counts. Instrumented clones are excluded
from runtime and build clocks.

## Complete builds

Apple M1/macOS 15.6.1/arm64; **16 balanced serial fresh-process orders**, eight
actors. Source through final executable link, with compilers and fixed support
equally prebuilt. Tests, profiling, training, archive work, compiler bootstrap and
stripping are outside these clocks. No compilation-time gain is established.

| Actor | Median seconds |
| --- | ---: |
| Selected Mech | 2.200666 |
| Linear 4,096 | 2.198552 |
| Linear 65,536 | 2.199013 |
| Square root 65,536 | 2.202191 |
| Linear, previous-layout guide | 2.199923 |
| Square root, previous-layout guide | 2.200522 |
| Roc | 2.993788 |
| libdeflate C | 0.485272 |

Selected paired build time is **0.734807× Roc / 4.546587× C**. Its median parsing
is 0.703388 s, frontend 0.217314 s and native pipeline 1.205074 s. Complete totals
include remaining artifact/encoding/process/link work. All Mech stripped images
remain **276,072 B**; Roc is 191,912 B and C 155,752 B. Selected text is
**1.100× Roc / 1.712× C**. Each trial checks every generated artifact and executable.
All **648 actual timed stripped-executable executions** have saved streams and
pass independent inflation.

## Source reproduction and verification

```sh
python3 build_compiler.py /tmp/mech-static-compiler
python3 reproduce.py /tmp/mech-static-compiler/release/compile_profile /tmp/mech-static-normal --role linear-65536-profiled
python3 reproduce.py /tmp/mech-static-compiler/release/compile_profile /tmp/mech-static-checked --role linear-65536-profiled --checked
python3 verify.py --extract-to /tmp/mech-static-evidence
python3 ../semantic-block-map/derive_static_register_weights.py /tmp/mech-static-evidence/study/training /tmp/mech-static-derived
```

All five public normal/checked source recipes reproduce object, full metadata
and bytecode exactly. Each candidate passes strict 162-success/output/inflation/
alias/resource/cleanup checks, six checked errors, twelve injected recoveries
and **4,752 B measured peak stack**, below 5,280 B. The same compiler source has
[79 optimized scalar tests passing](../semantic-block-map/README.md).

The verifier checks every archive member, compiler source snapshots/bindings,
all native gates, both independent frequency/layout derivations, every saved
runtime call and named observed loss, all 54 actual native counter cases, every
complete-build order and every saved deployment stream. `complete-profiles` is
the authoritative static-head derivation. The earlier `profiles/derivation.json`
includes one trailing symbol-padding head; no VCode label maps there, and all
three actual compiler profile files are byte-identical between derivations.
Repeated native/build files are stored once by content hash. The verifier
reconstructs and checks every original saved path; `--extract-to` retains that
tree for independent derivation or inspection.

The next structural experiment composes arguments through jump-only blocks to
reduce copied loop state and native compilation work. It must preserve escaping
SSA uses, aliases, per-edge argument permutations, cold/debug boundaries and the
efficient checked scalar paths. Fresh profiles and the named-win checks remain
required. **64 C wins is next; 81 remains the full goal.**
