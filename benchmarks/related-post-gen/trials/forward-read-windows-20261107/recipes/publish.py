from pathlib import Path
import gzip,hashlib,importlib.util,json,subprocess
S=Path('/private/tmp/mech-related-forward-read-windows-compiler-20261107');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results'
relative='benchmarks/related-post-gen/trials/forward-read-windows-20261107';out=S/relative;compiler='f018ed15430d173d3f931f1f59ac0f6ed83eca11'
git=lambda *args:subprocess.check_output(['git','-c','gc.auto=0',*args],cwd=S,text=True).strip()
assert git('rev-parse','HEAD')==compiler and not git('status','--porcelain')
spec=importlib.util.spec_from_file_location('inventory',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
O=Path('/private/tmp/mech-related-forward-read-windows-opt-20261107-v8');inventory=json.loads((O/'source-inventory.json').read_text())['files'];assert h.source_inventory(S)==inventory
files={'trial.json':D/'compiler-native-forward-read-windows-20261107.json','latency.json.gz':D/'comparison-native-forward-read-windows-randomized-20261107-5000-m3.json','resources.json.gz':D/'comparison-native-forward-read-windows-resources-20261107-5000-m3.json','sizes.json.gz':D/'size-native-forward-read-windows-20261107.json','24-hour-progress.json':D/'compiler-native-24-hour-progress-20261107T114151Z.json'}
for version in range(1,9):
 for kind in ['opt','tests','regressions']:
  folder=Path(f'/private/tmp/mech-related-forward-read-windows-{kind}-20261107-v{version}')
  if not folder.exists():continue
  for p in folder.iterdir():
   if p.is_file() and (p.suffix in ['.json','.py','.rs']or p.name.endswith('.log')):files[f'v{version}/{kind}/{p.name}.gz']=p
 U=Path(f'/private/tmp/mech-related-forward-read-windows-runners-20261107-v{version}')
 if not U.exists():continue
 for variant in ['after-4','reuse-4']:
  for name in ['runner.rs','native-link-audit.json','runner-aot.build.json','edge-run.json','edges/edge-fixtures.json','build/metadata.json','build/source.mec']:
   p=U/variant/name
   if p.is_file():files[f'v{version}/runners/{variant}/{name}.gz']=p
 for name in ['build-initial-runners.py','native-code-comparison.json','native-text-equality.json']:
  p=U/name
  if p.is_file():files[f'v{version}/runners/{name}.gz']=p
for version in [4,5,6]:
 folder=Path(f'/private/tmp/mech-related-forward-read-windows-debug-20261107-v{version}')
 for p in folder.iterdir():
  if p.is_file()and (p.suffix in ['.json','.py','.patch']or p.name.endswith('.log')):files[f'diagnostics/D{version}/{p.name}.gz']=p
M=Path('/private/tmp/mech-related-forward-read-windows-metrics-20261107-v1')
for name in ['metrics-config.json','source-pins.json']:files['metrics/'+name+'.gz']=M/name
for p in (M/'mech-ir264-reuse-4').rglob('*'):
 if p.is_file()and not p.is_symlink()and'build'not in p.relative_to(M).parts and p.suffix in ['.json','.rs','.patch']:files['metrics/'+str(p.relative_to(M))+'.gz']=p
for name in ['report','publish']:files['recipes/'+name+'.py']=Path(f'/private/tmp/mech-related-forward-read-windows-{name}-20261107.py')
records={}
def put(name,raw,source=None,original=None):
 b=gzip.compress(raw,mtime=0)if name.endswith('.gz')else raw;p=out/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
 v=dict(sha256=hashlib.sha256(b).hexdigest(),bytes=len(b),decoded_sha256=hashlib.sha256(raw).hexdigest(),decoded_bytes=len(raw))
 if source:v['retained_source']=str(source)
 if original:v['unredacted_source_sha256']=hashlib.sha256(original).hexdigest()
 records[name]=v
assert not out.exists();assert all(p.is_file()for p in files.values())
for name,p in files.items():put(name,p.read_bytes(),p)
P=Path('/private/tmp/mech-related-forward-read-windows-cohort-20261107-v1')
for p in P.iterdir():
 if p.is_file()and p.suffix in ['.json','.py']:
  raw=p.read_bytes()
  if p.name.endswith('-start-gate.json'):
   d=json.loads(raw);snapshot=d.pop('process_snapshot');d.update(process_snapshot_sha256=hashlib.sha256(snapshot.encode()).hexdigest(),process_snapshot_retained_locally=str(p),process_snapshot_omission='Unrelated workstation arguments remain local; all original gate predicates are unchanged.');put('cohort/'+p.name,(json.dumps(d,indent=2)+'\n').encode(),p,raw)
  else:put('cohort/'+p.name,raw,p)
readme='''# IR264 current forward-read windows (unselected)

Compiler f018ed15430d173d3f931f1f59ac0f6ed83eca11, IR264/work115/ABI41/image9, integrates PR973 through b6a42fde80dbc011dfefa9de5d9659be334411b0. The source branch is codex/related-forward-read-windows-20261107. Selected IR242 and its original promotion record are unchanged. Newer upstream feb2149 remains pending after this frozen trial.

Current native child-window discovery covers explicit and implicit mathematical one-based reads, including zero-offset forward intervals. Fresh gates retain source-bound selection on each visit. Counts copied through carriers require exact Initial identity at every ordinary reachable post-copy latch, independently of proposed windows. Changed bounds and terminal payload erasure decline. Unread owners keep existing structural/version checks and zero content permissions; a Length query alone cannot authorize the new flat forwarding route. Forward duplication requires a newly proved read or source successor. The paired-view floor, source errors, finite debits, ownership, aliases, atomic publication and retry are retained. No benchmark source or runtime algorithm changed.

Both hot query children now have real current gates, proving posting reads1358 and1569 and17 source successors. Query code grows27552 to31292B, stack stays544B. Arbitrary loaded mask indices remain checked. Original/mask code totals are917440/886344B. Native links are audited.

All248 latency outputs (31 randomized rounds, seed990264) and63 separate resource outputs (seven rounds, seed9902647) match Rust. No compiler/test/profile/compression work overlaps measurement. All first/adverse observations remain. Processing is continuous post-JSON through preparation/admission, worker creation/run/join and materialization; complete process includes restore/JSON/teardown. Resource/profile timings are excluded.

Fastest eligible Rust measures1.597166ms processing and22.859666ms complete. Selected IR242 measures2.844667ms/1.781072x and34.530292ms/1.510534x. IR257 measures2.847334ms/1.782741x and32.818083ms/1.435633x, winning13/26 paired rounds versus selected. IR264 measures2.817458ms/1.764036x and35.711042ms/1.562186x, winning21/6. Both remain unselected under the unchanged21/31-both-scopes requirement. No repeated favorable cohort or full adoption qualification is claimed.

IR264 processing uses3758 requests/8012356B and54727858 instructions versus selected54856449. Restore remains26837 requests/12830999B and125854487 instructions. Native bounds work yields a small processing gain; it does not establish whole-process improvement or Rust parity.

Qualification:894 GPU passes plus exactly the same two inherited failures;399 CI-selected names actually listed and executed;62 Rust edge outputs; all133 scalar/DEFLATE rows exact in every non-timing field. Ten focused groups pass. Eight guarded/direct/invariant-carried/terminal-discarding source variants cover original/decoded/restored execution, zero-trip/wrapped coordinates, source errors, aliases, publication, recovery and every finite boundary.

Every development attempt is retained. IR261 passed positive one/two-view coverage but failed the opaque negative because Length counted as content. IR262 tightens that condition to Read. IR263 first test build failed a metadata temporary borrow; the test-only correction is retained. D5 locates missing implicit-read discovery; D6 records20 actual query analyses whose counts stay Initial but were rejected by a syntactic write filter. Its first Debug-format build failure is retained. The first IR264 positive fixture erased its terminal bound; that original case remains a required conservative refusal. A separate positive retains the bound on all backedges. No benchmark source changes or relaxed errors replace compiler fixes.

verify.py checks stored/decoded hashes,248/63 exact outputs and every raw paired gate. Evidence adds no Rust source/build manifest and preserves the frozen compiler inventory. Executables/objects and full workstation snapshots stay local; their receipts, audit hashes and unchanged quiet predicates are included. The24-hour report compares selected IR208 and selected IR242 in separate historical/current cohorts; it is a checkpoint trend, not a paired causal estimate.
'''
put('README.md',readme.encode())
template=Path('/private/tmp/mech-related-selected-range-integration-compiler-20261107/benchmarks/related-post-gen/trials/selected-ranges-and-json-20261107/verify.py').read_text();put('verify.py',template.replace('434','248').replace('105','63').encode())
(out/'manifest.json').write_text(json.dumps(dict(schema='mech.related-post-gen.trial-evidence-manifest.v2',compiler_commit=compiler,selected=False,files=records),indent=2)+'\n')
subprocess.run(['python3',str(out/'verify.py')],check=True);assert h.source_inventory(S)==inventory
git('add','--',relative);git('diff','--cached','--check');assert all(n.startswith(relative+'/')for n in git('diff','--cached','--name-only').splitlines());git('commit','-m','Record qualified forward-read-window trial and adverse whole-process result')
receipt=dict(compiler_commit=compiler,evidence_commit=git('rev-parse','HEAD'),files=len(records),packet=relative,source_inventory_exact=True,hashes_verified=True,paired_gates_verified=True,pushed=False,selected_changed=False)
Path('/private/tmp/mech-related-forward-read-windows-publication-20261107.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))
