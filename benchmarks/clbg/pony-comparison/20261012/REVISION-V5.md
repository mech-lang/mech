This unqueued source-only revision fixes two shift argument types in optimized
K-nucleotide. Ponyv4 passed all12 fresh compact-K outputs, then optimized-K
compilation stopped with255: the U64 receivers' shift parameters require U64,
while `shift` and `2*_width` had type USize. No optimized-K binary/output proof
exists. All v4 sources, failed diagnostics and compact results stay unchanged.

The fresh `revisions/shift-types-v5` source adds only `.u64()` to those two
bounded dynamic shift amounts. Valid widths are1/2/3/4/6/12/18, so mask shifts
are at most36 and decode shifts at most34. The casts preserve the exact value
and all original key/mask bits. Constant shifts and every other operation,
loop, map, window visit, actor capability, join and output statement are exact.

`profiles-v5.json` changes only that optimized-K source/ID and retains its
original and v4 parent identifiers. The original12 already qualified profiles
and98 records plus v4 compact-K binary/12 records give13 exact binaries and110
production records to reconcile and reference, with no repeats or relabelling.
The seven remaining profiles (new optimized K, both trees/nbody/Fann), generic
primitives and production work/worker/LOC proofs still need actual execution.

No new guard/proposal, mocked test, compiler or native phase has launched for
v5. A future additive guard must verify the exact retained source/binary/tool,
commands, libraries, observations and each old record before reusing them.
Root reviews that concrete scope/proposal before one bounded continuation;
full studies remain separately reviewed. No timing gain or score is claimed.
