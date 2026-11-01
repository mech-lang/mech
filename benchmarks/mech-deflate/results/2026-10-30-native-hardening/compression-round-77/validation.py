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
failures=json.loads((r/'prior-failures.json').read_text()) if (r/'prior-failures.json').exists() else []
receipt('non-native/build.json')
assert (r/'engine-parent/budget-inventory.stdout').read_bytes()==(r/'engine-after/budget-inventory.stdout').read_bytes()
receipt('diagnostic-counts/gpu-build.json');receipt('diagnostic-counts/worker-build.json');receipt('diagnostic-counts/run.json')
for directory in ['prototype','prototype-borrow']:
 receipt(directory+'/build.json');receipt(directory+'/run.json')
 assert (r/directory/'run.stdout').read_text().startswith('EQUIVALENT cases=40 resident_error=40 resident_result=40 native_error=56 iteration_error=56')
for label,count in [('budget',108),('planner',22),('memory-runtime',None),('format',None),('whitespace',None)]:receipt('restored/'+label+'.json',count)
proof=json.loads((r/'restored/source-proof.json').read_text());assert proof['all_engine_gpu_ci_source_identical_to_accepted']
tail=json.loads((r/'success-tail.json').read_text());rows.append({'receipt':'success-tail.json','passed':all(tail['selected_routines_machine_bytes_identical'].values()) and tail['own_compute_frame_bytes']=={'before':176,'after':176},'scope':'Verified identical selected routine machine bytes, opcodes and stack frames; no broader executable identity claim.'})
status='pending' if any(x.get('status')=='pending' for x in rows) else ('passed' if all(x['passed'] for x in rows) else 'failed')
result={'status':status,'all_required_checks_pass':status=='passed','checks':rows,'prior_failures':failures,
 'parent_candidate_scope':'Accepted R76 and candidate each execute the same108 Budget tests and22 planner tests, with overlapping projection/shape/error-order selections. Parent executable reuse requires a complete staged-source/library match; all executions are fresh. Candidate compilation is fresh.',
 'skipped_coverage':[
 {'coverage':'full GPU runtime329, work/value32, components236, connected8, linked12, scalar1.05, frame8, CI35/Python132 and architecture/type selections','disposition':'Not launched after the main timing/counter/code audit establishes no further gain. Accepted R76 already has full92-check local qualification; no historical execution is relabeled as R77 coverage.'},
 {'coverage':'fresh full Core suite','disposition':'Core source/library frozen. Prior371-case Core coverage inherited.'},
 {'coverage':'remote CI and larger/full-scale inputs','disposition':'Not awaited/run as requested. The full pinned eight-byte computation only.'}],
 'scope':'Bounded rejected-experiment qualification and exact R76 restoration, not full candidate release qualification. Two synthetic40-case controls are separate from the108 Budget tests. Actual non-native warnings-as-errors compilation and four full diagnostic calls are retained. The analysis-reader path typo is introduced, preserved, fixed and separately classified; it is not a compiler failure.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'status':status,'checks':len(rows),'unpassed':[x for x in rows if not x['passed']]},indent=2))
