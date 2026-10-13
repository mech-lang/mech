# Private Rebol runtime intake candidate

Source only. Nothing in this packet has been imported, parsed, hashed, tested,
downloaded, generated, compiled or executed by its author. Root owns freeze,
actual receipt/digest review and one durable launch per phase. No global
install, Homebrew upgrade, signing/quarantine or permission change. Red is
excluded. No benchmarks, algorithms, full samples, canonical counts or dataset
changes are authorized by this runtime entry.

Review six files: bootstrap_runtime.py, contracts.py, fixture_runner.py,
test_runtime.py, runtime_probe.reb and this review. Root research/capability
notes are static prerequisites. Existing ten source ports remain a separate
source-owner packet and are not frozen or executed by this intake.

## Concrete phases and proposed commands

From rebol-comparison/20261013, first metadata-only seal:

```sh
python3 -B revisions/runtime-v1/bootstrap_runtime.py --seal
```

It acquires the named common mutex, immediately checks held/named device+inode,
keeps an inherited FD>=64, pins candidate inputs, validates retained actual
C3 private-v1 proposal43438603...050 and exact eleven passed ownership fixtures
before fresh service imports. It invokes no native command or download. It
freshly copies/pins Python+unittest .py sources and exact test/contracts copies,
records loader errors/discovered IDs before assertions, runs exactly six
methods with complete started IDs and zero skip/error/failure/expected outcomes,
retains raw/discovery/receipt bytes and seals proposals/runtime-v1.json. Early
hash/setup/import/fixture failures save partial terminal evidence under the lock.

Only after root reviews the emitted actual proposal SHA:

```sh
python3 -B revisions/runtime-v1/bootstrap_runtime.py --run --proposal-sha256 ACTUAL_REVIEWED_SHA
```

This is a separate exclusive runtime-qualification/runtime-v1 output plus
.build/runtime-v1. It verifies the proposal, all immutable inputs and exact six
actual fixture receipt before any download or owned child. It does not rerun
the fixtures. Before creating the private build root, it checks the prospective
resolved destination and each existing ancestor for a symlink or cohort escape;
the created directory is independently reconciled afterward. The existing
startup fixture includes an outside-target `.build` symlink regression.
No mode authorizes compiler/header/extension work or benchmarks.

## Publisher/runtime and source identities

