from pathlib import Path
import subprocess,json,time,os
ROOT=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');OUT=Path(__file__).parent
BUILD=Path('/private/tmp/mech-related-conjoined-window-integration-opt-20261107-v1/build.json')
def run(command,receipt,env=None):
 command=list(map(str,command));start=time.monotonic();r=subprocess.run(command,capture_output=True,text=True,env=env)
 receipt.write_text(json.dumps(dict(command=command,exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stderr;return r
for name in ['bound','independent','full-catalog']:
 p=OUT/name;run(['python3',ROOT/'build-host.py','--compiler-build',BUILD,'--source',p/'runner.rs','--out',p/'runner-budget','--bundle',p/'build','--linked'],p/'build-run.json')
rows=[]
for case in ['work','transport','stack','output_bytes','output_elements','temporary','nodes','comparison','success']:
 results={}
 for name in ['bound','independent','full-catalog']:
  p=OUT/name
  result=run([p/'runner-budget',p/'build/source.mec',ROOT/'fixtures/posts-first-10.json',p/(case+'-output.json'),'1'],p/(case+'.json'),env={**os.environ,'MECH_IMAGE_BUDGET_CASE':case})
  value=json.loads(result.stdout.splitlines()[-1]);results[name]=value
 if case=='success':
  assert all(json.loads((OUT/'bound/success-output.json').read_text())==json.loads((OUT/n/'success-output.json').read_text()) for n in ['independent','full-catalog'])
  assert all(0 < results[n]['work'] <= 1_000_000_000_000 for n in ['bound','independent','full-catalog'])
  for key in ['parallel_regions','parallel_batches','parallel_tasks','parallel_peak_workers','parallel_numeric_regions','parallel_numeric_work']:
   assert all(results['bound'][key]==results[n][key] for n in ['independent','full-catalog']),key
 else:
  assert all(results['bound']==results[n] for n in ['independent','full-catalog']),(case,results)
  assert results['bound']['output_unpublished'] and results['bound']['retained_bytes']==0 and results['bound']['retained_nodes']==0
 rows.append(dict(case=case,denial_exact=case!='success',full_output_exact=True if case=='success' else None,results=results));print(json.dumps(dict(case=case,validated=True)),flush=True)
(OUT/'summary.json').write_text(json.dumps(dict(cases=rows,all_eight_denials_exact=True,successful_full_output_exact=True,successful_finite_work_cap=1_000_000_000_000,exact_success_work_is_not_cross_process_invariant=True,variability_evidence="v2/work-variability-audit.json",inherited_128_work_fault_still_open=True,production_assertions_and_seeds_unchanged=True,diagnostic_only=True),indent=2)+'\n')
