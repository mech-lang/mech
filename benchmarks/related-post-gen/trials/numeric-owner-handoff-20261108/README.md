# Native numeric-tree owner handoff (IR308)

The compiled scatter used a fresh postings buffer, but an already-dead canonical arena pin defeated Core consuming transfer at its first write. The compiler now requests the existing same-plan handoff for selected numeric trees. The runtime validates the exact source-site last-use proof and retires only matching current frozen identities after closing the loan. Core's unchanged exclusivity check still decides reuse. Live prefix aliases keep both positions copies.

Source commit: `4482c116e1ef1eba5e18dac5d84b64211152e734`. Cranelift, unchanged benchmark source, four numeric workers. IR308/work125/ABI41/image11.

| Cohort | Fastest eligible Rust | IR307 | IR308 |
|---|---|---|---|
| First | 1.548667 ms | 1.760625 ms / 1.136865x | 1.733292 ms / 1.119215x |
| Confirmation | 1.520333 ms | 1.762416 ms / 1.159230x | 1.719750 ms / 1.131167x |

Both cohorts' paired 95% intervals exclude zero versus IR307, IR305 and the identical-runtime ablation. Confirmation paired reductions: 2.318% versus IR307 and 2.584% versus ablation. IR308 becomes the selected foundation; the goal is not complete.

Confirmation resource changes versus IR307: 722 to720 allocation requests, requested bytes7,729,165 to7,326,125, additional peak requested heap5,761,199 to5,357,407, retired instructions32,594,593 to32,339,570. Aggregate cycles10,435,834 to10,559,386: do not claim a cycle win. Native code1,035,256 to1,034,972 bytes; maximum native frame1568 bytes unchanged; executable17,182,256 bytes. Whole process51.392458 ms and restoration21.898875 ms remain reported outside the gate.

The new profile records64 exact outputs and a separate same-Object 10-second stack sample over10,001 fresh invocations. Combined postings first-write helper spans fall29.250 to3.208 us, with403,040 payload bytes removed. Spans include instrumentation/ownership checks and overlap scatter; their medians are not additive production savings. Triple construction remains the largest critical-path deficit.

Qualification: 484 compiler contracts, all480 parent names retained; exact unchanged Core/Engine qualification reuse168 contracts; 176 Map,29 image/caller,28 UTF8/range/protocol and2 packed-frame checks; all133 scalar/DEFLATE records unchanged outside timing;64 linked outputs;2,294 latency and532 resource outputs. Both first and confirmation cohorts and the earlier two stale-policy assertion failures are retained.

See report.json, evidence/processing-selection.json, evidence/ownership-diagnosis.json and evidence/next-compiler-work.txt. The prototype completed-key/prefix work remains test-only on a separate branch and removes no bounds checks in this implementation.
