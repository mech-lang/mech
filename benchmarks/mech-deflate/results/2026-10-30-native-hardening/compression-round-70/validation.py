"""Summarize this round's receipts only; never execute tests or accept zero-test filters."""
from pathlib import Path
import json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);rows=[];configuration=json.loads((r/'configuration.json').read_text())
def receipt(path,count=None,inventory=False,expected_exit=0,expected_error=None):
 if not (r/path).exists():rows.append({'receipt':path,'expected_tests':count,'inventory_only':inventory,'passed':False,'status':'pending'});return
 d=json.loads((r/path).read_text());value=d.get('listed_tests' if inventory else 'executed_tests',d.get('passed_tests'));passed=d.get('exit')==expected_exit and (count is None or value==count)
 if count is not None:passed &= value is not None and value>0
 if expected_exit:
  passed &= d.get('passed_tests')==0 and d.get('failed_tests')==1
  passed &= expected_error in (r/path).with_suffix('.stderr').read_text()
 rows.append({'receipt':path,'command':d.get('command'),'exit':d.get('exit'),'expected_exit':expected_exit,'expected_tests':count,'observed_tests':value,'inventory_only':inventory,'expected_negative_control':bool(expected_exit),'passed':bool(passed),**{k:d[k] for k in ['compiler_source','test_source_head','baseline_checkpoint','phase','production_source_sha256','source_sha256','required_features'] if k in d}})
for phase in ['parent','after']:
 receipt('engine-'+phase+'/build.json')
 for case,count in [('projection',12),('policy-order',1),('overflow-retry',1),('shape',6),('borrowed-turn',7),('full-planner',22)]:
  receipt('engine-'+phase+'/'+case+'-inventory.json',count,True);receipt('engine-'+phase+'/'+case+'.json',count)
for path,count,inventory in [('units-after/build.json',None,False),('units-after/vector-address-inventory.json',1,True),('units-after/vector-address.json',1,False),('units-after/first-change-inventory.json',1,True),('units-after/first-change.json',1,False),('units-after/locals-snapshot-inventory.json',1,True),('units-after/locals-snapshot.json',1,False),('units-after/preparation-inventory.json',2,True),('units-after/preparation.json',2,False),('units-after/runtime-inventory.json',configuration['gpu_runtime_expected'],True),('units-after/runtime.json',configuration['gpu_runtime_expected'],False),('units-after/work-values-inventory.json',32,True),('units-after/work-values.json',32,False),('qualification/component-build.json',None,False),('qualification/path-mechanism-inventory.json',1,True),('qualification/path-mechanism.json',1,False),('qualification/components-inventory.json',236,True),('qualification/components.json',236,False),('qualification/composition-inventory.json',8,True),('qualification/composition.json',8,False),('ci/workflow-python.json',132,False),('ci/engine-shape-inventory.json',6,True),('ci/engine-shape-selected.json',6,False),('ci/engine-projection-inventory.json',12,True),('ci/engine-projection-selected.json',12,False),('linked-acceptance/emit-build.json',None,False),('linked-acceptance/emit.json',None,False),('linked-acceptance/worker-build.json',None,False),('linked-acceptance/run.json',None,False)]:receipt(path,count,inventory)
if (r/'ci/results.json').exists():
 ci=json.loads((r/'ci/results.json').read_text());assert len(ci)==3
 for row in ci[1:]:
  inventory='listed_tests' in row;value=row.get('listed_tests',row.get('executed_tests'));expected=configuration['gpu_ci_expected'];rows.append({'receipt':'ci/results.json','command':row['command'],'exit':row['exit'],'expected_tests':expected,'observed_tests':value,'inventory_only':inventory,'passed':row['exit']==0 and value==expected})
else:rows.append({'receipt':'ci/results.json','passed':False,'status':'pending'})
architecture=[]
if (r/'architecture/results.json').exists():
 architecture=json.loads((r/'architecture/results.json').read_text());assert len(architecture)==10
 for row in architecture:rows.append({'receipt':'architecture/results.json','command':row['command'],'exit':row['exit'],'passed':row['exit']==0})
else:rows.append({'receipt':'architecture/results.json','passed':False,'status':'pending'})
for relative,kind in [('primary/scalar-results.json','scalar'),('frame-allocation/results.json','frame'),('linked-acceptance/run.stdout','linked')]:
 if not (r/relative).exists():rows.append({'receipt':relative,'passed':False,'status':'pending'});continue
 d=json.loads((r/relative).read_text())
 if kind=='scalar':count=len(d['controls']);passed=count==12 and d['direct_controls_pass'] and d['scalar_loop_allocation_slope_zero'] and all(all(x['checks'].values()) for x in d['controls'])
 elif kind=='frame':count=len(d);passed=count==8 and all(x['no_request_byte_or_work_increase'] for x in d)
 else:count=len(d);passed=[x.get('generated_success') for x in d]==[36,30,None,None,None,17]*2
 rows.append({'receipt':relative,'cases':count,'passed':passed})
failures=json.loads((r/'prior-failures.json').read_text()) if (r/'prior-failures.json').exists() else []
architecture_initial=json.loads((r/'type-architecture/results.json').read_text())
for index,row in enumerate(architecture_initial):rows.append({'receipt':'type-architecture/results.json','command':row['command'],'exit':row['exit'],'passed':row['exit']==0})
receipt('diagnostic-counts/gpu-build.json');receipt('diagnostic-counts/worker-build.json');receipt('diagnostic-counts/run.json')
status='pending' if any(x.get('status')=='pending' for x in rows) else ('passed' if all(x['passed'] for x in rows) else 'failed')
result={'status':status,'all_required_checks_pass':status=='passed','checks':rows,'architecture_initial':architecture_initial,'architecture_final':architecture,'prior_failures':failures,'parent_candidate_scope':'Both exact parent and candidate Engine implementations execute the same 22 planner tests, including three new current-output, finite-limit, unselected-branch, overflow/error and retry cases. The complete Core runtime implementation and compiled Core library are unchanged; R69 Core371 is inherited evidence, not relabeled as a fresh R70 run.','runtime_coverage':{'receipt':'units-after/runtime.json','required_executed_tests':configuration['gpu_runtime_expected'],'scope':'Fresh full R70 runtime execution is required. Historical batches are not counted. Projection12, shape6, planner22 and GPU CI selections are additional overlapping batches, not a sum of unique tests.'},'skipped_coverage':[{'coverage':'complete source-checker mutation suites','disposition':'These checker implementations and their mutation tests are unchanged from the accepted parent. Their production source checks execute; their complete mutation suites are not rerun in this bounded call-port experiment.'},{'test':'complete_unchanged_knucleotide_body_uses_the_owned_emitted_map','disposition':'Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.'},{'coverage':'full remote CI','disposition':'Not awaited as requested. Local selected tests with their actual retained feature configurations and architecture checks are reported separately.'},{'coverage':'larger compression inputs/full-scale timing','disposition':'Not run; this optimization measures the complete pinned eight-byte level-12 call only.'}],'scope':'Owned-plan candidate qualification; accepted parent and candidate execute the same corrected planner suite. Diagnostic event counts are separate instrumented observations, not performance claims. Both parent and candidate execute the same current Core tests; historical observations are not relabeled as current executions. Observed inventory/execution counts only. Failed initial attempts retain their actual provenance and disposition; they are neither erased nor accepted as successful qualification.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'status':status,'checks':len(rows),'unpassed':[x for x in rows if not x['passed']]},indent=2))
