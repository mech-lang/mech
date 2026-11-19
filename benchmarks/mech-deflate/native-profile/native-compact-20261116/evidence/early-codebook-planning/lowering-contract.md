# Next lowering step: physical units with one source ownership model

The typed source preview now identifies a 22-word capture packet and a six-word
normal result after constant elision and correct incoming-read liveness. The
existing all-path initializer accepts that boundary. All nine iterations and
both builders are contained in the region. These facts still do not authorize
native early emission.

Use the existing global source ownership/guard/transfer facts once and share
them across the root and helper. The boundary is a physical compilation unit,
not a new language-level function invocation. Re-deriving ownership from only
the helper's local liveness would change last-use decisions. Applying ordinary
function-entry retain/return-retain conventions would add heap operations,
possibly change copy-on-write decisions or failure ordering, and lose the
already qualified boundary's behavior.

The early helper should emit each original operation and its original
retire-after/retire-on-edge effects exactly once, with the existing source
origin. Its input words transport current source state; the synthetic root
call and normal return should not add ownership effects. The root resumes at
the original continuation with the helper's live output words. A helper error
must transport the original checked status/origin without additional source
operations. Existing global failure cleanup must remain correct for partial
work and allocation refusal.

Before implementation, derive word-level capture requirements from those
shared ownership facts as well as source uses/defs. Retirement may read an
owner lane even when its full typed value is not a source operand. Such lanes
need explicit current-state transport and the existing zero initialization;
they must not cause uninitialized descriptor fields to be captured. The new
value-level preview alone is insufficient to supply that proof. Conservatively
include these words, then narrow only with proven initialization and effects.

Give each emitted unit local word homes, source blocks and SSA IDs. Root edges
must refer to a synthetic coarse call for the omitted region; helper normal
edges return at its original exit. Builder predecessor/sealing counts use the
unit CFG, while semantic ownership/guard facts keep original source IDs through
explicit maps. Internal cursor and builder variables stay local because their
complete typed scopes are owned by this region. Do not create its internal
root SSA, even as a throwaway first attempt.

Numeric helpers remain shared module functions built once. Root-specific
native block/layout profiles require remapping or regeneration when block IDs
change; do not preserve old IDs by constructing empty region blocks and then
claim fewer blocks were constructed. Preserve the current local backend
policies without silently applying a root-numbered layout profile to a helper.
Measure every body, stub, packet and alignment byte, all-unit DFG work, complete
build time/RSS, and the actual new worker's both-order runtime results and stack.

This contract is the next implementation task. Shared source-fact transport,
word capture, unit CFG lowering, failure handling and ABI/frame admission have
not yet been implemented. The selected result stays the exact 63,804 B worker
and its existing 81/62 qualification.
