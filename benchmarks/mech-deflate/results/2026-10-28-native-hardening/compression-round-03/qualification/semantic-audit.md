# Schema summary review notes

Implementation: `ef0e5d37a47c547a7c4a28f1bfae508f4d977cb8`.
Only production file changed: `src/core/src/schema/table.rs`.

All entry-arena construction is private to that file. The builder, merged
extensions, preallocated extensions, full component closures and rooted closures
call one private constructor. It calculates the Option bound through the existing
checked no-allocation visitor. The callback API itself is unchanged. Cloning
shares immutable entries and copies the already derived Option. Public accessors
expose immutable entries/schema bodies/canonical bytes; callers cannot supply a
cached bound. Test-only collision injection mutates private entries solely to
exercise collision rejection and is not a production mutation path.

The cache covers only concrete schema layout. It includes its own inline field
through existing size_of<Table> arithmetic. It includes no current dimensions,
Values, account identity, loans, ownership, work allowance or publication state.
Existing account/alias/physical-ticket checks therefore remain live. Bytecode
encoding still uses canonical schemas and IDs; there is no new wire field.

The existing every-contribution precharge test still invokes its callback and
stops at the exact denied contribution, even after the summary is available.
The immutable memory tests retain exact/one-short transfer, rollback while caller
aliases survive, distinct retained schema owners, injected allocation failure,
recovery, and zero claims after final drop. No expected value or finite allowance
was changed to make these tests pass.

The new field can increase metadata admission and work derived from that actual
layout. This is reported separately from algorithmic work and physical payload
sizes. A higher required admission is not bypassed or hidden by subtracting the
cache's bytes. The complete-program comparison will retain both work totals.
