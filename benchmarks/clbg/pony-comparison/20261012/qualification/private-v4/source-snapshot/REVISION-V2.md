The private-v1 package and proposal remain unchanged. Its official compiler
archive/native ARM image checks succeeded; both regex and reverse-complement
variants compiled and all twenty complete bounded cases passed with unchanged
pins. Pi compilation then stopped because the public gmp4pony test module uses
the old package name ponytest, while Pony 0.74 calls that standard package
pony_test. All successful records and the failed compiler diagnostic are kept.

The additive support-v2/ponytest package aliases the four current standard
types used by that imported test module: PonyTest, TestList, UnitTest and
TestHelper. The pinned current definitions provide the same constructor,
tests callback, unit-test name/apply and assert_eq interfaces. The aliases retain
their underlying types and capabilities. They contain no GMP arithmetic,
benchmark recurrence or executor, and do not run the library's test Main.

This is a concrete uncompiled compatibility proposal. The published GMP
archive and every public arithmetic/test source remain byte-for-byte intact;
only the private compilation search path will add the ordinary alias package.
No old source, binary, proposal, report or receipt is edited. Actual compiler
capability/API validation and GMP primitive/source work remain required gates.

`guarded_prepare_v2.py` is a fresh additive runner. Its metadata seal runs ten
mocked ownership checks and ten mocked reuse/alias checks under the common
mutex. `proof_reuse_v2.py` verifies the exact old proposal, complete twenty-case
ledger, unchanged original four binaries and source snapshots, old build
commands, retained official compiler archive correspondence, public package
archives, native image metadata and every original input/output/raw record.
The failed Pi compiler diagnostic is pinned as retained lineage too. A separate
immutable reuse receipt preserves the old commands, environments and origin.
The new report refers to those twenty old cases without appending or relabeling
them as new attempts. Changed source, binary, runtime or status fails reuse.

Only reviewed static source/support files enter the new source snapshot; old
toolchains, binaries and evidence remain at their original pinned paths. New
compilation adds exactly `--path <snapshot>/support-v2`. The root's static
alias approval concerns Pony 0.74.0's four ordinary `pony_test` exported types
at compiler commit `95ad18015d04f4414d0fd4f469ed6b2e9b9b9e73`. All sixteen
remaining profiles must actually build and pass their bounded complete-output
cases; the generic GMP/map/regex contracts must also run. New binaries never
inherit old output evidence merely because source text matches.

Fixture/metadata execution awaits root's exact review; a later native revision
requires approval of its sealed proposal digest. No fixture, metadata seal,
new native revision, full sample or score is launched by this draft. Production
source work, real worker/finalizer gates and a canonical LOC audit remain
required before full studies.
