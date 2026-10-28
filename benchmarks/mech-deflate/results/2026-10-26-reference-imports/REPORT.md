# Reference import pinning review fix

The block oracle previously checked only its main source module. Imported and
transitive package modules could therefore differ from the declared revision.
The generator now verifies every tracked package source against the exact pin
before invoking the compiler, rejects missing/changed modules, and records all
14 module hashes in the reference provenance. Generated untracked applications
are excluded from the input inventory.

The focused Python qualification passes 3 tests, including mutation rejection
for the primary module, all four dependencies identified by review, an additional
transitive dependency, missing dependency rejection, and a clean package with an
untracked generated application. The real oracle rebuild passes with zero errors
and zero warnings and regenerates all three cases. Every input, scalar, shape and
buffer word in the checked-in cases remains unchanged. The exact copied fragment
and generated source hashes are unchanged. Only provenance fields are refreshed;
no reproducible-linked-binary claim is made.

The default sandbox denied the reference compiler's host cache/linker access.
The unchanged parent generator reproduces the same PermissionDenied failure.
Running the new generator with approved host access succeeds. Both failures and
the successful terminal receipt are retained; this is environmental, not an
introduced source/compiler failure.

The following collector diagnostic executes the regenerated corpus through real
Rust/native consumers in both original and decoded form. Its receipts and exact
larger-account policy belong to the block-completion report; no existing component
expected values or limits were changed by this pinning fix.
