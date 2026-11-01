"""Replay actual checks; reject pending/zero-test coverage and retain failed attempts."""
from pathlib import Path
import json,re,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);checks=[]
def check(path,count=None,inventory=False):
 p=r/path
 if not p.exists():checks.append({'receipt':path,'passed':False,'status':'pending'});return
 d=json.loads(p.read_text());ok=d.get('exit')==0;observed=None
 if count is not None:
  raw=p.with_suffix('.stdout').read_text()
  if inventory:observed=len(re.findall(r': test$',raw,re.M))
  else:
   m=re.search(r'test result: ok. (\d+) passed; 0 failed;',raw);observed=int(m[1]) if m else None
  ok &= observed==count and count>0
 checks.append({'receipt':path,'command':d.get('command'),'exit':d.get('exit'),'expected_tests':count,'observed_tests':observed,'inventory_only':inventory,'passed':bool(ok)})
check('units-final/build.json')
for label in ['vector-address','first-change','locals-snapshot','new-0','new-1']:
 check('units-final/'+label+'-inventory.json',1,True);check('units-final/'+label+'.json',1)
for label,count in [('runtime',331),('work-values',32),('ci',37)]:
 check('units-final/'+label+'-inventory.json',count,True);check('units-final/'+label+'-run.json',count)
for label,count in [('components',236),('composition',8),('path-mechanism',1)]:
 check('qualification/'+label+'-inventory.json',count,True);check('qualification/'+label+'.json',count)
check('qualification/component-build.json');check('linked-acceptance/worker-build.json');check('linked-acceptance/run.json')
linked=json.loads((r/'linked-acceptance/run.stdout').read_text());assert len(linked)==12
checks.append({'receipt':'linked-acceptance/run.stdout','observed_cases':len(linked),'passed':True,'scope':'Original/decoded valid-failure-retry source assertions, arithmetic, alias and publication checks, as asserted by the actual linked worker.'})
scalar=json.loads((r/'primary/scalar-results.json').read_text());checks.append({'receipt':'primary/scalar-results.json','observed_controls':len(scalar['controls']),'passed':len(scalar['controls'])==12 and all(all(x['checks'].values()) for x in scalar['controls']),'ceiling':1.05})
frames=json.loads((r/'frame-allocation/results.json').read_text());checks.append({'receipt':'frame-allocation/results.json','observed_cases':len(frames),'passed':len(frames)==8 and all(x['no_request_byte_or_work_increase'] for x in frames)})
for family,count in [('projection',12),('shape',6)]:
 for kind,inventory in [('inventory',True),('selected',False)]:check('ci/engine-'+family+'-'+kind+'.json',count,inventory)
check('ci/workflow-python.json');python=json.loads((r/'ci/workflow-python.json').read_text());assert python['executed_tests']==132
for directory,count in [('architecture-final',10),('type-architecture',5)]:
 p=r/directory/'results.json'
 if not p.exists():checks.append({'receipt':str(p.relative_to(r)),'passed':False,'status':'pending'});continue
 rows=json.loads(p.read_text());checks.append({'receipt':str(p.relative_to(r)),'observed_checks':len(rows),'commands':[x['command'] for x in rows],'passed':len(rows)==count and all(x['exit']==0 for x in rows)})
for label in ['format','whitespace']:check('final-source/'+label+'.json')
check('final-checks/runtime-only.json');check('parent-controls/run.json',3)
for label in ['gpu-build','worker-build','run']:check('diagnostic-counts/'+label+'.json')
proof_path=r/'final-source/proof.json'
if proof_path.exists():
 proof=json.loads(proof_path.read_text());checks.append({'receipt':'final-source/proof.json','passed':proof['production_files_identical'] and proof['non_gpu_source_identical'] and proof['test_boundary_allowlist_unchanged']})
else:checks.append({'receipt':'final-source/proof.json','passed':False,'status':'pending'})
failures=json.loads((r/'prior-failures.json').read_text());pending=any(x.get('status')=='pending' for x in checks);status='pending' if pending else ('passed' if all(x['passed'] for x in checks) else 'failed')
result={'status':status,'all_required_checks_pass':status=='passed','checks':checks,'prior_failures':failures,'inherited_coverage':[{'coverage':'371 Core tests and108 Budget/22 planner tests at accepted R76','reason':'Their complete production source and dependency artifacts are unchanged and hash-pinned. Not relabeled as fresh R78 executions.'}],'skipped_coverage':[{'coverage':'complete_unchanged_knucleotide_body_uses_the_owned_emitted_map','reason':'Existing large unrelated test excluded from the maintained bounded runtime inventory.'},{'coverage':'larger compressor inputs / full-scale timing','reason':'User requested small-scale timings only.'},{'coverage':'remote completed CI','reason':'Publication CI is cancelled at user request; no green-CI claim.'}],'scope':'Fresh final GPU test binary after test-owner placement, unchanged measured production from14d0c11. Component, linked, scalar and allocation qualifications use that exact measured release. Test migration changes neither expected algorithmic results nor resource allowances. Inventories and executions kept distinct; overlapping test selections are not summed as unique tests.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'status':status,'checks':len(checks),'unpassed':[x for x in checks if not x['passed']]},indent=2))
