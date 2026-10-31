# Measured transport profitability

Candidate source 552e537fc completes the exact eight-byte computation and focused
correctness, but its first isolated pair regresses: 4.560646→4.6861665 ms,
15.61865×→16.04852× the common 0.292 ms reference (+2.75225%). These 32-call
cohorts are preliminary, not a confirmed result, and remain committed unchanged.

Actual callbacks confirm useful native execution: admissions 13678→12450,
helpers 9156→8836, and unchanged child/grandchild visits. There are 232 additional
selected parent entries, however. Profile weights under begin_numeric_segment
increase 9→38 out of about 1330 weights. Two new pieces per path-scan instance
save only three old calls, while transporting six and one managed sources.
The existing profitability rule counts only the two new boundary callbacks.

Refine that same general rule to include the current-owner/account check for
each managed entry source. Select a piece only when saved old calls exceed two
boundary calls plus its source inventory. This is an optimization estimate, not
a program/resource limit: every declined piece retains its original execution.
There is no size cutoff, source/function name, input length, allocator change,
new operation, owner, protocol or changed price.

Static comparison identifies eight newly emitted tiny pieces across four
instances; none of the previously emitted round-38 pieces changes selection.
Recheck actual source values, Boolean/error/retry and finite boundaries, then
measure the refined executable against BOTH the frozen round-38 baseline and
the first candidate with one common fresh reference. No speed gain is claimed
until that measurement and balanced primary comparison finish.
