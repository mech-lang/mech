# Pack distinct origins in shared Apple AArch64 checked exits

The static prototype uses the current 71,768 B object and preserves the text
addresses of all instructions preceding the three affected cold exit groups.
All downstream function symbols remain congruent modulo 64. It reduces the
unrounded checked root body to 50,736 B from 51,240 B and the emitted object
text to 71,256 B. This includes the three origin tables as text, their padding,
and the decoder instructions. No worker link or execution was performed.

`probe.py` reproduces the exact Mach-O object and checks each record's original
origin and branch target. `report.json` identifies the baseline and generated
object. The largest two groups have 139 and 133 sites. With a smaller
selection such as those two groups only, recompute the linked results and
placement profile from the exact source-emitted build.

## Admission

- Scope this optional flag to the compact checked root and to the existing
  Apple AArch64 MachBuffer rewrite gates. Keep the existing exclusions for
  BTI, code-CFG output, precise debug data, stack maps, exception metadata,
  unsupported relocations, jump tables, constant pools and veneers.
- Supply a specific ABI-proofed shared cold return. In
  `vendor/cranelift-codegen/src/machinst/vcode.rs`, after final branch
  optimization and before `buffer.finish`, resolve the existing
  `epilogue_target` label. Return its offset in `EmitResult` only if
  `self.abi.frame_layout().setup_area_size != 0`. AArch64's Apple frame setup
  in `isa/aarch64/abi.rs` stores FP/LR before entering the function body, and
  its epilogue restores them before returning. The AArch64 caller uses this
  witness only for the Apple calling convention. This avoids inferring saved
  LR or the return semantics from a generic branch pattern.
- Select independent two-word blocks entirely inside the recorded cold
  range. Match zero-shift `MOVZ W1/X1, unsigned_16` followed by a local
  unconditional `B`. The block has an explicit incoming branch, no
  predecessor fallthrough, no interior entry and no protected PC. A block
  after BL or BLR has possible fallthrough. An eligible predecessor has a
  terminal return/trap or a non-linking unconditional B.
- From the B target, follow only a finite acyclic sequence of status-only
  `MOVZ W0/X0` and `MOVK W0` and local unconditional B until the trusted
  epilogue's exact start. Decline paths that touch x1, call, alter SP/FP, use
  any other register or have a different exit. The old status instructions
  and all other intermediate entry points stay where they are.
- Keep candidate pairs disjoint from an existing cold-tail substitution and
  ensure their exact continuation has an offset in the resulting layout.
  Append data only after a terminal instruction or a non-linking direct B.

## Implementation sketch

Plumb a diagnostic boolean through the existing function flag pattern in
`vendor/cranelift-codegen/src/ir/function.rs` and the compact root's controls in
`hosts/gpu/src/native_artifact/program/compact/emit.rs`. Pass the optional ABI
witness from AArch64 `mod.rs` to `cold_tails/mod.rs`. Reuse that module's word
decoder, branch range checks, and single atomic commit of remapped metadata.

The prototype started from bytes after the existing identical-tail sharing.
To reproduce that order, the existing pass can return the new cold range
and epilogue address from its `moved` mapping. Parse the remapped words for the
new policy, and append data only after both scans finish. A single
transaction that reserves the existing identical tails before it selects
origin groups is also possible. Verify the actual result with the frozen
recipe because its final selection may differ from the prototype.

```text
for two-word cold entries with a valid saved-link return:
    group (old_site, origin) by old B destination

for each group:
    n = number of entries
    padded_table_words = (n + 1) / 2
    if 2*n <= n + 6 + padded_table_words: skip
    reserve the pairs so no other transformation can consume them

while copying installed code into scratch:
    omit selected pairs and retain every other old→new word mapping

for each selected group:
    first_stub = scratch.byte_offset()
    for each (old_site, origin), in stable order:
        map old_site to scratch.byte_offset()
        emit BL to the group decoder
    decoder = scratch.byte_offset()
    emit ADR  x1, first_stub + 4
    emit SUB  x1, x30, x1
    emit LSR  x1, x1, #1
    emit ADR  x30, origin_table
    emit LDRH w1, [x30, x1]
    emit B to the mapped original continuation
    emit origins as little-endian u16 and pad to 4 bytes

assert every original and new branch/ADR fits its signed range
remap existing branch targets, external relocs, traps and call metadata
commit the scratch buffer and metadata together
```

A local BL's return address is `first_stub + 4*i + 4`. The decoder's
shift computes the table's byte index exactly. The 16-bit load zero-extends
the origin, matching both original forms of MOVZ. Only x1 and the reserved
link register are written, with NZCV, frame registers, heap and owned data
preserved. The decoder does not return through x30 or introduce a real call,
safepoint or separate frame. Root teardown reloads the original saved LR and
performs its original pointer authentication. Preserve metadata for all
original instructions with the existing remapper. The inline halfwords use
the selected Darwin object's readable code mapping; the optional mode
requires that environment and excludes execute-only mappings.

## Validation

1. Construct synthetic MachBuffer groups with at least 13 distinct origins
   to exercise the benefit guard. Verify every encoded branch and table
   lookup, including a high unsigned origin and odd/even table sizes.
   Verify address-sensitive relocation, trap and call-site records survive.
   Add negative controls for a missing saved-LR witness, interior or
   fallthrough entries, wrong origin/status destination, an enlarged
   continuation with a call, unrelated frame and out-of-range branch.
2. Build the opt-in candidate from the existing recipe, and verify that
   flag-off still reproduces the exact control. Inspect the exact Mach-O
   emitted sizes and keep placement congruent with the pinned worker in
   the first screen. The checked root tail profile validates the full
   emitted length and needs a separately recorded candidate profile.
3. Run the existing checked exits, source-origin and ownership probes,
   including allocation failure and normal output/aliasing. Measure stack
   high-water and reproduce the linked worker identity from the compiler.
   Keep candidate runtime observations scoped to its exact linked worker.
   Cold-only execution leaves the warm instruction layout largely fixed,
   but branch displacement and subsequent symbol addresses still change.
   Qualify the runtime with both prescribed full orders before promotion.
