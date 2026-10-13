# Private SWI runtime candidate, unexecuted

bootstrap_runtime.py and tools/runtime_probe.pl are source-only. No download,
mount, copy, slice, version probe, library import or hash has run. This plan makes
no global Homebrew, /Applications, PATH or system installation change. It must
be frozen with the source candidate before metadata seal/root digest review.

The official [stable page](https://www.swi-prolog.org/download/stable?show=all)
describes10.0.2-1 as a relocatable universal app with commandline tools under
Contents/MacOS. It publishes34,301,763 bytes and SHA256
`bf775f0b8d7880f4908dee513316013ef42a73793be392814fde2a0a8e9ddc5d`.
The exact [source archive index](https://www.swi-prolog.org/download/stable/src/)
lists swipl-10.0.2.tar.gz; the stable page publishes12,825,701 bytes and SHA256
`e42cc098f7b8a6051c4f79a99b55162d467098aba60f69649bdc7583f0734b57`.
These become local verification only after the future guarded archive checks.
The docs' GCC15 statement is publisher context; no local compiler receipt or
native generated-application backend is inferred from it.

Proposed later command, after actual metadata proposal/root review:

```
python3 -B drafts/private-v1/bootstrap_runtime.py --bootstrap --proposal-sha256 <actual-root-reviewed-SHA>
```

Root launches it once using the durable wrapper. It takes the common mutex
before discovery/hash/import/download/native operations and keeps it until all
owned commands, exact mount cleanup and terminal partial pins are saved. The
frozen generic service owns every new Popen/PGID and bounded cleanup/drain.
Existing PATH swipl/version is observed if present and labeled unselected/
unverified; the measured route remains the private publisher bundle. An existing
observation failure is preserved and stops, without hidden retry/fallback.

The script exclusively creates .build/swi-runtime-private-v1 and downloads only
the two exact publisher archives. It checks sizes/hashes before attach and safely
extracts files/relative internal links; duplicate/escaping/unselected special or
hardlink entries diagnose. It mounts only the verified DMG read-only at its fresh
owned directory and validates the actual plist mount point. Exactly one app and
internal-only links are required. Immutable original fat app is retained; a
separate copy receives a saved lipo ARM64 slice of its unique actual Mach-O swipl.
Native libraries may remain publisher fat binaries, while the scored launcher
must be exactly ARM64 and the running VM must report ARM64. No filename-only
architecture assumption or broad process/disk cleanup is permitted.

Contents/swipl is a checked candidate home layout, not an observed fact; the
relocated running runtime must agree. Different layout stops and retains bytes.
The official [tagged shlib.pl](https://raw.githubusercontent.com/SWI-Prolog/swipl/V10.0.2/library/shlib.pl)
describes bundle foreign search under PlugIns and current_foreign_library
registration. The probe resolves actual File specs through the public executable
absolute_file_name search used by that loader; unresolved/resource/external
paths diagnose rather than infer package identity.

The unscored probe records actual version/home/arch/default GC/stack/bounded
flags, generic220-bit positive division and negative floor/modulo, sqrt, real
standard table growth/update, and PCRE2 fold/replacement/version. These are
runtime/API checks, not workload witnesses. Explicit jit(false) remains a source
semantic-review gate because unsupported options may be silently ignored. The
controller records exact runtime/foreign images, recursively resolves non-OS
Mach dependencies within the private app, saves owned lipo/otool receipts and
pins complete fat/native/source corpora and actual loaded library/boot files.

finally detaches only the exact freshly owned directory if it remains mounted.
Failed detach stays fatal and requires root review. Interrupt handlers make
ordinary termination enter cleanup. A killed worker still has durable wrapper/
attach evidence for exact cleanup review. Terminal closure continues collecting
all readable files after an artifact error, retains each per-path error and the
original cause, and exits nonzero. No mount takeover or inferred device signal.

Success intentionally emits runtime-discovered-awaiting-root-source-library-review
with configuration_admitted=false. It cannot satisfy --prepare's required
qualified-runtime-discovery/default_gc_review passed. Root must inspect actual
boot/default-GC/numeric/hash/PCRE/version/ARM/source/image correspondence, then
produce an additive metadata-only qualified configuration bound to that exact
report/terminal digest and actual review bytes. No native/version rerun is
needed for metadata admission. Its later reviewed SHA goes to bounded prepare.

Reviewer focus: exact owned mount lifecycle, actual Mach-O path, ARM slice
source/output receipt identity, GC/unbounded flags, QLF versus native application
backend, complete non-OS/foreign source-image closure, original archive/fat app
preservation, late terminal errors and zero full runs.
