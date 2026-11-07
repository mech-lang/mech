from pathlib import Path
import copy,hashlib,importlib.util,json,os,subprocess,time
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-native-check-census-compiler-20261107');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
B=Path('/private/tmp/mech-related-llvm-native-image-opt-20261107-v4');O=Path('/private/tmp/mech-related-native-check-census-opt-20261107-v1');O.mkdir(exist_ok=True)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def write(p,j):p.write_text(json.dumps(j,indent=2)+'\n')
spec=importlib.util.spec_from_file_location('build_helper',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
j=json.loads((B/'build.json').read_text());old_files=json.loads((B/'source-inventory.json').read_text())['files'];files=h.source_inventory(S)
changed=[n for n in files.keys()|old_files.keys()if files.get(n)!=old_files.get(n)];assert changed and all(n.startswith('hosts/gpu/')for n in changed),changed
source_hash=hashlib.sha256(json.dumps(files,sort_keys=True).encode()).hexdigest();write(O/'source-inventory.json',dict(root=str(S),sha256=source_hash,files=files))
for stage in j['stages']:assert stage['exit']==0 and sha(stage['command'][-1])==stage['library_sha256']
report=copy.deepcopy(j);report.update(schema='mech.native-check-census.diagnostic-build.v1',source=str(S),source_sha256=source_hash,base_build=str(B/'build.json'),diagnostic_only=True,additional_rust_cfg='mech_native_check_census',default_Object_emission_uninstrumented=True,stages=[])
for stage in j['stages']:
 if stage['crate']!='mech_gpu':
  report['stages'].append(dict(stage,reused=True));continue
 cmd=[a.replace(j['source'],str(S))for a in stage['command']];cmd[-1]=str(O/'libmech_gpu.rlib');cmd[1:1]=['--cfg','mech_native_check_census','-A','unexpected_cfgs','-L','dependency='+str(O)]
 for i,a in enumerate(cmd):
  if a.startswith('metadata='):cmd[i]='metadata=native_check_census_'+source_hash[:16]
 env={k:v.replace(j['source'],str(S))for k,v in stage['environment'].items()};start=time.monotonic();run=subprocess.run(cmd,cwd=S,env={**os.environ,**env},capture_output=True,text=True)
 receipt=dict(crate='mech_gpu',command=cmd,environment=env,source_sha256=source_hash,reused=False,exit=run.returncode,seconds=time.monotonic()-start,stdout=run.stdout,stderr=run.stderr)
 if run.returncode==0:receipt['library_sha256']=sha(cmd[-1])
 write(O/'mech_gpu.build.json',receipt);report['stages'].append(receipt);write(O/'build.json',report)
 assert run.returncode==0,run.stderr[-7000:]
print('Diagnostic GPU built; other coherent libraries unchanged',flush=True)
# Use the existing adapter's JIT route and only add an end-of-run counter dump.
control=Path('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4')
source=(control/'runner.rs').read_text();needle='    assert_eq!(memory.retained_snapshot_nodes(), 0);';assert source.count(needle)==1
source=source.replace(needle,needle+'\n    std::fs::write(std::env::var("MECH_CHECK_CENSUS_OUTPUT").unwrap(), serde_json::to_vec_pretty(&mech_gpu::native_artifact::diagnostic_native_check_census()).unwrap()).unwrap();')
source=source.replace('let catalog = linked_catalog().unwrap();','let catalog = mech_stdlib::source_catalog();')
(P/'runner.rs').write_text(source)
if not(P/'build').exists():(P/'build').symlink_to(control/'build',target_is_directory=True)
cmd=['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'runner.rs','--out',P/'runner-jit'];start=time.monotonic();run=subprocess.run(list(map(str,cmd)),capture_output=True,text=True)
write(P/'host-build.json',dict(command=list(map(str,cmd)),exit=run.returncode,seconds=time.monotonic()-start,stdout=run.stdout,stderr=run.stderr));assert run.returncode==0,run.stdout[-4000:]+run.stderr[-4000:]
print('JIT diagnostic executable built',flush=True)
