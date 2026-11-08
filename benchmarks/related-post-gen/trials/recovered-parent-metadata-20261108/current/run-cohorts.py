from pathlib import Path
import hashlib,importlib.util,json,os,re,subprocess,time
P=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results'
S=Path('/private/tmp/mech-related-recovered-parent-metadata-compiler-20261108')
O=Path('/private/tmp/mech-related-recovered-parent-metadata-opt-20261108-v1')
T=Path('/private/tmp/mech-related-recovered-parent-metadata-tests-20261108-v1')
N=Path('/private/tmp/mech-related-recovered-parent-metadata-runners-20261108-v1')
def write(p,d):p.write_text(json.dumps(d,indent=2)+'\n')
def quiet(name):
 snap=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True);busy=[]
 for line in snap.splitlines()[1:]:
  a=line.split(None,4)
  if len(a)!=5 or int(a[0])in[os.getpid(),os.getppid()]:continue
  if re.search(r'(?:^|/)(?:rustc|cargo|clang|clang\+\+|llc|opt|sample|gzip|xz|zstd|gpu-tests|one-kib|scalar|runner-(?:aot|metrics|pc|trace|phases))(?:\s|$)',a[4]):busy.append(a[:2]+[a[4].split()[0]])
 assert not busy,busy
 write(P/(name+'-quiet.json'),dict(quiet=True,process_snapshot_sha256=hashlib.sha256(snap.encode()).hexdigest(),no_other_own_benchmark_build_test_profile_archive=True))
def run(cmd,name):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True)
 write(P/(name+'.json'),dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr));print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-4000:]+r.stderr[-4000:]
spec=importlib.util.spec_from_file_location('h',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
assert h.source_inventory(S)==json.loads((O/'source-inventory.json').read_text())['files']
assert not subprocess.check_output(['git','-c','gc.auto=0','status','--porcelain'],cwd=S,text=True).strip()
assert json.loads((T/'summary.json').read_text())['actual_CI_passed']==418
assert '4 passed' in json.loads((T/'optional-llvm-regressions.json').read_text())['stdout']
for kind in ['scalar','one-kib']:
 q=json.loads(Path('/private/tmp/mech-related-recovered-parent-metadata-regressions-20261108-v1',kind+'-comparison.json').read_text());assert q['all_semantic_nontiming_fields_equal_IR240'];assert q['allocation_requests_never_regress'] and q['requested_bytes_never_regress']
q=json.loads((N/'plain/summary.json').read_text());assert q['exact_outputs']==32 and q['linked_audit_passed'] and q['native_functions']==51
assert json.loads((P/'extra-qualification.json').read_text())['extra_edge_outputs_exact']==3
for f,n in [('latency-config.json',30),('resources-config.json',31)]:assert len(json.loads((P/f).read_text())['implementations'])==n
for f in ['comparison-native-recovered-parent-metadata-20261108-5000-m3.json','comparison-native-recovered-parent-metadata-resources-20261108-5000-m3.json']:assert not (D/f).exists(),'Existing cohort must not be overwritten'
write(P/'source-freeze.json',dict(source=str(S),build=str(O),source_inventory_exact=True,commit=subprocess.check_output(['git','-c','gc.auto=0','rev-parse','HEAD'],cwd=S,text=True).strip(),predeclared_rule_sha256=hashlib.sha256((P/'prospective-rule.json').read_bytes()).hexdigest(),no_prior_cohort=True))
quiet('latency')
run(['python3',R/'compare-implementations.py','--config',P/'latency-config.json','--input','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','--samples','31','--order','randomized','--seed','990296','--deduplicate-outputs','--work',P/'latency','--output',D/'comparison-native-recovered-parent-metadata-20261108-5000-m3.json'],'latency-run')
lat=json.loads((D/'comparison-native-recovered-parent-metadata-20261108-5000-m3.json').read_text());res=json.loads((P/'resources-config.json').read_text())
instrumented=lambda n:'rust-standard-instrumented'if n=='rust'else n+'-instrumented'
res['resource_reference']=instrumented(lat['matched_native_reference']['variant']);res['whole_process_resource_reference']=instrumented(lat['whole_process_reference']['variant']);assert {res['resource_reference'],res['whole_process_resource_reference']}<={v['name']for v in res['implementations']};write(P/'resources-config.json',res)
quiet('resources')
run(['python3',R/'compare-implementations.py','--config',P/'resources-config.json','--input','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','--samples','7','--order','randomized','--seed','9902967','--deduplicate-outputs','--resources','--work',P/'resources','--output',D/'comparison-native-recovered-parent-metadata-resources-20261108-5000-m3.json'],'resources-run')
assert h.source_inventory(S)==json.loads((O/'source-inventory.json').read_text())['files']
print('both cohorts complete',flush=True)
