# Rejected SSA lookup trials

Both trials are rejected and production source is restored. Neither improves
compiler work on the current level-12 Deflate program. The retained complete
source-to-executable result remains [18.646 seconds](native-scoped-source-facts-compile-profile.md);
the sub-ten-second goal remains open.

The control is the qualified scoped-source compiler, binary SHA-256
`ae517d6c58d4620b999fa43c9138ee804dda85521fffb1611717541545108d01`.
Inline-first storage starts at `972039beaa`; shared predecessor roots start at
`f19531c3b0`, which adds publication documentation. Both use the same current
Deflate/runtime, standard optimized compiler profile and 61 program flags plus
two workers. Earlier smaller-program and PGO cohorts remain separate.

## Changes and measurements

Inline-first storage keeps the first variable definition inside the existing
64-byte record, avoiding a hash-table entry until another block needs one.
It leaves SSA traversal and parameter construction unchanged. The measured
patch SHA-256 is
`81fd32351821e34dfbff2e901be4497627da49c4c8f4a7a7dbf5df37ad36ca33`.

Shared roots instead retain the original definition storage. A shared forest
compresses sealed single-predecessor chains. Lookup can skip a chain only while
every real definition names one exact Value and that Value is stored at its
root. Any SSA parameter or differing definition permanently disables the
shortcut for that variable. Ordinary lookup, cycle handling, parameter creation
and queried-block caching remain. The shortcut metadata grows with blocks and variables
separately. Its patch SHA-256 is
`9125cfb45b46d964c991003ffad6a57513b2493fba2c0dcc7399d142ebd3f0ad`.

| Measurement | Inline-first control → candidate | Shared-roots control → candidate |
| --- | ---: | ---: |
| Internal compiler phases | 18.2690 → 18.4137 s (+0.79%) | 18.2959 → 18.3655 s (+0.38%) |
| Complete source-to-Object process | 18.3257 → 18.4701 s (+0.79%) | 18.3563 → 18.4237 s (+0.37%) |
| Source CFG construction | 2.3761 → 2.4831 s (+4.50%) | 2.3375 → 2.3614 s (+1.02%) |
| Retired instructions | 275.129 → 277.881 G (+1.00%) | 275.044 → 275.763 G (+0.26%) |

Each trial has eight serial primary observations in ABBA/BAAB order. Compiler
time, source CFG time and instructions are worse in both blocks for each trial.
Maximum RSS changes -0.015% and +0.00047%; every other stage and adverse
observation remains in the [complete result](../../benchmarks/mech-deflate/native-profile/ssa-lookup-trials-apple-m3-20261115.json).
No heavy build, test, sampling, hashing or archival overlaps primary timing.
No balanced complete-executable timing cohort is run because targeted compiler
work already regresses. Cached relinks below are equivalence gates only.

## Equivalence and evidence

All 29 copied-module tests pass for inline-first storage, including the
50,000-operation ordered-map oracle, inline/dense promotion and record size.
All 30 pass for shared roots, adding later-sealed roots, cycle closure, clearing
and conservative hint invalidation. Its 160 paired CFG cases independently
verify 320 functions and compare complete IR, raw value IDs, immediate/resolved
aliases, types, definitions, queries, side effects and ordered enumeration.
They include sparse IDs, late first uses, incremental/reverse sealing, changed
definitions, mixed types, joins and unreachable loops.

Each trial also passes fourteen fixture pairs, every primary source/bytecode/
Object/complete-metadata byte gate and two cached whole-executable gates. Every
gated executable retains SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
No grammar/language change, weaker fresh proof, scalar retirement or new
emitted-program runtime campaign is claimed. Source and the qualified canonical
compiler tool are restored; rejected dependency caches require a fresh rebuild.

The thirty-second [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains both complete source/patches, commands/logs, tests, all observations,
program/executable bytes and fixed native inputs. The shared-roots first screen
completes compilation with matching bytes but its counter wrapper fails on a
sandbox-denied `sysctl kern.clockrate`; that attempt remains outside primary
means. The unchanged trial then completes with benchmark counter permission.
Compiler/test executables and dependency libraries are excluded and hash-pinned.
Fresh extraction verifies every manifest entry and Git blob, then pure offline
replay applies both patches and reconstructs all means/blocks and byte gates.
