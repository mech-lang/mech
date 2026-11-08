# Current-cell checked increment writes

IR284 lets the numeric traversal proof retain a successful cell read until the
same current U64 cell is updated with its successfully checked old value plus
one. The mathematical index identities must agree on every incoming path, the
loaded scalar must still survive, and the checked addition must already have
succeeded. Only then does emission omit the duplicate bounds check, old-cell
load and equality branch.

The existing first-change ownership flag, private-copy boundary, store,
publication, source error ordering and finite accounting remain in place. The
optimization does not turn arbitrary stores into changes: writing zero into a
zero posting cell still takes the ordinary equality path.

Every replacement, unknown definition or opaque step forgets all remembered
cells, including reads of different potentially aliased arrays. Incompatible
merges discard the fact, and facts never cross a latch or a callback. This
first implementation deliberately declines position updates separated from
their reads by a posting write. Analysis uses the existing finite site, word,
term, partition and relation allowances; exhausting them retains checked code.

The emitted report records the write, successful read, array, index and incoming
word. The private CheckedIncrementWrites ablation exercises the independent
ordinary emission, with no alternate public backend or runtime dependency.
Tests cover current-index identities, joins, absent/overwritten reads, wrapping
arithmetic, unchanged/different-cell writes, intervening writes, aliases,
overflow, bounds failures, publication, work accounting and retry. Benchmark
results must separately establish machine-code removal, instructions, elapsed
time, allocations and artifact sizes; the estimated saving is not a timing
claim.

Fused generators can append before reading a count. Independently fresh local
scopes cover at most 32 consecutive source sites and 256 writes per numeric
region, reusing the same transfer engine. They stop at branches, joins,
mutations, array copies, loops and callbacks; no pre-boundary fact is imported.
Only checked-update facts escape these scopes. Existing loop-prefix proofs and
their allowances remain unchanged.
