# Resolve the instruction/time disagreement without changing the candidate

The first fixed cohort is 4.3876875 to 4.3887915 ms: 15.39539x to 15.39927x
the shared 0.285 ms reference. Two of four pairs and 27/64 descriptive blocks
improve; pooled time is +0.02516%. This establishes no elapsed gain.

Separate hardware counters decrease 49,005,195 to 48,881,711.5 instructions
(0.25198%), while cycles increase in that diagnostic cohort. A smaller hardware
instruction count alone does not prove lower latency.

Run one more unchanged four-pair cohort using the identical frozen workers and
binding files. Use the same fixed block/call counts, alternating orders and
isolation. Record its own fresh shared 20,000-call reference and every pair
immediately; keep both cohorts separately, including all unfavorable results.
No implementation, emitter selection, source, allocator or allowance changes
are made in response to the first timing. Both fresh samples are already
terminal before this confirmation starts.

Use all eight preparations to assess the unchanged implementation. If the
additional result remains mixed/neutral, preserve the instruction reduction as
an observation and the full patch for reconstruction, but do not label it an
elapsed optimization. This is not a positive small elapsed gain being discarded
for its size. Qualification and broad control work must not hide the outcome.
