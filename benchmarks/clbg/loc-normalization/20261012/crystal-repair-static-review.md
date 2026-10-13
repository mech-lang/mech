# Crystal repair review

This note records static review of the two Crystal failures reported by the
actual immutable `future-minima-fixtures-01` job: 129 methods ran, 127 passed,
two Crystal methods failed, and there were no test errors. It is not an
execution receipt, a qualified source count, or chart admission. The frozen
failed job is unchanged. No imports, lexing, normalization, tests, hashing,
builds, probes, Git operations, or timing interruption ran for this review.

The postfix-control repair gives `_header_end` an explicit `postfix` mode. Its
span stops before a written `}` or a non-receiver `end`, `else`, `elsif`,
`rescue`, or `ensure`. Those delimiters belong to their original executable
owner. A member such as `match.end` stays in the condition because the same
receiver-context proof excludes it from keyword-close treatment. Existing
counts in the postfix fixture remain unchanged; packed and expanded callback
and method contrasts now exercise these boundaries.

The property repair identifies an unproved `property {x,y}` declaration as
data plus an explicit property-declaration review diagnostic. It no longer
turns that malformed declaration into an executable callback and hides the
declaration's gate. The initial repair also matched `receiver.property { ... }`
and equivalent getter/setter receiver calls. This review added the existing
receiver-context guard to the declaration test, with receiver-callback and
malformed property/getter/setter contrasts in the same fixture method. These
ordinary receiver calls keep callback header, body, and written-close units;
the repair does not infer generated fields or accessors.

The current application forms relevant to this distinction are typed
`property left/right : Node?` in the two binarytrees sources, seven separate
`property` names in optimized nbody, and `def self.last=(x)` in optimized fasta.
Their constructor formals, declared property names, and named setter header
have different owners. The current application text contains no property,
getter, or setter brace form. These are source-form observations from mutable
preparation inputs, not proof of a pinned qualified revision or an assigned
LOC total.

`foreign_crystal.py` and `test_foreign_crystal.py` remain unhooked. The fixture
file still has 33 methods; added variants do not weaken the original expected
counts or remove a gate. A fresh immutable guarded job must execute these
revised fixtures before exact-source replay and pin review can support a
display overlay.
