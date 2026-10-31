# Correction to historical pointer-role counts

The R62 audit found an error in the earlier static stack-access parser. It recognized immediate offsets written to `w16` but missed the equivalent `mov x16,#offset` form. Some later accesses were therefore attributed to a stale pointer offset. The corrected parser handles both forms and invalidates unknown writes to that register.

The [corrected ledger](pointer-role-correction-ledger.json) supersedes the following role-specific counts. It identifies the exact unchanged Objects and retains the old and corrected counts.

| Pointer role | R59 old → corrected | R60 old → corrected | R61 old → corrected | R62 corrected |
|---|---:|---:|---:|---:|
| Locals pointer | 118,456 → 95,127 | 181,076 → 148,811 | 165,897 → 139,076 | 39,933 |
| Argument-lane pointer | 15,523 → 15,352 | 31,255 → 30,966 | 31,199 → 30,900 | 15,445 |
| Argument capacity | 8,336 → 8,241 | 8,348 → 8,241 | 8,345 → 8,241 | 8,241 |

Consequently, the R59→R60 locals-pointer increase is **53,684**, superseding 62,620. Its combined locals/argument-pointer increase is **69,298**, superseding 78,364. The R60→R61 locals-pointer decrease is **9,735**, superseding 15,179.

Whole-Object opcode counts and total scalar stack loads/stores are unchanged. All measured elapsed times, retired instructions, CPU cycles and allocation results are also unchanged. This correction changes attribution among static stack slots; these counts do not represent executed instructions. Historical scorecards and original receipts remain preserved rather than silently rewritten.

R62 reduces corrected locals-pointer loads from 139,076 to 39,933 and argument-pointer loads from 30,900 to 15,445. Other register-allocation changes offset part of those reductions: the complete Object has 79,325 fewer scalar stack loads. Role deltas must not be added to whole-stack deltas or treated as independent timing improvements.

The bounded correction archive included in R62's binary-audit evidence contains the old parser, corrected parser, four passing parser regressions, exact Object hashes, corrected slot histograms and source-matched assembly. The register allocator's split-limit behavior remains an unproved causal hypothesis; this correction does not establish which allocator event occurred.
