# Experimental preserved-register preference for the checked root

**Status: experimental, full 81-case qualification pending.** The maintained named recipe stays at 64,444 B and its existing 81/81 Roc / 62/81 C point wins. This option is disabled by default.

Source `6dc64cff731a7388fb1239945b3aa4eaf2ace0ce` adds `MECH_COMPACT_PREFER_PRESERVED_REGS=1` for eligible memory-backed checked roots. On Apple AArch64 it reverses only the existing preferred/nonpreferred integer register sets. The allocatable union, reserved/pinned exclusions, call clobbers, ABI preservation and register-allocation checker are unchanged. The Mech program is unchanged.

The source-built default reproduces the qualified 64,444 B object, full metadata and canonical bytecode exactly. The option emits **63,804 B**, saving **640 B** entirely from the checked entry (**47,428 → 46,788 B**). Its root frame grows **3,104 → 3,296 B** and measured controller-plus-callee stack grows **4,656 → 4,848 B**, below the unchanged 5,280 B ceiling.

All five focused native/validation tests pass, including 18 opaque call results retained across imported calls under both allocation preferences and optimization modes, exact outputs, neighboring sentinels and clearing the function attribute. Linked checks pass **162 exact output/decompression/alias observations** and **six checked allocation-failure cases** with matching origins and cleanup. Two earlier harness setup failures are retained: a stale directory binding and a reused result label. Neither was a candidate output failure.

The focused runtime comparison records **15/15 Roc and 13/15 C point wins in both orders**, matching the 64,444 B control with no lost peer cases. The unchanged v3 gate says `Unresolved` solely because both complete timing orders are required. This is not a new full-suite win claim. Full acquisition and complete build timing still need to run before promotion.

## Remaining architectural work

The original brief already had separate numeric kernels. Existing work shares three checked controller bodies and six continuation bodies across 53 selected sites. The **47,428 B qualified controller remains large**, so the small direct controller with bounded continuation regions is incomplete. The register experiment does not complete that refactor.

A static census of the exact qualified root finds 11,857 instructions, including 2,106 direct-SP `ldr`, 857 `str`, and 2,084 `mov`. Post-cleanup IR contains seven explicit slots totaling 560 B; native frame size is 3,104 B. A larger structural pilot should extract one substantial checked fallback outside an inner hot loop, using exact live-ins/outs, ownership and resume/error contracts, while retaining hot kernels and the full active-stack ceiling. Root disassembly/IR and before/after region reports are archived here; diagnostics reproduce all three qualified program artifacts exactly.

[Candidate and flags](candidate.json) · [Implementation plan](plan.md) · [Default source reproduction](control-reproduction.json) · [Linked checks](linked-checks.json) · [Focus decision](focus-screen.json) · [Raw focus replay](focus-replay.json.gz) · [Qualified-root census](qualified-root-instruction-census.json)

From the benchmark packet directory, rebuild the source compiler and regenerate the experimental object:

```sh
python3 -B rebuild.py /tmp/mech-qualified-control --variant m1-literal-frame-cleanup
python3 -B evidence/encoder-size-gap/preserved-root-registers/replay.py /tmp/mech-qualified-control /tmp/mech-preserved-registers
```

The second command reuses the freshly rebuilt compiler and checks the archived candidate object, complete metadata and bytecode. It does not claim runtime wins. [Verified source replay](source-replay.json)
