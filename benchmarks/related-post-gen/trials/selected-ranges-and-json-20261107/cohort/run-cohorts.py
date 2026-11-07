from pathlib import Path
import hashlib, json, os, re, subprocess, time

R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D = R/'results'
P = Path('/private/tmp/mech-related-selected-ranges-cohort-20261107-v1')
input_path = Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json')
assert hashlib.sha256(input_path.read_bytes()).hexdigest() == '500530b81cb81c5b1a5e48aa30de6a89ba0a350460ad826c4c7bb023e6e46cb8'
read = lambda p: json.loads(p.read_text())
qualified = {}
T=Path('/private/tmp/mech-related-selected-range-integration-tests-20261107-v1')
t=read(T/'test-run.json');assert t['expected_failures_exact'] and not t['new_failures'] and '884 passed; 2 failed' in t['summary'][0]
ci=read(T/'ci-selected-inventory-audit.json');assert ci['expected']==ci['listed']==ci['executed_passed']==389 and ci['all_names_match']
C=Path('/private/tmp/mech-related-selected-range-integration-regressions-20261107-v1')
for kind,count in [('scalar',128),('one-kib',5)]:
 c=read(C/(kind+'-comparison.json'));assert c['rows']==count and c['all_nontiming_fields_equal_IR240']
escaped=read(Path('/private/tmp/mech-related-selected-ranges-borrowed-json-runners-20261107-v1/escaped-field-checks/summary.json'));assert escaped['all_outputs_exact'] and escaped['fresh_processes']==32
qualified=dict(GPU_passed=884,inherited_failures=2,new_failures=0,CI_executed=389,controls_nontiming_exact=133,Rust_edge_outputs_exact=248,extended_host_differential_processes=32)

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

latency=D/'comparison-native-selected-ranges-randomized-20261107-5000-m3.json'
assert not latency.exists()
(P/'latency-start-gate.json').write_text(json.dumps(dict(quiet=True,process_snapshot=quiet(),qualification=qualified,
    seed=990252,samples=31,variants=14,primary_boundary='continuous post-JSON including setup and workers',
    secondary_boundary='fresh launch through exit'),indent=2)+'\n')
run(['python3',R/'compare-implementations.py','--config',P/'latency-config.json','--input',input_path,
     '--samples','31','--work','/private/tmp/mech-related-native-selected-ranges-latency-20261107-v1',
     '--output',latency,'--order','randomized','--seed','990252','--deduplicate-outputs'],P/'latency-run.json')
a=read(latency)
assert len(a['samples'])==434 and all(x['exact_rust_output'] and x['exit']==0 for x in a['samples'])
print(json.dumps(dict(latency_outputs_exact=434,reference=a['matched_native_reference'],whole_reference=a['whole_process_reference'])),flush=True)
resources=D/'comparison-native-selected-ranges-resources-20261107-5000-m3.json'
assert not resources.exists()
config=read(P/'resources-config.json')
def instrumented(name):
    return 'rust-standard-instrumented' if name=='rust' else name+'-instrumented'
config['resource_reference']=instrumented(a['matched_native_reference']['variant'])
config['whole_process_resource_reference']=instrumented(a['whole_process_reference']['variant'])
names={v['name'] for v in config['implementations']}
assert config['resource_reference'] in names and config['whole_process_resource_reference'] in names
(P/'resources-config.json').write_text(json.dumps(config,indent=2)+'\n')
(P/'resources-start-gate.json').write_text(json.dumps(dict(quiet=True,process_snapshot=quiet(),latency_complete_exact=434,
    seed=9902527,samples=7,variants=15,resource_timings_excluded=True),indent=2)+'\n')
run(['python3',R/'compare-implementations.py','--config',P/'resources-config.json','--input',input_path,
     '--samples','7','--work','/private/tmp/mech-related-native-selected-ranges-resources-20261107-v1',
     '--output',resources,'--order','randomized','--seed','9902527','--deduplicate-outputs','--resources'],P/'resources-run.json')
b=read(resources)
assert len(b['samples'])==105 and all(x['exact_rust_output'] and x['exit']==0 for x in b['samples'])
print(json.dumps(dict(resource_outputs_exact=105)),flush=True)
run(['python3',R/'collect-size-metrics.py','--config',P/'latency-config.json','--reference',a['whole_process_reference']['variant'],
     '--output',D/'size-native-selected-ranges-20261107.json'],P/'sizes-run.json')
print('latency, resources and sizes completed',flush=True)
