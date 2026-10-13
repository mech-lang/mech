# Existing fixture-host correction, unexecuted

The actual runtime-v3 discovery records an import failure before any of the six
fixture bodies started. The chain is `test_image_closure_v2 -> plistlib ->
xml.parsers.expat -> pyexpat.cpython-314-darwin.so`; dyld reports missing
`_XML_SetAllocTrackerActivationThreshold` in `/usr/lib/libexpat.1.dylib`.
The actual report, receipt and discovery pins are respectively
`5f301491fe64b8872e6737ef1dfd945e99a9dddc49c9a73ad44e8ca1b89423b6`,
`4ad5eb929a0377ab9004864c4a94562ee76336c1cc05ccb7ebd7714c98c30b06`,
and `d4c2daf2e3b85beeb34af65bda7b3d086c0940edf5d25aad85ec3885a925ce89`.
These literals were read from saved terminal metadata, not freshly hashed.

Installed Homebrew Python 3.14.8 source metadata declares
`--with-system-expat` and `MODULE_PYEXPAT_LDFLAGS=-lexpat`; its retained formula
uses macOS Expat. No Homebrew Expat installation was found. This establishes
the selected host extension/system-image ABI mismatch. It does not establish
which OS/SDK update introduced it. The saved discovery did not record the
inherited dyld environment, so it cannot prove whether an inherited override
also affected loading. The frozen fixture runner does not change the current
process environment. Its preparation path builds `compact.ENV` for later child
commands, rather than applying that mapping to this same-PID fixture loader.
Changing benchmark `ENV` would not be an evidenced fix for the recorded import.

The existing Codex bundled Python is locally available at
`/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python`.
Its saved build configuration identifies Python 3.12.14, deployment target 11.0,
`MODULE_BUILDTYPE=static`, and `pyexpat` among built-in modules. This is source
metadata, not an actual successful import claim. The new entry selects that
exact existing interpreter and validates version, prefix, startup flags,
actual built-in module availability and spec origin under the original common
lock. It pins the interpreter, libpython, build metadata and exact original
plistlib/XML wrappers before fixture discovery. After the same fixtures pass,
it requires actual built-in pyexpat origin, records its actual Expat version,
and checks every loaded wrapper against those selected source pins.

No v2 source, parser, bootstrap, fixture body or fixture ID changes. The entry
reuses the exact original v2 bootstrap helpers and original three fixture input
files through the pinned v3 diagnostic runner. That runner preserves actual
discovery/import failures and requires exactly the same six unique discovered
and started IDs, zero skipped/error/failure outcomes, actual normal completion,
unchanged module identities and unchanged source pins. The original six Prolog
preparation fixtures and eleven generic ownership receipts remain prerequisites.
Both actual failed stages and all their original bytes remain pinned.

The new exclusive stage is `proposal-seals/runtime-v4`; it cannot retry, resume
or replace an existing directory. The original bootstrap reads the literal
metadata path `proposals/runtime-v2.json`, which neither failed seal emitted.
After true v4 fixture success, this candidate creates `proposals/runtime-v4.json`
and a byte-identical, explicitly labeled v4 alias at that unused expected path.
Both metadata files identify `qualification_stage=runtime-v4`, the correction,
original bootstrap, both failures and the actual new fixture receipt. It does
not relabel either failed v2/v3 report as passed. Root must review this
compatibility-path choice before the single launch. A failed publication retains
an immutable failed outcome with incremental available proposal pins; success
requires a completed publication outcome and actual durable command exit zero.

All runtime/source-work/chart admission remains false. This sealer performs no
download, mount, architecture inspection, native tool or benchmark. It emits
metadata for separately reviewed runtime bootstrap; no automatic next phase is
started. It has one common mutex and inherits the existing durable owner lease.

Source-only command for root's existing durable launcher, from the Prolog cohort
directory after review:

```text
/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12 -E -S -B -u revisions/runtime-v4/seal_runtime_v4.py --seal --proposal-sha256 5b9186bb51e748fe69b16dcd35a632dd9ed8556abdbc30cbab45159a36a1bf88
```

Nothing in this packet has been hashed, imported, compiled, tested, downloaded,
or executed by its author. Root owns review, actual execution and Git.
