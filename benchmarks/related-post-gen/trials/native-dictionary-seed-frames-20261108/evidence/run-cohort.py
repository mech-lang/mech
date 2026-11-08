from pathlib import Path
import datetime,hashlib,importlib.util,json,os,re,shutil,subprocess,time,sys
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results';S=Path('/private/tmp/mech-related-native-dictionary-seed-frames-compiler-20261108')
confirm=len(sys.argv)>1 and sys.argv[1]=='confirmation'; label='confirmation'if confirm else'first';Q=P/label;Q.mkdir(exist_ok=True)
def read(p):return json.loads(Path(p).read_text())
def write(p,d):Path(p).write_text(json.dumps(d,indent=2)+'\n')
rule=read(P/'prospective-rule.json');spec=importlib.util.spec_from_file_location('h',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);files=h.source_inventory(S)
assert files==read(P/'qualified-source-inventory.json')['files']==read('/private/tmp/mech-related-native-dictionary-seed-frames-opt-20261108-v4/source-inventory.json')['files']==read('/private/tmp/mech-related-native-dictionary-seed-frames-runners-20261108-v1/runtime/source-inventory.json')['files']
assert read(P/'GPU-qualification-summary.json')['actual_CI_passed']==466
assert read(P/'Engine-component-qualification.json')['all138_native_collection_contracts_passed']
assert read(P/'control-parent-parity.json')['all_nontiming_fields_exact']
assert read('/private/tmp/mech-related-native-dictionary-seed-frames-runners-20261108-v1/histogram-reuse/summary.json')['exact_outputs']==32
assert read('/private/tmp/mech-related-native-dictionary-seed-frames-emission-control-20261108/summary.json')['exact_outputs']==32
assert not subprocess.check_output(['git','-C',str(S),'status','--porcelain'],text=True).strip(),'source must be committed before authoritative timing'
commit=subprocess.check_output(['git','-C',str(S),'rev-parse','HEAD'],text=True).strip()
for n in ['latency-config','resources-config']:
 c=read(P/(n+'.json'));write(Q/(n+'.json'),c)
 for v in c['implementations']:assert Path(v['executable']).is_file()and os.access(v['executable'],os.X_OK)
suffix='-confirmation'if confirm else''
latname='comparison-native-dictionary-seed-frames'+suffix+'-20261108-5000-m3.json';resname='comparison-native-dictionary-seed-frames-resources'+suffix+'-20261108-5000-m3.json'
assert not(D/latname).exists()and not(D/resname).exists(),'existing cohort cannot be overwritten'
write(Q/'source-freeze.json',dict(compiler_commit=commit,files_sha256=hashlib.sha256(json.dumps(files,sort_keys=True).encode()).hexdigest(),rule_sha256=hashlib.sha256((P/'prospective-rule.json').read_bytes()).hexdigest(),source_algorithm_unchanged=True,qualification_utc=datetime.datetime.now(datetime.timezone.utc).isoformat()))
def quiet(n):
 assert shutil.disk_usage(P).free>=2*1024**3,'Need2GiBfree for deduplicated data and external storage pressure'
 snap=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True);busy=[]
 for l in snap.splitlines()[1:]:
  a=l.split(None,4)
  if len(a)!=5 or int(a[0])in[os.getpid(),os.getppid()]:continue
  cmd=Path(a[4].split()[0]).name
  if re.fullmatch(r'rustc|cargo|clang|clang\+\+|llc|opt|sample|gzip|xz|zstd|(?:gpu-native|gpu|engine|mech_core|mech_engine)-tests|one-kib|scalar|runner-(?:aot|metrics|profile|pc|trace|phases)',cmd):busy.append(a[:2]+[a[4].split()[0]])
 assert not busy,busy;write(Q/(n+'-quiet.json'),dict(quiet=True,process_snapshot_sha256=hashlib.sha256(snap.encode()).hexdigest(),no_build_test_profile_archive_overlapped=True))
def run(c,n):
 c=list(map(str,c));t=time.monotonic();x=subprocess.run(c,capture_output=True,text=True);write(Q/(n+'.json'),dict(command=c,exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr));print(label,n,x.returncode,flush=True);assert x.returncode==0,x.stdout[-3500:]+x.stderr[-3500:]
quiet('latency');run(['python3',R/'compare-implementations.py','--config',Q/'latency-config.json','--input','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','--samples','31','--order','randomized','--seed',str(rule['confirmation_seed']if confirm else rule['seed']),'--deduplicate-outputs','--work',Q/'latency','--output',D/latname],'latency-run')
lat=read(D/latname);res=read(Q/'resources-config.json');inst=lambda n:'rust-standard-instrumented'if n=='rust'else n+'-instrumented';res['resource_reference']=inst(lat['matched_native_reference']['variant']);res['whole_process_resource_reference']=inst(lat['whole_process_reference']['variant']);write(Q/'resources-config.json',res)
quiet('resources');run(['python3',R/'compare-implementations.py','--config',Q/'resources-config.json','--input','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','--samples','7','--order','randomized','--seed',str(rule['confirmation_resource_seed']if confirm else rule['resource_seed']),'--deduplicate-outputs','--resources','--work',Q/'resources','--output',D/resname],'resources-run')
assert files==h.source_inventory(S);write(Q/'cohort-receipt.json',dict(compiler_commit=commit,source_unchanged=True,latency=str(D/latname),resources=str(D/resname),goal_complete=False));print(label,'cohort complete; no promotion decision yet',flush=True)
