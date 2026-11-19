# Dormant preview before build-feature isolation

This eight-balanced-build campaign compares the prior selected compiler with
the new default source before the experimental preview was gated. All emitted
artifacts are exact. Source-through-final-link medians are 2.207502 s prior,
2.212215 s new, 2.977011 s Roc and 0.481022 s C. The paired cost is +4.325 ms
(90% interval +0.808 to +18.653 ms; 2/8 faster). No new optimization gain is
claimed. This prompted isolating the planning preview behind the explicit
`early-region-planning` Cargo feature and rechecking the ordinary build. Raw
observations, identities and code remain here as adverse evidence.
