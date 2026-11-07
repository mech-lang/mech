from pathlib import Path
import hashlib,json,os,re,subprocess,time
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results';P=Path(__file__).parent
input_path=Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json')
assert hashlib.sha256(input_path.read_bytes()).hexdigest()=='500530b81cb81c5b1a5e48aa30de6a89ba0a350460ad826c4c7bb023e6e46cb8'
read=lambda p:json.loads(p.read_text())
Q=Path('/private/tmp/mech-related-ordered-rust-controls-20261107-v1');q=read(Q/'qualification-summary.json');assert q['all_exact']and q['exact_outputs']==192 and q['variants']==6 and q['cases_each']==32
for name,v in read(Q/'builds.json').items():
 t=read(Q/name/'test.json');assert t['exit']==0 and '4 passed; 0 failed'in t['stdout']
 assert hashlib.sha256(Path(v['executable']).read_bytes()).hexdigest()==v['executable_sha256']
qualified=dict(new_Rust_variants=5,source_unit_tests=20,exact_differential_outputs=192,unchanged_old_reference_checked=True,all_setup_inside_post_JSON=True,Mech_compiler_unchanged='IR242/four native workers; prior full qualification retained')

def quiet():
    snapshot=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True)
    own={os.getpid(),os.getppid()}
    busy=[]
    for line in snapshot.splitlines()[1:]:
        parts=line.split(None,4)
        if len(parts)!=5 or int(parts[0]) in own:
            continue
        command=parts[4]
        if re.search(r'(?:^|/)(?:rustc|cargo|clang|clang\+\+|sample)(?:\s|$)',command) or re.search(r'/mech_gpu-tests(?:\s|$)|/one-kib(?:\s|$)|/runner-(?:aot|metrics|profile)(?:\s|$)|compare-implementations\.py|build-metrics\.py',command):
            busy.append(line)
    assert not busy, busy
    return snapshot

def run(cmd, receipt):
    start=time.monotonic()
    with receipt.with_suffix('.stdout.log').open('w') as stdout, receipt.with_suffix('.stderr.log').open('w') as stderr:
        x=subprocess.run(list(map(str,cmd)),stdout=stdout,stderr=stderr)
    result=dict(command=list(map(str,cmd)),exit=x.returncode,seconds=time.monotonic()-start)
    receipt.write_text(json.dumps(result,indent=2)+'\n')
    assert x.returncode==0, result

latency=D/'comparison-ordered-rust-controls-randomized-20261107-5000-m3.json'
assert not latency.exists()
(P/'latency-start-gate.json').write_text(json.dumps(dict(quiet=True,process_snapshot=quiet(),qualification=qualified,
    seed=990270,samples=31,variants=11,primary_boundary='continuous post-JSON including setup and workers',
    secondary_boundary='fresh launch through exit'),indent=2)+'\n')
run(['python3',R/'compare-implementations.py','--config',P/'latency-config.json','--input',input_path,
     '--samples','31','--work','/private/tmp/mech-related-ordered-rust-controls-latency-20261107-v1',
     '--output',latency,'--order','randomized','--seed','990270','--deduplicate-outputs'],P/'latency-run.json')
a=read(latency)
assert len(a['samples'])==341 and all(x['exact_rust_output'] and x['exit']==0 for x in a['samples'])
print(json.dumps(dict(latency_outputs_exact=341,reference=a['matched_native_reference'],whole_reference=a['whole_process_reference'])),flush=True)
resources=D/'comparison-ordered-rust-controls-resources-20261107-5000-m3.json'
assert not resources.exists()
config=read(P/'resources-config.json')
def instrumented(name):
    return 'rust-standard-instrumented' if name=='rust' else name+'-instrumented'
config['resource_reference']=instrumented(a['matched_native_reference']['variant'])
config['whole_process_resource_reference']=instrumented(a['whole_process_reference']['variant'])
names={v['name'] for v in config['implementations']}
assert config['resource_reference'] in names and config['whole_process_resource_reference'] in names
(P/'resources-config.json').write_text(json.dumps(config,indent=2)+'\n')
(P/'resources-start-gate.json').write_text(json.dumps(dict(quiet=True,process_snapshot=quiet(),latency_complete_exact=341,
    seed=9902707,samples=7,variants=12,resource_timings_excluded=True),indent=2)+'\n')
run(['python3',R/'compare-implementations.py','--config',P/'resources-config.json','--input',input_path,
     '--samples','7','--work','/private/tmp/mech-related-ordered-rust-controls-resources-20261107-v1',
     '--output',resources,'--order','randomized','--seed','9902707','--deduplicate-outputs','--resources'],P/'resources-run.json')
b=read(resources)
assert len(b['samples'])==84 and all(x['exact_rust_output'] and x['exit']==0 for x in b['samples'])
print(json.dumps(dict(resource_outputs_exact=84)),flush=True)
run(['python3',R/'collect-size-metrics.py','--config',P/'latency-config.json','--reference',a['whole_process_reference']['variant'],
     '--output',D/'size-ordered-rust-controls-20261107.json'],P/'sizes-run.json')
print('latency, resources and sizes completed',flush=True)
