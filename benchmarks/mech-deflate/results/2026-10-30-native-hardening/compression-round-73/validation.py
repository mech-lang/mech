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
 for case,count in [('new-compute',4),('budget',108),('projection',12),('policy-order',1),('overflow-retry',1),('shape',6),('borrowed-turn',7),('full-planner',22)]:
  receipt('engine-'+phase+'/'+case+'-inventory.json',count,True);receipt('engine-'+phase+'/'+case+'.json',count)
for path in ['non-native/build.json','diagnostic-counts/gpu-build.json','diagnostic-counts/worker-build.json','diagnostic-counts/run.json']:receipt(path)
for name,count in [('budget',108),('planner',22),('memory-runtime',None),('format',None),('whitespace',None)]:receipt('restore-checks/'+name+'.json',count)
assert (r/'engine-parent/budget-inventory.stdout').read_bytes()==(r/'engine-after/budget-inventory.stdout').read_bytes()
status='pending' if any(x.get('status')=='pending' for x in rows) else ('passed' if all(x['passed'] for x in rows) else 'failed')
result={'status':status,'all_required_checks_pass':status=='passed','checks':rows,'prior_failures':json.loads((r/'prior-failures.json').read_text()),'qualification_scope':'Bounded rejected experiment:parent/candidate Budget108 including4 retained cases,planner22,turn7,projection12,shape6 and focused policy/overflow controls execute. Counts overlap. Parent executable reuse is verified by full staged source tree/library hashes; candidate root compiles freshly. No historical parent compilation is relabeled. Source restoration and three source/format checks execute after rejection.','skipped_coverage':[{'coverage':'Broader GPU runtime/component/connected/linked/scalar/frame/CI/architecture/type packages','disposition':'Deferred diagnostic-first; not started because paired elapsed and instructions reject the candidate. Complete R71 and selected R72 executions are inherited only. The1.05 scalar ceiling is unchanged; no fresh R73 scalar pass is claimed.'},{'coverage':'full hosted/all-feature CI,unchanged mutation suites,excluded complete-knucleotide case,larger inputs/full-scale timing','disposition':'Not run.'}],'scope':'All required checks means only the explicitly bounded rejected-candidate checks. No code from this candidate is retained; all raw cohorts and the original three-attribute patch remain reviewable.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'status':status,'checks':len(rows),'unpassed':[x for x in rows if not x['passed']]},indent=2))
