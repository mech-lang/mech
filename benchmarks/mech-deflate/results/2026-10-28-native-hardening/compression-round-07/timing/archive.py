from pathlib import Path
import gzip,hashlib,json,shutil,subprocess
root=Path(__file__).resolve().parent
w=Path('/private/tmp/mech-deflate-linear-stack')
dest=w/'benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-07'
assert not dest.exists()
(dest/'timing').mkdir(parents=True);(dest/'qualification').mkdir()
sc=json.loads((root/'scalar-results.json').read_text())
shutil.copy2('/private/tmp/mech-live-control-target/release/native_frame_transport',root/'frame-after')
assert hashlib.sha256((root/'frame-after').read_bytes()).hexdigest()==sc['after_sha256']
(root/'frame-after.json').write_text(json.dumps({'sha256':sc['after_sha256'],'compiler':'51eeb3f5fc5390b7b1b5213870b60ed3c577d182'},indent=2)+'\n')
for p in root.iterdir():
 if not p.is_file():continue
 if p.suffix in ['.json','.py']:
  shutil.copy2(p,dest/'timing'/p.name)
 elif p.suffix in ['.stdout','.stderr','.log','.rs'] or p.name.endswith('.sample.txt'):
  (dest/'timing'/(p.name+'.gz')).write_bytes(gzip.compress(p.read_bytes(),mtime=0))
v=Path('/private/tmp/mech-linear-restack-20261026/validation')
for p in sorted(v.glob('round07-*.json')):
 shutil.copy2(p,dest/'qualification'/p.name)
 log=p.with_suffix('.log')
 if log.exists():(dest/'qualification'/(log.name+'.gz')).write_bytes(gzip.compress(log.read_bytes(),mtime=0))
