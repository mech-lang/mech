# Allocation-window probe and the historical receipts

The current probe uses twelve boxed values of size 1,024 B and alignment
16 B. Each of the four worker threads keeps three boxes alive across the
allocation and release barriers. The tracked global allocator sees the
same allocation and deallocation layout, and the fixture no longer needs
raw allocation or pointer reads and writes.

Both optimized programs pass the original check. An instrumented repeat
checks exactly twelve successful allocations and zero frees at the
allocated barrier, then exactly twelve frees after the workers finish.
The layout and memory accounting checks pass in both implementations.
[Runtime checks](runtime-validation.json)

The historical probe and verifier are digest-pinned in the related-post-gen
receipt. The verifier checks their archived bytes from the original
publication and still checks ordinary checkout files against the original
inventory. The focused verifier exercise uses a fixture with no Git
checkout and tests missing, corrupt, changed, and out-of-packet captures.
The full historical verifier also has unrelated checkout drift.
[Focused provenance checks](provenance-verification.log)

The hosted architecture check failed on the original probe. After this
repair the complete-tree gate reaches the inherited linked-module restore
in after-4.rs, which requires a trusted same-object binding.
[Hosted failure](https://github.com/mech-lang/mech/actions/runs/44066015097/job/133980787160)
· [Local full-tree audit](full-tree-boxed.log)
