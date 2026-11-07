# Borrowed parent-frame metadata

IR253 carries the existing private frame reader through the static composition
inventory, including each parent's segment/frame plan. Genuine process-lifetime
compiled modules select `CompiledFrame`; independent and ordinary object-bound
images retain owned `FrameWords` and their existing rederivation.

Parent frames previously decoded their compiler-only field and scalar-path
vectors eagerly even when root numeric frames borrowed the same immutable
metadata. The current allocation trace records these remaining parent-frame
allocations during restoration. No runtime consumer needs those compiler fields;
arrays, array bindings, arity and node counts remain eagerly available.

The wire vocabulary, field inventory and public serialization remain exact.
Structural validation still visits every compiler-field/path item without a
retained vector. Unknown fields and malformed nested values are rejected at
every wire level. The stronger reader is selected only through the existing
private same-Object compiled-module witness, never a serialized flag, lifetime
claim or caller-supplied digest. Lazy compiler-field access remains available for
inspection and public serialization.

Tests cover original/decoded genuine Object metadata, actual parent frames,
runtime-lane reads without materialization, exact public-image roundtrip and
malformed fields in both JSON and native metadata. Finite source accounting,
ownership, publication and scalar native emission retain their existing tests.
Performance requires its own frozen build and matched cohort; allocation counts
alone do not establish a speed gain.
