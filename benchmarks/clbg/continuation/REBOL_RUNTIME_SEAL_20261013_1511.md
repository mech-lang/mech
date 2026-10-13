# Rebol runtime preparation queued, 2026-10-13 15:11 UTC

Root read all six runtime-v1 files and the two research/capability prerequisites.
Pony independently accepted the source after the prospective private-directory
gate was moved before creation and its same-method regression was added. The
six runtime files and both notes are frozen. The existing ten-main/five-helper
baseline packet had already been independently reviewed and frozen.

Durable job `rebol-runtime6-fixture-seal-v1`, worker40175, now waits on the common
timing mutex. Command uses the existing bundled Python3.12 with -E -S -B -u
and invokes only `bootstrap_runtime.py --seal`. It requires exactly six actual
discovered/started fixture IDs, zero unsuccessful outcomes and the unchanged
actual eleven C3 ownership fixture receipts. This stage downloads nothing,
starts no native command, generates no headers and runs no benchmark.

An actual passed seal and its emitted proposal hash must be reviewed before
the separate private publisher-verified runtime discovery. The runtime, SDK,
general extension ABI/build/load/byte IO/formatting/GC/work, canonical/formatted
LOC and full timings remain unqualified. No Rebol dataset/Site entry is admitted.

The next source task is the exact tagged SDK-generation/primitive-build entry,
consuming future actual runtime/source pins and isolated vendor-generation
copies. V remains report-only; Red has no active route.
