# Fixed integer interval contract

Fixed integer intervals are the supported G12 constrained scalar form. A kind such as
`u8:1..10` denotes the values 1 through 9. The lower endpoint is included;
`..` excludes the upper endpoint and `..=` includes it. Equal endpoints are
valid only with `..=`. Descending and empty intervals are invalid.

This contract does not include arbitrary
scalar predicates, dynamic bounds, stepped membership, implicit interval
subtyping, or arithmetic result-bound inference; those need separate contracts.

The supported domains are the exact signed and unsigned integer widths 8, 16,
32, 64, and 128. Both endpoints must be closed decimal integer literals that
fit the declared width. Mixed-width endpoints, fractions, floating point,
complex, rational, NaN, infinity, and all noninteger scalar domains are
rejected at the range annotation. Three-operand ranges and their step semantics
are deliberately unsupported in this first form; they receive a positioned
diagnostic rather than an inferred interval meaning.

Endpoints are evaluated once at source admission. Names, effects, captures,
activation-dependent values, and turn-dependent values are not admitted as
endpoints. A constrained kind has distinct kind and schema identity. Its
canonical schema and reified-kind encodings include signedness, width, both
endpoints, and the upper-inclusion flag. Artifact transport must retain that
identity and revalidate values after decoding or rebinding.

An exact-base integer constant can enter its interval only after a membership
check. A live plain integer cannot be implicitly narrowed to an interval.
An interval value can be rebound to its exact base kind by a checked lower
layer, but source typing does not implicitly widen it. Different intervals
do not implicitly subtype one another, even when one contains the other.
Aggregate members and checked mutable or external snapshots are checked by the
schema-directed snapshot finalizer. Initialized fixed-width interval regions
are separately checked against their exact integer bounds under retained
storage authority before publication; initialization and shape alone are not
membership evidence. An out-of-range value must fail without publishing a
partial state change.

The following paired outcomes are the minimum acceptance evidence: lower and
last admitted value succeed; the value below lower and the excluded upper fail;
the included upper succeeds with `..=`; invalid width, empty interval,
unsupported domain, dynamic endpoint, and stepped expression fail at the
source location; reified identity, aggregate members, external input, state
update, and bytecode roundtrip retain the same interval and rejection behavior.

## Semantic capabilities and templates

Valid intervals are Equatable, Keyable, and Ordered whether constructed directly
or derived from a schema. They do not gain Number, arithmetic promotion, or
range-endpoint capabilities. Structural child evidence uses the same classifier;
table and map roots remain non-keyable, and aggregates do not gain scalar ordering.
Comparison accepts equal interval identities and preserves exact integer values.
Mixed intervals and implicit base-integer conversions are rejected.

Output templates require exact equality between the resolved interval and its
schema: signedness, width, bounds, and upper inclusion all participate. Recursive
aggregate materialization preserves that identity and existing dimension witnesses.

## Candidate publication

Single and batch readiness validate initialized interval candidates before locking
cells or entering the infallible commit gate. Validation reads exact typed views;
it does not construct a canonical payload copy. Matrix membership uses logical
coordinates, not padding or spare capacity, and reports row-major member indices.
Empty geometry requires no scan of an empty large axis.

A read lease remains held from membership validation through commit or abort so
writers and host-arena projections cannot invalidate the checked bytes. Required
in-place candidates use their matching retained exclusive undo lease. Refusal
retains rollback authority and leaves accepted values, aliases, schemas, shapes,
and publication revisions unchanged. A later valid candidate can use the same
cell or activated runtime instance.

## Positional selectors and transport

Managed positional-index binding selects physical integer backing from the
interval base width while retaining the exact input schema, shape, identity, and
publication authority. One-based portable index limits still apply: zero, negative
members, or overflow may belong to the interval while being invalid selectors.
A late invalid matrix member must reject without committing any output prefix.

Checked host conversion validates membership before admitting a base value.
Unconverted live base snapshots cannot implicitly narrow. Source and decoded
artifacts preserve scalar and aggregate identity, exact 64/128-bit values, and
same-instance rejection and retry. The core type, snapshot, managed publication,
registered selector, and public runtime suites own these contracts.
