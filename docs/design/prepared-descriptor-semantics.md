# Prepared descriptor semantics and current shapes

`ResolvedValueDescriptor` certifies a resolved semantic type, its canonical
schema and a current shape. The type and schema are immutable. Their heap
trees previously acquired independent copies whenever an already prepared
descriptor was cloned or supplied with a new shape. The current compression
witness attributes 756 requests to those two paths before this change; those
families overlap binding and planning and must not be added to their parents.

The descriptor now retains one shared immutable type/schema definition and
owns its current `ShapeInstance` separately. A validated construction creates
the definition once. Cloning retains it. `with_current_shape` still calls the
same schema shape instantiator before retaining the definition, so current
parameter arity, bounds, dimension arithmetic and lifetime witnesses remain
checked. A rejected shape leaves all existing descriptors unchanged. Sharing
the definition cannot make distinct current shapes equal or authorize a
program-lifetime shape change.

This is semantic metadata, not a managed snapshot, value buffer, kernel or
execution cache. The shared definition contains no account, reservation,
mutable factory state, argument, published output or invocation permission.
Ordinary call binding, the memory planner, current value/account validation,
finite limits and atomic publication retain their existing paths. Original
conservative metadata/work envelopes remain in force. No resource price or
canonical materialization boundary changes in this patch.

The two accessors returning the type and schema become ordinary Rust methods
because their storage is shared; their read-only return types and values are
unchanged. `shape` remains a constant accessor. The descriptor's diagnostic
format keeps its previous public field names and order. Canonical schema,
shape and artifact codecs continue serializing semantic content, not the
sharing representation. All Core-dependent crates must be rebuilt together;
this is not permission to mix Rust libraries from different builds.

The acceptance witness clones a closed aggregate with short and 64 KiB labels
1, 64 and 4096 times. No requests may arise from cloning or retiring those
copies. Current-shape tests compare with independent descriptor derivation,
retain earlier shapes, reject invalid bounds, retry, and distinguish turn
from activation lifetime contracts. The unchanged complete compression call,
original/decoded components, finite failures, publication/recovery and scalar
latency controls qualify the wider execution path. Allocation counters,
hardware counters and primary timing are separate cohorts.

This removes repeated definition construction. It does not eliminate the
remaining bound-call arrays, current turn plans, canonical values or private
owner handoffs. Those require their own consumer/lifetime evidence.
