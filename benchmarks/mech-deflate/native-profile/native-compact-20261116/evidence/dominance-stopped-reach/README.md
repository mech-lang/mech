# Dominance proofs for identity-route reachability

The prototype uses a known dominator relation to avoid searching the graph
when a live use must be reached after the forwarding destination, or when
any route to the use must revisit the original definition. Other uses take
the existing cached search. The proof model matches the reference on
82,723 graph queries. The Rust differential probe checks 46,890 real
Cranelift dominance queries and three original reachability tests.
[Patch and proof probes](as-run/) · [Rust checks](checks/dominance-rust.log)

On Apple M1, the native phase measured **1.52426 to
1.50829 s** across eight paired builds.
Seven pairs are faster. The **0.9958%** median paired native
reduction falls just short of the predeclared 1% screen. End-to-end time
changes **0.0779%** in the paired median, with four faster orders.
[Predeclared gate](matched/plan.json) · [Matched raw trials](matched/timing.json.gz)

The encoder object, metadata, canonical bytecode, linked executable, and
timed worker reproduce exactly. The worker passes 162 exact linked calls.
[Artifact identities and compile results](report.json)
