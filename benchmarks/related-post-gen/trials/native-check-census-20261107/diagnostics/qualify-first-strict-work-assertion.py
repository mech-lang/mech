from pathlib import Path
import collections,hashlib,importlib.util,json,os,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');B=Path('/private/tmp/mech-related-llvm-native-image-opt-20261107-v4');O=Path('/private/tmp/mech-related-native-check-census-opt-20261107-v1');C=Path('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4')
def write(p,j):p.write_text(json.dumps(j,indent=2)+'\n')
def run(cmd,name,env=None):
 cmd=list(map(str,cmd));start=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})});write(P/(name+'.json'),dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr,diagnostic_timings_excluded=True));print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-4000:]+r.stderr[-6000:];return r
expected=json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
run([P/'runner-jit',P/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',P/'output-repeat.json','1'],'repeat5000-run',{'MECH_CHECK_CENSUS_OUTPUT':str(P/'census-repeat.json')})
assert json.loads((P/'output-repeat.json').read_text())==expected
first=json.loads((P/'census.json').read_text());second=json.loads((P/'census-repeat.json').read_text());assert first==second,'No assumption of deterministic diagnostics: preserve any differences'
source=(C/'runner.rs').read_text().replace('let catalog = linked_catalog().unwrap();','let catalog = mech_stdlib::source_catalog();');(P/'parent-runner.rs').write_text(source)
run(['python3',R/'build-host.py','--compiler-build',B/'build.json','--source',P/'parent-runner.rs','--out',P/'parent-runner-jit'],'parent-host-build')
parent=run([P/'parent-runner-jit',P/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',P/'parent-output.json','1'],'parent5000-run')
assert json.loads((P/'parent-output.json').read_text())==expected
before=json.loads(parent.stdout);after=json.loads(json.loads((P/'full5000-run.json').read_text())['stdout']);timing={k for k in before if 'seconds'in k};assert {k:v for k,v in before.items()if k not in timing}=={k:v for k,v in after.items()if k not in timing}
# JIT-only activation must leave actual Object emission completely unchanged.
(P/'compiler-facts.rs').write_text(Path('/private/tmp/mech-related-forward-read-windows-opt-20261107-v8/compiler-facts.rs').read_text())
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'compiler-facts.rs','--out',P/'compiler'],'Object-host-build')
run([P/'compiler',P/'build/source.mec',P/'uninstrumented-Object'],'Object-emission')
files={}
for f in sorted((C/'build').iterdir()):
 if f.is_file():
  actual=P/'uninstrumented-Object'/f.name;assert actual.exists()and actual.read_bytes()==f.read_bytes(),f.name
  files[f.name]=hashlib.sha256(actual.read_bytes()).hexdigest()
write(P/'Object-equivalence.json',dict(all_bundle_files_exact=True,files=files,Object_unchanged=True,LLVM_tools_used=False))
groups=collections.defaultdict(lambda:[0,0,0]);owners=collections.defaultdict(lambda:[0,0,0]);active=[]
for row in first['rows']:
 k=row['key'];v=[row['attempts'],row['successes'],row['failures']];groups[k['kind']]=[a+b for a,b in zip(groups[k['kind']],v)];key=(k['owner'],k['kind']);owners[key]=[a+b for a,b in zip(owners[key],v)]
 if row['attempts']:active.append(row)
query=[r for r in active if r['key']['owner']==1282]
windows=[r for r in query if r['key']['kind']=='funded-body'and r['key']['version']]
summary=dict(schema='mech.related-post-gen.executed-native-check-census.v1',diagnostic_only=True,timing_claim=False,scope=first['coverage'],all_two_5000_outputs_exact=True,independent_fresh_process_counters_exact=True,unmodified_parent_JIT_output_and_all_nontiming_report_fields_exact=True,uninstrumented_Object_bundle_files_exact=len(files),compiled_rows=len(first['rows']),active_rows=len(active),kind_totals={k:dict(zip(['attempts','successes','failures'],v))for k,v in groups.items()},owner_kind_totals=[dict(owner=k[0],kind=k[1],attempts=v[0],successes=v[1],failures=v[2])for k,v in owners.items()],main_query_rows=query,main_query_selected_leaf_funding_attempts=sum(r['attempts']for r in windows),
interpretation='Existing child-funding fast path executes zero times. Query1282 selected child-window bodies still execute109097 per-header funding gates. The selected windows prove immutable bound/index recurrences, but Funding::derive requires phase.checked_children; this numeric tree has no phase, so the funding proof is never constructed. Extend bounded activation funding to independently proved read-only numeric-tree leaves, retaining every actual debit, counter, source error and checked fallback. A gate-only count is not a dynamic instruction cost or measured speedup.',limitations=['Native atomic probes perturb instruction selection, register pressure and scheduling. No diagnostic timing is eligible for a benchmark gain.','Component funding checks and combined funded-body gates overlap; do not add them as distinct source traversals.','Coverage is limited to emitted numeric gates and current-window paths; it is not a complete interpreter/host/GPU/source-primitive census.','Successful bounds checks alone do not authorize their removal. Source/provenance/alias and current-state proofs are still required.'])
assert summary['main_query_selected_leaf_funding_attempts']==109097
write(P/'summary.json',summary);print('Census qualified: repeat counts, parent report and',len(files),'actual Object files exact',flush=True)
