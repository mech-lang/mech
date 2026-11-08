# Shared call metadata during native admission

The current related-post allocation trace contains 381 allocations under repeated
BoundCall clones in memory planning. This change shares an immutable operation
certificate and a separately owned current-port bundle. Cloning either retains
its checked descriptors rather than recursively copying schema/kind/evidence
vectors. Fresh descriptor refreshes still validate every type/lifetime contract
and own the new current shape witnesses. Contract resolution uses copy-on-write
so retained calls remain independent, including rejected descriptor refresh and retry.

Every existing shape, storage, alias, memory-plan and finite admission calculation
still executes. This does not cache live admission or add a runtime algorithm.
The unchanged Debug vocabulary and value equality remain authoritative. The
Rust metadata getters now borrow shared storage and are ordinary functions
rather than const functions. Mech semantics and constructor interfaces stay the
same. Production remains Cranelift. IR283/work116/ABI41/image11 provisionally;
all scalar/DEFLATE work and physical allocations must be measured explicitly.

Operation identity validation previously cloned the full canonical name and
contract solely to call the consuming constructor's identity check. Both paths
now use the same borrowed name/ID check. This removes temporary allocations
without changing which fields validation checks. Memory-plan contract, shape,
alias, current facts and finite admission checks continue at their original
boundaries. The first added fixture mistakenly expected identity validation to
reject contract arity; that failed setup is retained and the corrected test
checks the actual activation-lifetime refresh rejection and copy independence.