The publisher [3.22.1 asset](https://github.com/Oldes/Rebol3/releases/expanded_assets/3.22.1)
rebol3-bulk-macos-arm64.gz has observed publisher SHA256
6f078cfba45dcfc43502fee6795361180ce0f1ed6964521278a488613408311a.
The [release commit link](https://github.com/Oldes/Rebol3/commit/7fe158a9a9b0be8826c35a27596cb3c207b5ed6c)
identifies the full tag commit7fe158a9a9b0be8826c35a27596cb3c207b5ed6c.
The runtime download is bounded and exclusively retained; publisher hash must
match before bounded gzip extraction. Original decompressed binary is retained,
then copied byte-equal to a separate private executable. Actual owned lipo,
otool header and linkage must identify exactly ARM64. No x64/fat/emulated fallback.

Frozen C3 owned_command's lifecycle is reused unchanged with its passed11
ownership receipts, timeout/new-PGID/known leader cleanup and bounded reap/drain.
Its native_images runs only on the entry executable. Every install name then
must be a normalized absolute OS-library path; escaped /usr/lib paths reject.
Unknown private/@rpath dependencies stop for separate closure review rather
than reuse the frozen service's library @executable_path assumptions. Original
raw command receipts/image metadata are preserved. No C3 binaries are run.

The source tar URL names that full commit. No source tar publisher SHA was
observed in official metadata, and none is invented. Actual downloaded tar SHA
is newly captured; safe extraction requires exactly the full-commit archive
root, canonical member paths, no links/devices/duplicates, finite file/total
limits and exact .version3.22.1. All readable original source bytes and explicit
header-generator/startup/GC/IO/ABI sources are pinned. The actual tar digest and
source correspondence require root review before any vendor-source execution;
the report explicitly leaves independently verified source-tree commit false.

The exact tagged generation route is preserved: make/pre-make.r3 invokes its
six scripts, with make/tools/make-reb-lib.reb scanning complete a-lib.c then
f-extension.c in order, and make-boot.reb/types-ext.reb emitting extension type
tables. Full source intake preserves dependencies rather than a guessed partial
RL_LIB table. No generator runs in this packet. A later separately reviewed
header-generation service must consume the actual source tar/tree pins and
execute copies, preserving these immutable originals and all generated outputs.

## Native feature and startup boundaries

The child environment explicitly supplies ordinary OS PATH, C locale and a
private REBOL_HOME. It changes no global HOME or permission setting. Runtime
adjacent rebol.reb, private user.reb and any private module/data file must be
absent before and after queries. The later exact environment/startup checks
must also guard every original scored process and count each authored helper.
The child -s option permits the private script's file operation without a
prompt and is recorded as child-local policy, not a global setting.

Five actual preparation-only child commands are planned: three image queries,
one exact version query and one tiny generic runtime-feature script. It observes
selected version/macOS, a user function value distinct from native primitives,
dynamic rebinding, independent fresh block identities, actual automatic GC
counter advance during at most65536 discarded1KiB generic buffers, and exact
all256 regular-file bytes. There is no manual collector or policy-changing
call. Normal automatic collection must be observed; a stalled counter fails.
All stdout/receipts/counters/native image/actual probe source remain pinned.

The exact automatic-GC observation API is source-bound: tagged
[natives.reb:914](https://github.com/Oldes/Rebol3/blob/3.22.1/src/boot/natives.reb#L914)
declares /profile as the second stats refinement;
[n-system.c:146](https://github.com/Oldes/Rebol3/blob/3.22.1/src/core/n-system.c#L146)
handles D_REF(2), returns SYS_STANDARD/STD_STATS as an object and fills its
Recycle_Counter field at176. The tagged
[standard object:590](https://github.com/Oldes/Rebol3/blob/3.22.1/src/boot/sysobj.reb#L590)
names that field recycles. The probe first evaluates stats/profile, then reads
the returned object's recycles field and saves the integer before requerying;
it does not assume a combined stats/profile/recycles call path.

The controller has no Python3.14 restriction. Root may use its existing bundled
Python3.12 with -E -S -B -u. Actual executable/version and fresh unittest source
snapshot identities are captured in the fixture receipt; no Python version
subprocess is run by the seal.

The runtime's interpreted function value plus official source semantics is a
review input, not publication metadata proving original scored program work.
Regular-file binary IO cannot prove pipe stdin/stdout. Their observed field
remains false; source-reviewed counted generic chunk-read/write-all facade,
Rebol chunk aggregation and
actual byte/short-read/EINTR/ownership probes are later requirements. GMP/PCRE2
extension ABI, generated headers, all twenty profiles, source-work and complete
full samples likewise remain false/unexecuted. No helper/library implementation
is guessed or installed here.

## Exact fixture and closure review

Six RuntimeContracts methods, listed literally in contracts.IDS, cover:

1. Metadata seal cannot authorize downloads/native/header/full work; partial,
   skipped or numeric-boolean fixture/proposal inventories cannot run.
2. Exact publisher ARM64 gzip/full-commit source identity; x64/master/abbreviated
   commit/invented prior source digest reject.
3. Actual archive member path rules reject traversal, links/devices, wrong root,
   aliases and duplicate entries.
4. OS-image exemption rejects escaped absolute installs, private/@rpath/unknown
   libraries and noncanonical separators.
5. Real temporary startup directories reject adjacent/user/imported module text.
6. Wrong interpreter/version/GC types/counts/full byte output fail; successful
   regular IO explicitly leaves pipe/extension/header/full flags false.

Fixtures are purely synthetic and never runtime qualification. Existing actual
eleven ownership tests are reused with complete byte/receipt pins; no mirror
cleanup test catalogue or duplicate native command is added. Terminal capture
continues after individual hash/read failures, preserves all available raw and
private-root bytes, marks failed-terminal-integrity with partial pins and exits
nonzero. No retry/overwrite is permitted.

Root/peer should inspect first-seal dispatch, proposal-before-native gate,
fresh source/import identities, exact fixture discovery+loader diagnostics,
common lock inode/lifetime, inherited FD, original fat/native separation,
bounded publisher/extraction paths, strict OS linkage, private startup inputs,
automatic GC inference limits, source SHA provenance and terminal failures.
Actual outputs must be reviewed before a later header/primitive/bounded suite.