report='''# Round 07: successful native metadata precharge

Starting published head: `14c64b97c56793fce379ccf6fd3b96ebcf4b927a`.
Protocol and red regression: `0a575edfce86deeedcd11e9b5ee5e2ec5b93c1a8`.
Separate feature guard repair: `03a6f6518d15e480e30270c1acbe9fa5cffb9e89`.
Runtime implementation and final test head: `51eeb3f5fc5390b7b1b5213870b60ed3c577d182`.
Stack parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The existing immutable checked schema summary now prepays a successful native
structural projection scan once, against its actual current scope and policy.
Native meters previously required an active resident turn-plan cache for preview,
so they missed this successful accounting path. Preview spends nothing and
checks the original owning ledger, finite call limits and memory/node policy.
Shared worker loans retain the incremental path. Tight budgets, overflow and
foreign/detached scopes keep the original ordered contributions, first denial
and spent prefix. The existing borrowed-metadata prepayment now benefits from
the same preview; independent incremental-oracle tests establish its behavior.
No representation, scope authority, admission or publication boundary changes.

## Small whole-compression timing

Apple M1, unchanged eight-byte `abcdefgh`, level 12, **Roc = 1x**:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned reference, 20000 observations | 0.000284 | 1x |
| Before batching | 0.162907833 | 573.62x |
| After batching | 0.140640417 | 495.21x |

This is **1.1583x faster** than the paired Mech baseline, or **13.67% less
elapsed time**. The preceding 574.09x uses a different reference cohort. The
retained before executable is rerun here against the same fresh reference.
Three primary observations per subject, warmup/extra calls excluded, all raw
rows retained. A separate two-second sample and its 29 repetitions are excluded.
No compilation or other qualification overlaps primary or protected scalar
measurements. No larger complete input was run.

Before executable SHA-256:
`8b2a5b0c09ee3f2af07232c930b61d741b47db03eafed1f3597629298adebf33`.
After executable SHA-256:
`97aa2b05a99ddc3efecc0fe04cda64a817bb8ef21bee0f8d61375e715b00d3d8`.
Source SHA-256 is `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
artifact SHA-256 is `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
The selected catalog, pinned mimalloc 2.1.7 and evaluate_into-only harness are
unchanged. Every call passes exact raw bytes, independent inflation, aliases,
capacity 13 and terminal zero managed bytes/nodes. Complete logical work stays
**888390537256**, with zero delta. All 25 source components, algorithm, tables,
search/EOF limits, real buffers, allocation sites, six unused initializers and
timing boundaries are unchanged. No qualification allowance is increased.

## Validation and dispositions

**104 scoped Rust tests pass:** 20 native-budget/metadata/work tests, 71 existing
component tests, eight native scalar tests, three bit tests and two actual
collector tests. The new checks compare result, error, spent work and retained
state with the original incremental oracle at every positive schema contribution
boundary, prior spent work, node/call/account denials, overflow and worker-loan
fallback. Foreign/escaped meters do not preview successfully. Repeated/nested
scopes, unwind cleanup and retry retain their existing tests. Both original and
decoded component forms remain covered. All expected values and policies stay.

The actual collector completes six positions on 320 bytes with the full 262144
U16 tree and 1500513 U32 cache, depth 300 and nice 258. Search and skip remain
separate. Golden full state, aliases, failure cleanup and recovery pass. Original
instrumented traffic remains 8362 requests / 35045914 bytes; decoded remains
8361 / 35045850. The separate 24-observation store diagnostic also keeps exactly
the same allocations, bytes and work. This patch reduces accounting execution;
it does not remove additional payload copies.

Protected original/decoded Bool/U64/I64/F64 controls preserve exact allocation
counts and bytes. Median ratios 1.0029–1.0149 pass the declared 1.05 gate.
Zero/64-visit scalar-loop allocation slopes stay zero. Locked offline release
build passes in 447.89 seconds, reusing all machine libraries. The alternate
full_compiler,async profile without native-artifact checks successfully in
15.65 seconds. All twelve routing/catalog/layout/architecture/inventory/warning/
unsafe/format commands pass. Exact commands, feature inventories and hashes are
archived. The default configuration still treats warnings as errors.

The initial preview command selected **zero tests** because native-artifact was
missing; it is not passing qualification. Adding that feature exposed an unused
mapper import introduced by round 06 in the sequence_buffers-disabled profile.
The separate guard commit repairs that introduced failure. A corrected native
run then executes four passing tests and the expected new preview failure on
03a6f651. One attempted combined test command has invalid CLI filter placement;
it is retained as setup failure. The corrected final command executes all 20
tests on clean 51eeb3f5. No executed production failure remains. The intentional
unwind panic is a passing cleanup test, not an unhandled test failure.

Fresh source-to-artifact regeneration was last checked in round 05; source and
artifact hashes are verified again here and only native/runtime paths change.
The all-primitives test requires not/xor absent from this selected catalog and
is explicitly skipped. Broader linked/generated applications and the host route
unit suite are not fresh executions. Historical block/window/full-corpus limits
remain unchanged; ignored/cancelled campaigns supply no new qualification. CI is
cancelled by request; #973 stays draft and unmerged.

The new profile has 1364 samples and 37 inclusive ordered-schema-precharge
samples, versus 212 of 1356 previously. TLS (154), budget evaluation (79), visits
(70), memmove (55) and helper admission (55) remain prominent exclusive symbols.
These are diagnostic sample counts, not exact time shares. Raw unresolved/JIT
symbols are preserved without invented source attribution. The remaining 495x
gap calls for eliminating repeated canonical/runtime transactions in real numeric
loops, rather than another series of small metadata changes. Parity is unmet.
'''
(dest/'REPORT.md').write_text(report)
rows=[{'path':str(p.relative_to(dest)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(dest.rglob('*')) if p.is_file()]
(dest/'manifest.json').write_text(json.dumps({'files':rows},indent=2)+'\n')
print(json.dumps({'packet':str(dest),'files':len(rows),'bytes':sum(r['bytes'] for r in rows)}))
