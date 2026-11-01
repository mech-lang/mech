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
 for case,count in [('new-compute',4),('budget',108 if phase=='parent' else 113),('projection',12),('policy-order',1),('overflow-retry',1),('shape',6),('borrowed-turn',7),('full-planner',22)]:
  receipt('engine-'+phase+'/'+case+'-inventory.json',count,True);receipt('engine-'+phase+'/'+case+'.json',count)
receipt('engine-after/scoped-context-inventory.json',5,True);receipt('engine-after/scoped-context.json',5)
failures=json.loads((r/'prior-failures.json').read_text())
architecture=[]
architecture_initial=[]
receipt('non-native/build.json')
for name in ['valid','valid-run','escape','send']:
 d=json.loads((r/'borrow-controls'/(name+'.json')).read_text());expected=d.get('expected_exit',0);passed=d['exit']==expected
 if d.get('expected_error'):passed &= d['expected_error'] in (r/'borrow-controls'/(name+'.stderr')).read_text()
 if name=='valid-run':passed &= (r/'borrow-controls/valid-run.stdout').read_text()==d['expected_stdout']
 rows.append({'receipt':'borrow-controls/'+name+'.json','command':d['command'],'exit':d['exit'],'expected_exit':expected,'expected_negative_type_control':bool(expected),'passed':bool(passed),'scope':'Actual Rust consumer; lifetime/thread errors are required compile-time rejections.'})
receipt('diagnostic-counts/gpu-build.json');receipt('diagnostic-counts/worker-build.json');receipt('diagnostic-counts/run.json')
for label,count in [('budget',108),('planner',22),('memory-runtime',None),('format',None),('whitespace',None)]:receipt('restored/'+label+'.json',count)
proof=json.loads((r/'restored/source-proof.json').read_text());assert proof['all_engine_gpu_ci_source_identical_to_accepted']
status='pending' if any(x.get('status')=='pending' for x in rows) else ('passed' if all(x['passed'] for x in rows) else 'failed')
result={
 'status':status,'all_required_checks_pass':status=='passed','checks':rows,
 'prior_failures':failures,
 'parent_candidate_scope':'Fresh parent executions after whole-tree/library-verified reuse of its unchanged Rust unit binary. Fresh candidate compilation. Budget108 versus113, planner22, projection12, shape6 and the new five scoped-context controls are overlapping selections, not a sum of unique tests.',
 'skipped_coverage':[
  {'coverage':'full GPU runtime, component236, connected8, linked12, scalar1.05, frame-allocation controls, architecture and CI selections','disposition':'Not launched after the primary and counter cohorts rejected R75. Required before accepting any successor; no R75 pass is inferred from historical coverage.'},
  {'coverage':'fresh full Core unit suite','disposition':'Core source and libraries are frozen; prior Core371 is inherited.'},
  {'coverage':'remote CI and larger inputs','disposition':'Not awaited/run as requested. Complete eight-byte compression only.'}
 ],
 'scope':'Bounded rejected R75 experiment, not full release qualification. Retains actual fresh Engine and Rust borrow/type controls, non-native warnings-as-errors compilation and four full diagnostic compression calls. The R74 comparator is included in the primary triple only. Production restoration is documented separately after all diagnostic calls finish. No acceptance is claimed.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'status':status,'checks':len(rows),'unpassed':[x for x in rows if not x['passed']]},indent=2))
