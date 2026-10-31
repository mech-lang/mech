# Lent selector geometry — withdrawn before timing

Source `74df5a9a4921fe724e08cb13fee1116383a05ff8` attempted to retain the paid selector borrow while lending disjoint geometry to coexistence measurement. The optimized unit build failed with E0425 (missing work-budget type import) and E0433 (the physical `line_calls.rs` file belongs to the private `runtime::lines` module, not `runtime::invocation`). These are introduced errors in this unmeasured candidate. No tests, timings or performance gain are claimed. Exact command, configuration, stderr and full source patch are archived.

Withdrawn in favor of the requested exact record-admission priority. All candidate production/test/workflow changes from rounds 44–45 are restored to the previously qualified `b12f6c5afb2c859f6cde29ba566ddf09e70d2f79`; the recorded evidence remains. Hash preparation and the measured pack-handle improvement remain retained. No CI was triggered for this candidate.
