# Static C++ explicit member-initializer review

This unexecuted proposal implements the approved `policy-explicit-initialization-20261012.json`: constructor header 1 + each explicitly written independent member initialization 1 + written closing brace 1. Argument delimiters/formals are structural; independently written argument/body work is not hidden. No implicit, inherited or compiler-generated fields are counted.

The saved job15 measured C++ pool contains 37 distinct source identities. Bounded saved constructor-signature evidence and exact source slices identify 12 initializer-list sites across nine hashes. Hashes below are copied from the immutable audit, not recomputed. This reviewer ran only Python source syntax compilation without module imports/execution. No parser, lexer, normalization, tests, hashes, build, timing or jobs ran.

The complete written lists account for 34 omitted member units per distinct source. The bounded source/type resolver can propose eight constructor owners accounting for 25 of those units; four sites remain gated for template class or member type provenance. Registration-weighted totals are in the JSON and count each measured compiler/mode registration separately. A source may have a proved constructor and other unresolved diagnostics; none is admitted to charts by this review.

## Exact constructor sites

| Source | Line | Explicit members | Old constructor units | Approved written units | Proposal role |
|---|---:|---|---:|---:|---|
| binarytrees-gpp-1.cpp | 22 | l, r | 2 | 4 | bounded-source-owner |
| binarytrees-gpp-1.cpp | 23 | l, r | 2 | 4 | bounded-source-owner |
| binarytrees-gpp-2.cpp | 15 | l, r | 2 | 4 | bounded-source-owner |
| binarytrees-gpp-2.cpp | 16 | l, r | 2 | 4 | bounded-source-owner |
| nbody-gpp-5.cpp | 22 | x, y, z | 2 | 5 | bounded-source-owner |
| nbody-gpp-3.cpp | 19 | x, y, z, vx, vy, vz, mass | 2 | 9 | bounded-source-owner |
| fasta-gpp-1.cpp | 75 | i, size, alu | 2 | 5 | bounded-source-owner |
| fasta-gpp-1.cpp | 86 | i | 2 | 3 | gate-unproved-template-member-type |
| pidigits-gpp-4.cpp | 19 | q, r, t, k | 2 | 6 | bounded-source-owner |
| pidigits-gpp-5.cpp | 74 | q, r, t, k | 2 | 6 | gate-unproved-ulong-member-type |
| nbody-gpp-1.cpp | 27 | fst, snd | 2 | 4 | gate-template-dependent-class-owner |
| nbody-gpp-6.cpp | 28 | fst, snd | 2 | 4 | gate-template-dependent-class-owner |

`Random` writes `std::vector<IUB>& i` after its methods; this type template is not proved by the supplied bounded declaration resolver. `LFT` in pidigits-gpp-5 writes `ulong k`, absent from its explicit source/header type environment. The two nbody `tuple` definitions are dependent template specializations, outside the proven plain class/struct collector. Their written semantic counts are recorded, but the helper emits no accepted constructor owner for those unresolved lists.

## API and integration contract

`classify_cpp_initializers(cleaned, language, contexts, type_environment, *, declaration_resolver)` returns events, exact owners/evidence, diagnostics, context overrides, executable regions and canonical suppression spans. Other language labels return an empty proposal rather than aliasing another recognizer. The helper imports no lexer/normalizer/type parser and reads no files. The caller supplies its explicit `identify_decl` contract for direct member fields.

The parent integration classifies the original masked text, applies proven frames before value/field work, then makes only the constructor prefix and its opening/closing syntax opaque. It adds the proved events after canonical scanning. A missing constructor body frame is inserted in lexical order before nested argument/data/control frames. This preserves every body binding and nested owner, and avoids duplicate stale surface headers without generic diagnostic filtering.

Unknown/base/delegating/duplicate/static/conditional/template/expired ownership is gated. Argument assignments, updates, sequences and executable callbacks are also gated until independent work has its own count proof. Any initializer argument group spanning rows currently retains a narrow `cpp-wrapped-initializer-data-review`; this avoids erasing literal or braced-data wrapping without exact row evidence. Formal/header/list-between-member wrapping is admitted as structural.

## Separate function-shadow roles

- binarytrees-gpp-2.cpp:26 offset 518 `Node`: new-type-operand-grammar-parent-owned. These are outside constructor owner spans.
- binarytrees-gpp-2.cpp:27 offset 539 `Node`: new-type-operand-grammar-parent-owned. These are outside constructor owner spans.
- nbody-gpp-3.cpp:88 offset 2968 `NBodySystem`: retain-separate-ordinary-type-call-review. These are outside constructor owner spans.
- fasta-gpp-1.cpp:131 offset 2554 `Repeat`: retain-separate-ordinary-type-call-review. These are outside constructor owner spans.
- fasta-gpp-1.cpp:132 offset 2611 `Random`: retain-separate-ordinary-type-call-review. These are outside constructor owner spans.
- fasta-gpp-1.cpp:133 offset 2673 `Random`: retain-separate-ordinary-type-call-review. These are outside constructor owner spans.
- pidigits-gpp-4.cpp:19 offset 307 `LFT`: retain-separate-ordinary-type-call-review. These are outside constructor owner spans.
- pidigits-gpp-5.cpp:74 offset 1599 `LFT`: retain-separate-ordinary-type-call-review. These are outside constructor owner spans.
- pidigits-gpp-5.cpp:102 offset 2171 `Context`: retain-separate-ordinary-type-call-review. These are outside constructor owner spans.

Node gpp2 offsets 518/539 are later `new Node(...)` construction operands. The parent separately repaired that exact immediately adjacent unqualified grammar; constructor prefix ownership alone must not discharge them. Fasta Repeat/Random temporary type calls remain reviewed. Do not blanket delete function/type/formal-lifetime diagnostics merely because a source contains a proved constructor.

## Fixtures and guarded qualification

The helper has 24 focused fixture methods, covering packed/expanded lists, braced members, body visibility, post-constructor fields, modifiers/noexcept, exact commas, namespace/nested qualification, local expiry, template/conditional/static/base/delegation/duplicate/unknown gates, effects/callbacks, preserved multiline-data gates, and ordinary type-call separation. The parent separately owns seven whole-source integration methods: expected 8/10/9 totals, assignment equivalence, unknown/effect gates, ordinary same-name functions, and constructor followed by `new Node`. Existing fixture expectations were preserved.

All proposed constructor/source totals need one new frozen common-mutex full fixture pass and source audit. The static review found the parent insertion/opacity seam consistent with the declared count model; it is not a validation receipt or source/chart admission.
