from pathlib import Path
import datetime,hashlib,json,os,re,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results';I=Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json');read=lambda p:json.loads(p.read_text())
rule=read(P/'prospective-performance-rule.json');assert rule['measurements_started']==False
assert read(Path('/private/tmp/mech-related-llvm-native-image-tests-20261107-v4/summary.json'))['CI399_passed']==399
assert read(Path('/private/tmp/mech-related-llvm-native-image-regressions-20261107-v4/scalar-comparison.json'))['all_nontiming_fields_equal_IR240']
assert read(Path('/private/tmp/mech-related-llvm-native-image-regressions-20261107-v4/one-kib-comparison.json'))['all_nontiming_fields_equal_IR240']
assert read(Path('/private/tmp/mech-related-llvm-native-image-smoke-20261107-v4/linked-correctness.json'))['exact_outputs']==64
assert read(Path('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4/summary.json'))['Rust_exact_outputs']==32
assert read(Path('/private/tmp/mech-related-llvm-native-image-loaders-budgets-20261107-v3/summary.json'))['finite_denials_exact']==48
assert read(Path('/private/tmp/mech-related-apple-m3-rust-controls-20261107-v1/qualification-summary.json'))['all_full5000_exact']
assert read(Path('/private/tmp/mech-related-native-lifecycle-trace-20261107-v1/summary.json'))['exact_outputs']==18
assert read(Path('/private/tmp/mech-related-restore-instruction-phases-20261107-v1/summary.json'))['full5000_outputs_exact']==7
assert len(read(P/'latency-config.json')['implementations'])==25
assert len(read(P/'resources-config-base.json')['implementations'])==26
def quiet(label):
 snapshot=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True);busy=[]
 for line in snapshot.splitlines()[1:]:
  parts=line.split(None,4)
  if len(parts)!=5 or int(parts[0])in[os.getpid(),os.getppid()]:continue
  if re.search(r'(?:^|/)(?:rustc|cargo|clang|clang\+\+|llc|opt|sample|gzip|xz|zstd|gpu-tests|one-kib|scalar|runner-(?:aot|metrics|pc|trace|phases))(?:\s|$)',parts[4])or 'compare-implementations.py'in parts[4]or '/build-metrics.py'in parts[4]:busy.append(dict(pid=int(parts[0]),program=parts[4].split()[0]))
 assert not busy,busy
 (P/(label+'-start-gate.json')).write_text(json.dumps(dict(quiet=True,checked_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),process_snapshot_sha256=hashlib.sha256(snapshot.encode()).hexdigest(),all_own_build_test_profile_handles_terminal=True,prospective_rule_sha256=hashlib.sha256((P/'prospective-performance-rule.json').read_bytes()).hexdigest(),native_source_commit='b638320000f7cb33837df2ee73f09221551826a0',upstream_latest='7223f308e49e44d69598be73b5770a859322da51',upstream_delta_docs_only=True),indent=2)+'\n')
def run(cmd,name):
 cmd=list(map(str,cmd));t=time.monotonic()
 with(P/(name+'.stdout.log')).open('w')as out,(P/(name+'.stderr.log')).open('w')as err:r=subprocess.run(cmd,stdout=out,stderr=err)
 receipt=dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t);(P/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==0,(P/(name+'.stderr.log')).read_text()[-5000:]
latency=D/'comparison-llvm-native-image-randomized-20261107-5000-m3.json';assert not latency.exists();quiet('latency')
run(['python3',R/'compare-implementations.py','--config',P/'latency-config.json','--input',I,'--samples','31','--work','/private/tmp/mech-related-llvm-native-image-latency-20261107-v1','--output',latency,'--order','randomized','--seed','990273','--deduplicate-outputs'],'latency-run');a=read(latency);assert len(a['samples'])==775 and all(x['exact_rust_output']and x['exit']==0 for x in a['samples']);print(json.dumps(dict(latency_exact_outputs=775,reference=a['matched_native_reference'],whole_reference=a['whole_process_reference'])),flush=True)
config=read(P/'resources-config-base.json')
def instrumented(n):return 'rust-standard-instrumented'if n=='rust'else n+'-instrumented'
config['resource_reference']=instrumented(a['matched_native_reference']['variant']);config['whole_process_resource_reference']=instrumented(a['whole_process_reference']['variant']);names={v['name']for v in config['implementations']};assert config['resource_reference']in names and config['whole_process_resource_reference']in names;(P/'resources-config.json').write_text(json.dumps(config,indent=2)+'\n')
resources=D/'comparison-llvm-native-image-resources-20261107-5000-m3.json';assert not resources.exists();quiet('resources')
run(['python3',R/'compare-implementations.py','--config',P/'resources-config.json','--input',I,'--samples','7','--work','/private/tmp/mech-related-llvm-native-image-resources-20261107-v1','--output',resources,'--order','randomized','--seed','9902737','--deduplicate-outputs','--resources'],'resources-run');b=read(resources);assert len(b['samples'])==182 and all(x['exact_rust_output']and x['exit']==0 for x in b['samples']);print('182 resource outputs exact; no resource timings used as latency',flush=True)
run(['python3',R/'collect-size-metrics.py','--config',P/'latency-config.json','--reference',a['whole_process_reference']['variant'],'--output',D/'size-llvm-native-image-20261107.json'],'sizes-run')
