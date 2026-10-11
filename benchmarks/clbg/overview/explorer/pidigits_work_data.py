"""Retire Node's one-extract pi surrogate and admit its measured correction."""
import math
import statistics
from pathlib import Path
COHORT='pidigits-work-audit/20261011'
def add_pidigits_work_audit(root,data,load,measured,sha):
 report_path=root/COHORT/'results/timings.json'
 if not report_path.exists():return
 report=load(COHORT+'/results/timings.json')
 if report.get('status')!='complete':return
 audit=load(COHORT+'/audit.json')
 assert report['source_audit_sha256']==sha(root/COHORT/'audit.json')
 assert report['sample_count']>=3 and report['power_profile']==dict(source='AC Power',low_power_mode=0)
 snapshot=load(COHORT+'/evidence/control-audit-snapshot.json')
 assert sha(root/COHORT/'evidence/control-audit-snapshot.json')==report['control_audit_sha256']
 correction=audit['corrections'][0];row=report['programs'][correction['key']]
 control_key=next(r['key'] for r in snapshot['controls'] if r['benchmark']=='pidigits')
 control=report['programs'][control_key]
 expected_control=next(r for r in snapshot['controls'] if r['key']==control_key)
 reference=root/'pidigits/inputs/output-full.txt';reference_bytes=reference.stat().st_size;reference_sha=sha(reference)
 for actual,expected in ((row,correction),(control,expected_control)):
  for field in ('source','source_sha256','command','loc','runtime_environment'):
   assert actual.get(field)==expected.get(field),(actual['key'],field)
  assert sha(actual['source'])==actual['source_sha256']
  samples=actual['samples_seconds'];n=report['sample_count']
  assert len(samples)==len(actual['sample_validation'])==len(actual['sample_details'])==n
  assert all(math.isfinite(s) and s>0 for s in samples)
  assert actual['median_seconds']==statistics.median(samples)
  assert actual['sample_stdev_seconds']==statistics.stdev(samples)
  command=[s.replace('{n}','10000') for s in actual['command']]
  for seconds,validation,detail in zip(samples,actual['sample_validation'],actual['sample_details']):
   assert detail['key']==actual['key'] and detail['source_sha256']==actual['source_sha256']
   assert detail['command']==command and detail['cwd']==actual['cwd']
   assert detail['seconds']==seconds and detail['workload_n']==10000 and detail['full_workload']
   assert detail['valid_environment'] and detail['exit_status']==0
   assert detail['competing_compilers']==detail['power_changes']==[]
   assert detail['power_at_end']==report['power_profile']
   assert detail['validation']==validation
   assert validation==dict(bytes=reference_bytes,sha256=reference_sha,reference_match=True)
 comparison=dict(seconds=row['median_seconds'],time_x_mech=row['median_seconds']/control['median_seconds'],loc=row['loc'],loc_x_mech=row['loc']/control['loc'],mech_seconds=control['median_seconds'],mech_loc=control['loc'])
 assert report['comparison']==comparison
 for file,digest in report['pins'].items():
  if file==report['control_audit']:continue # immutable snapshot above replaces the advancing cohort path
  assert sha(file)==digest,file
 assert correction['algorithm']['strict'] and not correction['algorithm']['native_benchmark_kernel']
 measured(row['key'],'pidigits','javascript',row['loc'],row['samples_seconds'],row['source'],row['source_sha256'],COHORT+'/results/timings.json',COHORT+'/audit.json',
  algorithm=correction['algorithm'],required_work_validated=True,loc_metric='statement-normalized',physical_loc=correction['physical_loc'],loc_expansions=[],source_url=correction['source_url'],local_adaptation=True,adaptations=correction['adaptations'],runtime_policy='Node.js '+correction['runtime']['version'],timing_boundary=report['protocol'],
  mech_comparison=dict(seconds=control['median_seconds'],fastest_loc=control['loc'],smallest_loc=control['loc'],time_x_mech=comparison['time_x_mech'],fastest_loc_x_mech=comparison['loc_x_mech'],smallest_loc_x_mech=comparison['loc_x_mech']))
 group=next(g for b in data['benchmarks'] if b['key']=='pidigits' for g in b['groups'] if g['language']=='javascript')
 rejected=set(correction['replaces_keys'])
 group['programs']=[k for k in group['programs'] if k not in rejected]
 if row['key'] not in group['programs']:group['programs'].append(row['key'])
 for old in audit['rejected_endpoints']:
  if old['key'] not in rejected:continue
  data['programs'].pop(old['key'],None)
  data['exclusions'].append(dict(key=old['key'],benchmark='pidigits',language='javascript',reason='One exact quotient plus remainder surrogate omits extract(4); replaced by independently measured two-extract source.',source_sha256=old['source_sha256']))
 data['pidigits_work_audit']=dict(source_audit=COHORT+'/audit.json',endpoint_count=audit['endpoint_count'],distinct_application_source_count=audit['distinct_application_source_count'],rejected=list(rejected),correction=row['key'],sample_count=report['sample_count'],comparison=comparison,policy=audit['established_policy']['rule'])
 data['protocol']['pidigits_extracts']=audit['established_policy']['rule']+' Node.js #2 was corrected and freshly timed; its prior one-extract samples are excluded.'
