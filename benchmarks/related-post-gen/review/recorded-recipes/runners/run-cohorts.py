from pathlib import Path
import json,subprocess,time
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results'
T=Path('/private/tmp/mech-related-native-fixed-frame-arenas-tests-20261106-v2')
read=lambda p:json.loads(p.read_text())
assert read(T/'fixed-arenas-run.json')['exit']==0
test=read(T/'test-run.json');assert test['expected_failures_exact'] and '689 passed; 2 failed' in test['summary'][0]
for label in ['after','compact']:
    for w in [4,8]:
        rows=read(U/f'{label}-{w}/edges/edge-fixtures.json')
        assert len(rows)==31 and all(x['exit']==0 and x.get('exact_expected_output',x.get('exact_rust_output'))for x in rows)
assert read(U/'metrics-build.json')['exit']==0
processes=subprocess.check_output(['ps','-Ao','pid,etime,pcpu,comm'],text=True)
assert not any('rustc'in x or 'mech_gpu-tests'in x or 'mech_engine-tests'in x or 'clang'in x or 'runner-profile'in x or 'runner-pc'in x for x in processes.splitlines()),processes
(U/'timing-preflight.json').write_text(json.dumps(dict(all_qualification_terminal=True,all124_edges_exact=True,no_compiler_or_test_processes=True,process_inventory=processes),indent=2)+'\n')
for phase,n,config,suffix in [('latency',9,'comparison-config.json',''),('confirmation-latency',15,'comparison-config.json','-confirmation'),('resources',7,'resource-config.json','-resources')]:
    if phase=='resources':
        prior=read(D/'comparison-native-fixed-frame-arenas-confirmation-20261106-5000-m3.json');resource=read(U/config);names={r['name']for r in resource['implementations']}
        whole=prior['whole_process_reference']['variant'];matched=prior['matched_native_reference']['variant']
        resource['whole_process_resource_reference']=whole+'-instrumented'if whole+'-instrumented'in names else whole
        resource['resource_reference']=matched+'-instrumented'if matched+'-instrumented'in names else matched
        resource['authoritative_latency_comparison']='comparison-native-fixed-frame-arenas-confirmation-20261106-5000-m3.json'
        (U/config).write_text(json.dumps(resource,indent=2)+'\n')
    output=D/('comparison-native-fixed-frame-arenas'+suffix+'-20261106-5000-m3.json')
    command=['python3',str(R/'compare-implementations.py'),'--config',str(U/config),'--input','/private/tmp/mech-related-post-rust-local-20261102/posts.json','--samples',str(n),'--work',str(U/phase),'--output',str(output)]
    if phase=='resources':command.append('--resources')
    start=time.monotonic()
    with (U/(phase+'.stdout.log')).open('w')as out,(U/(phase+'.stderr.log')).open('w')as err:result=subprocess.run(command,stdout=out,stderr=err)
    receipt=dict(command=command,exit=result.returncode,seconds=time.monotonic()-start)
    (U/phase/'run.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(dict(phase=phase,**receipt)),flush=True);assert result.returncode==0
    report=read(output)
    assert len(report['samples'])==n*len(report['implementations'])and all(x['exit']==0 and x['exact_rust_output']for x in report['samples'])
    print(json.dumps(dict(phase=phase,exact_outputs=len(report['samples']),whole_reference=report['whole_process_reference'],matched_reference=report['matched_native_reference'],mech={name:{k:v for k,v in row.items()if k in ['median_whole_process_seconds','whole_process_over_fastest','median_post_json_seconds','post_json_over_fastest_matched_native']}for name,row in report['summary'].items()if name.startswith('mech-')})),flush=True)
