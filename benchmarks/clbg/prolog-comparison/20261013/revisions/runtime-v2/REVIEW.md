# Additive private runtime bootstrap correction

Source-only and unexecuted. All twenty Prolog mains, counted helpers, frozen
private-v1 metadata packet, its six-fixture job, and the frozen generic C3
service remain unchanged. The runtime version remains the official
SWI-Prolog 10.0.2-1 universal DMG and matching 10.0.2 source archive, with the
publisher hashes already recorded in private-v1.

Files for review are `bootstrap_runtime_v2.py`, `image_closure_v2.py`,
`fixture_runner_v2.py`, `test_image_closure_v2.py`, and this file. No runtime
version, download, image, library, syntax, fixture or source-work outcome is
claimed from authoring these files.

The first concrete command, after root reviews the actual successful Prolog
private-v1 proposal and supplies its digest, is:

```text
python3 -B revisions/runtime-v2/bootstrap_runtime_v2.py --seal --proposal-sha256 <reviewed-private-v1-proposal-SHA>
```

It requires and rechecks the completed private-v1 six-fixture receipt and
retained eleven-fixture ownership receipt before any reused code executes.
It freezes all five additive inputs and a fresh Python/unittest source
snapshot, runs exactly the following six synthetic fixtures under the common
mutex, retains discovered and started IDs plus all raw and terminal evidence,
and writes `proposals/runtime-v2.json`. No SWI command, download or disk-image
attachment occurs during this seal.

1. `test_scoped_arm64_header_and_load_command_inventory`
2. `test_dylib_install_id_is_not_a_dependency`
3. `test_executable_and_loader_anchors_differ_for_a_library`
4. `test_rpaths_require_actual_unique_private_images`
5. `test_fresh_import_rejects_cache_or_changed_source_before_execution`
6. `test_partial_attach_preserves_actual_device_entities_without_detach_authority`

The exact class-qualified IDs are fixed independently in the bootstrap and
runner. Six discovered and six started methods, zero skipped/failed/error/
expected-failure/unexpected-success outcomes, completed execution and unchanged
source pins are mandatory. Partial fixture logs/starts/receipts survive setup,
discovery, execution and final-integrity failures.

After root reviews the actual new proposal digest, the separate command is:

```text
python3 -B revisions/runtime-v2/bootstrap_runtime_v2.py --bootstrap --proposal-sha256 <reviewed-private-v1-proposal-SHA> --runtime-proposal-sha256 <reviewed-runtime-v2-proposal-SHA>
```

Both commands retain `/tmp/mech-clbg-timing.lock` for their complete lifetime.
Root owns a single durable launch of each authorized command; this packet does
not create sibling workers, retries or detach from the mutex itself.

The runtime phase creates an exclusive `.build/swi-runtime-private-v2` output.
It reads and verifies the exact original proposal and guarded-preparation
source before fresh source compilation/import. Cache/path checks cover the
entry, imported preparation and every reused service/harness module. The
additive service-loading shim repeats private-v1's retained receipt/pin gates
while permitting only explicitly pinned additive source paths; it does not
alter a module's `__file__` or modify the generic service.

The original publisher DMG and complete fat application are preserved and
pinned before a second private copy receives an exact ARM64 executable slice.
All application bytes are pinned before the version/feature probe. Native tool
bytes, actual commands, raw outputs, owned receipts, source archive, vendor
Prolog sources and private native image closure are retained. The system PATH
runtime is observed but remains unselected.

The private image inspector runs original owned `lipo -archs` and scoped
`otool -arch arm64 -hv/-L/-D/-l` commands. It correlates the ARM64 Mach header,
all numbered load commands, `-L` listing and distinct `LC_ID_DYLIB`/`-D` value.
A library's install ID is recorded as its identity. Only actual dependency
load commands enter the recursive closure. `@executable_path` uses the original
entry SWI parent for every image; `@loader_path` uses the importing image parent.
`@rpath` uses actual `LC_RPATH` contexts and inherits the loading chain. Unknown,
relative, nonexistent, externally escaping or ambiguous targets diagnose.
Ambiguous candidates are not selected by an invented basename search. OS
absolute install names are disclosed as shared-cache/system images rather
than assigned fabricated on-disk hashes. Unresolved OS rpath candidates remain
diagnostics. Every selected private image must contain ARM64; the executable
must be exactly ARM64.

This resolver is intentionally conservative about multiple existing rpath
targets: it diagnoses ambiguity rather than claiming the loader selected a
particular target without further actual loader evidence. A finite context
limit diagnoses unexpected recursive linkage. The actual publisher bundle
layout and install names are still unobserved.

Only the exact newly created requested mount can be detached. The bootstrap
preserves actual attach plist device/mount entities and raw outputs even after
failed or malformed attachment. When no exact mount is observable, it records
unresolved device closure and requires root review. It never guesses or signals
a listed device. Successful exact detach needs an actual owned exit-zero
receipt and absence of the requested mount; unknown/extra mount entities stay
unresolved. Cleanup failures preserve the preceding failure and are fatal.

All initial pinning/import/setup phases sit inside terminal failure envelopes.
Terminal capture continues over every readable artifact after a per-path
failure, retains partial pins and reports incomplete integrity with nonzero
exit. Download/bootstrap success still reports
`runtime-discovered-awaiting-root-source-library-review`, with runtime admission,
source-work admission and chart admission false and zero full processes.
The later reviewed configuration must additionally bind actual archive,
source-library/default-GC, image, feature and mount-closure evidence. No such
configuration is invented here.

Reviewer checklist:

- Confirm proposal and exact source verification precedes candidate imports;
  no cached or sibling bytecode substitutes for recorded source.
- Confirm exact original six plus generic eleven prerequisites and the six new
  actual fixture inventory, including partial failure evidence.
- Check the distinction between install ID and dependency commands and the
  fixed executable anchor across libraries in different directories.
- Check every selected target against actual LC_RPATH, symlink/root, ARM64,
  owned command, before/after byte and inherited-mutex evidence.
- Check partial attach/cleanup cannot become qualified configuration and
  terminal integrity cannot discard later readable evidence.
- Require actual bundle/runtime/source-library review before admitting a config;
  no benchmark compilation, source-work proof or timing is authorized by a seal.

Primary path semantics and command basis: [Apple's dynamic library
identification](https://developer.apple.com/forums/thread/736719), [Apple's
dynamic library design
guidelines](https://developer.apple.com/library/archive/documentation/DeveloperTools/Conceptual/DynamicLibraries/100-Articles/DynamicLibraryDesignGuidelines.html),
and [Apple's run-path dependent library
documentation](https://developer.apple.com/library/archive/documentation/DeveloperTools/Conceptual/DynamicLibraries/100-Articles/RunpathDependentLibraries.html).
These support the source design, not an asserted observation of the DMG.
