# Nim full-index metadata correction

Source-only additive sealer. Frozen `revisions/work-full-v5` and every scored
source, binary, work proof, fixture, native timing controller and launcher remain
unchanged. This file copies the original v5 sealer and changes only:

- The emitted `controller_revision` from the stale `work-full-v3` label to the
  actual imported `work-full-v5` source path.
- The emitted `exact_controller_fixture_methods` from12 to14, matching the
  actual passed work-v5 fixture receipt that the unchanged validator consumes.
- Explicit imports and evidence/review paths back to the original preserved
  v5 packet, plus this new sealer/review and original sealer as input evidence.

No new native method, fixture, qualification, algorithm, build, full controller
or launcher is introduced. The imported module paths are checked against the
exact original packet. The original v5 validation still requires actual14
fixture methods/zero outcomes, all20 original sources/binaries and154 retained
production cases, the unchanged24 primitive/ORC/map/Pi proofs, eight fresh
tree witnesses, scored allocation and full source reviews. The failed work-v4
14-method receipt remains failed and preserved. Root owns actual under-lock
hashing/review and durable launch; none ran while this correction was authored.

The proposed command from `nim-comparison/20261012`, after root's manual review
of the actual saved work-v5 report and original scored allocation evidence, is:

```text
/opt/homebrew/opt/python@3.14/bin/python3.14 -B revisions/full-seal-v6/seal_full_studies.py --work-revision work-v5 --study-revision full-v6
```

It creates fresh `studies/full-v6/index.json` and ten manifests under the common
mutex. It authorizes no native timing by itself. Root must review that actual
index and its emitted digest, then may use the unchanged original launcher:

```text
/opt/homebrew/opt/python@3.14/bin/python3.14 -B revisions/work-full-v5/launch_all.py --revision full-v6 --index-sha256 <root-reviewed-actual-index-SHA>
```

The launcher and timing controller execute from their actual original v5 paths,
which each manifest pins. The existing strict twenty native gates, disjoint
ordinary source-failure isolation, three rotated rounds,90 fresh processes,
current unchanged Mech controls and null LOC pending a canonical overlay are
retained. Successful old records are not rerun by this metadata correction.

Saved evidence read for the manual review (no hashes were recomputed):

- `work-qualification/work-v5/report.json`: status
  `source-work-allocation-passed-awaiting-full-review-and-LOC`;20 source reviews,
  eight qualified tree attempts, all20 native gates true; no full runs/manifests.
- `work-qualification/work-v5/fixtures/actual-inventory.json`:14 discovered/run
  methods, successful true and zero failures/errors/skips/expected outcomes.
- Each tree profile's `scored-creator.c`, `scored-allocation-review.json` and
  `scored-binary/disassembly.stdout/.json` bind the unchanged scored source,
  binary and original generated C separately from the observer build.
- Compact creator `_make__compact_u7` allocates at `0x10000af8c`, before its
  depth/leaf branch at `0x10000af98`; real recursive calls are at
  `0x10000afa0` and `0x10000b01c`. Its sole preallocation exception test targets
  `0x10000b0c8`, a literal-NULL-only epilogue.
- Optimized `_make__optimized_u17` allocates at `0x10000a804`, before its depth
  branch at `0x10000a810`; recursive calls are at `0x10000a818` and
  `0x10000a894`. The preallocation error target `0x10000a930` returns literal
  NULL. Every other reachable return follows allocation in both creators.
- Original Nim-generated C initializes result to NIL, reads `nimErrorFlag`,
  takes only the corresponding pending-error preamble before `nimNewObj`, then
  tests actual depth and assigns both real children. `_nimNewObj`'s saved ARM
  calls `rawAlloc` and zeroes the resulting object; it does not return a shared
  leaf literal. This path is C-to-Clang22.1.8/O3/strict-FP to final native ARM.
  No standalone Nim LLVM IR artifact was found or claimed in this inspection;
  original generated C plus final scored ARM is the actual allocation evidence.
- Tree observations at N0/N4 each reconcile83 roots/4398 allocated nodes/2240
  leaves; N7 reconciles163/8798/4480; N9 reconciles675/51550/26112, for both forms.
  Compact has one worker; optimized has at most eight, with own iteration labels,
  joined ranges and concurrently retained/churn graph disjointness reconciled.
- `qualification/private-v7/report.json`:20 original profiles and154 prior bounded
  records;24 source proofs comprise three retained v5 primitive ABI checks, one
  retained v6 ASAN ORC ownership check, six ASAN complete-map witnesses and14
  actual Pi candidate/division traces. All proofs passed; retained identities
  are disclosed separately from the20 fresh map/Pi attempts.

These source reads support root's allocation/lineage review. They do not replace
the sealer's actual under-lock pin verification or the remaining full outputs,
samples, canonical source counts and publication metadata qualifications.
