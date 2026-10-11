# Pi extraction audit

This audit reviews all 60 current explorer entries (52 distinct application sources). The established protocol permits the published Rust #4 impossible-candidate guard, but every tested candidate must compute both exact extract(3) and extract(4) divisions.

Only the published Node.js #2 candidate used a one-division remainder surrogate. Its original source, retired timings, and rejection are retained in audit.json. The corrected source restores the second BigInt division and expands two packed declarations to 49 normalized LOC. Six small cases compare complete bytes to both its upstream source and an independent GMP reference. No prior sample is reused.

run.py measures three canonical N=10000 fresh-process samples of the corrected source and an unchanged Mech AOT control, using the common timing mutex and power/compiler guards. Startup, complete output, and teardown are included for both. results/timings.json pins the runner, source/runtime/control, full output reference, and imported guards. The original control audit was copied byte-for-byte to evidence/control-audit-snapshot.json before the separate R/APL/Uiua harness refreeze; its recorded hash remains inspectable after that cohort advances.
