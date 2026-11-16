from pathlib import Path
import copy,json,subprocess,time
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');read=lambda p:json.loads(p.read_text())
command=['python3',str(R/'build-metrics.py'),'--config',str(U/'screen-comparison-config.json'),'--out',str(U/'extra-metrics')]
for name in ['mech-ir238-original-8','mech-ir238-compact-4','mech-ir238-compact-8']:command.extend(['--variant',name])
start=time.monotonic();r=subprocess.run(command,capture_output=True,text=True);(U/'extra-metrics-build-run.json').write_text(json.dumps(dict(command=command,exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr
B=Path('/private/tmp/mech-related-local-facts-integration-runners-20261107-v1');S=Path('/private/tmp/mech-related-native-frozen-uniform-layouts-runners-20261106-v2');cfg=read(B/'resource-config.json');cfg['implementations']=[v for v in cfg['implementations']if v['kind']!='mech']
selected=copy.deepcopy(next(v for v in read(S/'resource-config.json')['implementations']if v['name']=='mech-after-4-instrumented'));selected.update(name='mech-selected-ir208-original-4-instrumented',category='IR208_selected_original_four_instrumented');cfg['implementations'].append(selected)
for ir,root in [(236,B),(238,U)]:
 for label in ['original','reuse']:
  v=copy.deepcopy(next(x for x in read(root/'resource-config.json')['implementations']if x['name']==f'mech-current-{label}-4-instrumented'));v.update(name=f'mech-ir{ir}-{label}-4-instrumented',category=f'IR{ir}_{label}_four_instrumented');cfg['implementations'].append(v)
for folder in ['extra-metrics','source-experiment-metrics']:
 cfg['implementations'].extend(v for v in read(U/folder/'metrics-config.json')['implementations']if v['kind']=='mech')
assert len(cfg['implementations'])==15
cfg.update(authoritative_latency_comparison='comparison-native-canonical-schema-sinks-randomized-20261107-5000-m3.json',experiment='IR238 canonical schema encoding plus independent source/worker screen; selectedIR208 and parentIR236 retained. References will be pinned from completed same-cohort latency.')
(U/'screen-resource-config.json').write_text(json.dumps(cfg,indent=2)+'\n');print('All15 separate resource binaries prepared; no timing cohort started',flush=True)
