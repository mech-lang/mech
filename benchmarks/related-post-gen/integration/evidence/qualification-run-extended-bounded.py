from pathlib import Path
import json,subprocess,time,hashlib
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results'
work=U/'extended-bounded-latency';output=D/'comparison-native-latest-deflate-integration-extended-bounded-20261106-5000-m3.json'
processes=subprocess.check_output(['ps','-Ao','pid,etime,pcpu,comm'],text=True)
assert not any(any(name in line for name in ['rustc','mech_gpu-tests','mech_engine-tests','runner-pc','runner-profile','clang'])for line in processes.splitlines())
cmd=['python3',str(R/'compare-implementations.py'),'--config',str(U/'comparison-config.json'),'--input','/private/tmp/mech-related-post-rust-local-20261102/posts.json','--samples','31','--work',str(work),'--output',str(output),'--deduplicate-outputs']
t=time.monotonic()
with (U/'extended-bounded-latency.stdout.log').open('w')as o,(U/'extended-bounded-latency.stderr.log').open('w')as e:r=subprocess.run(cmd,stdout=o,stderr=e)
d=dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,harness_sha256=hashlib.sha256((R/'compare-implementations.py').read_bytes()).hexdigest());(work/'run.json').write_text(json.dumps(d,indent=2)+'\n');assert r.returncode==0
report=json.loads(output.read_text());assert len(report['samples'])==1395 and all(x['exit']==0 and x['exact_rust_output']for x in report['samples'])
print(json.dumps(dict(exact_outputs=len(report['samples']),matched=report['matched_native_reference'],whole=report['whole_process_reference'],mech={k:{f:v[f] for f in ['median_post_json_seconds','median_whole_process_seconds','post_json_over_fastest_matched_native','whole_process_over_fastest']}for k,v in report['summary'].items()if k.startswith('mech-')})),flush=True)
p=U/'compact-extended-bounded-outputs.py';base=(U/'compact-outputs.py').read_text();p.write_text(base.replace("['latency','confirmation-latency','resources']","['extended-bounded-latency']"));subprocess.run(['python3',str(p)],check=True)
