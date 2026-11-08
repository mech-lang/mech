# Recovered scalar-home masks: IR279

Source 3ad8bb9210d322124eb5df913f5652c10336cb4c recovers the missing IR241
compiler optimization on IR278. Compiler-local SSA masks skip physical clearing
of homes already cleared since entry or the last resumable helper. Every helper
conservatively resets all bits. No runtime mask allocation, weaker admission,
accounting change, source algorithm change or scalar-entry retirement.
Cranelift stays production; LLVM remains an optional comparison.

Fresh unchanged-parent processing instructions fall 51,147,313 -> 49,520,546
(-1,626,767 / -3.18055%). Processing medians are 2.912833 -> 2.881333 ms;
20/31 paired wins, paired change -0.84666%, 95% [-1.38028%, +0.24786%].
Elapsed-time superiority remains inconclusive. Fastest eligible measured Rust
is 1.631083 ms; IR279 is 1.766515x versus IR278 1.785828x in this cohort.

Whole-process medians 37.220791 -> 37.352500 ms; paired change -0.92980%,
95% [-3.08132%, +2.00840%]. Fastest independent whole Rust is 24.964875 ms;
IR279 is 1.496202x. Restore medians are 7.736709 -> 7.733959 ms. All samples
are retained, including adverse ones. Whole-process timing is reported without
a mandatory acceptance gate. No unchanged favorable retry.

Retained for composition as a verified instruction-saving compiler candidate;
IR278 remains the working latency control. The frozen processing target remains
false. This does not claim a timing promotion or Rust parity. Do not sum
historical instruction gains across different parents.

Allocation requests/requested bytes/peak additional heap remain equal in both
phases. Post-JSON 2945 / 7919355 B / 5566127 B; restore 26837 / 12830999 B /
7003853 B. Native code 886344 -> 889768 B, cumulative stack 1536 -> 1552 B,
51 functions, metadata 4335172 B unchanged. All memory/cycle metrics are in
the reports. RSS/footprint observations are not a complete causal attribution.

899 latency + 210 separate resource outputs match Rust; 35 qualification outputs,
408 actual native CI names, 3 focused mask/finite ablations, 8 row/route tests,
27 image/caller tests, 4 explicitly executed optional LLVM checks, and 133 exact
scalar/DEFLATE non-timing/allocation rows pass. First harness count mismatch
(expected407, actual408) is retained; no production failure or timing discarded.
No full hosted-CI-green claim. No build/test/profile overlapped the cohorts.

The recovery audit covers 120 reports and 18 prioritized entries. Paired wide
clears and conditional compact table storage remain composition hypotheses;
their unfavorable standalone processing results remain recorded. Newest PR973
geometry/address staging is separate work not present in this frozen compiler.

Run python3 verify.py to independently verify records, outputs, actual test names,
bootstrap calculations, fastest Rust, resource medians and native Object pins.
