"""Carry execution-identical evidence across an explicitly approved refreeze."""
import argparse,copy,json
from datetime import datetime,timezone
from pathlib import Path
from prepare import G,LANGUAGES,sha,save,support_pins

METRICS=['samples_seconds','sample_validation','attempts','median_seconds','sample_stdev_seconds','full_validation','full_failure','measurement_status','aliased_sample_source']
def equivalent(before,after):
 # New audit pins can be added, but an existing runtime/source pin cannot change.
 fields=['key','benchmark','language','source_sha256','binary_sha256','command','cwd','runtime_environment','loc','timing_alias_of']
 if any(before.get(k)!=after.get(k) for k in fields):return False
 old=support_pins(before);new=support_pins(after)
 return all(new.get(p)==digest for p,digest in old.items())

def migrate(language,archive,reason):
 path=G/'results'/('timings-'+language+'.json')
 if not path.exists():return
 report=json.loads(path.read_text());old_audit_path=archive/('source-audit-'+language+'.json');new_path=G/('source-audit-'+language+'.json')
 old=json.loads(old_audit_path.read_text());new=json.loads(new_path.read_text())
 assert report['source_audit_sha256']==sha(old_audit_path),('Old report/audit mismatch',language)
 before=report['programs'];after={r['key']:dict(r,samples_seconds=[],sample_validation=[],attempts=[],measurement_status='pending') for r in new['programs']+new['controls']}
 preserved=[];retired={}
 for key,row in before.items():
  if key in after and equivalent(row,after[key]):
   for field in METRICS:
    if field in row:after[key][field]=copy.deepcopy(row[field])
   preserved.append(key)
  else:retired[key]=row
 kept=[];mapping={};historical=[]
 for attempt in report['attempts']:
  if attempt['key'] in preserved:
   attempt=copy.deepcopy(attempt);old_number=attempt['attempt_number'];attempt['prior_guard_revision_attempt_number']=old_number;attempt['attempt_number']=len(kept)+1;mapping[old_number]=attempt['attempt_number'];kept.append(attempt)
  else:historical.append(attempt)
 for key in preserved:
  after[key]['attempts']=[mapping[x] for x in after[key]['attempts'] if x in mapping]
  if after[key].get('full_failure'):after[key]['full_failure']['attempt_number']=mapping[after[key]['full_failure']['attempt_number']]
 report.setdefault('historical_attempts',[]).extend(historical)
 report.setdefault('retired_candidates',{}).update(retired)
 report.setdefault('guard_revisions',[]).append({'at':datetime.now(timezone.utc).isoformat(),'prior_source_audit':str(old_audit_path),'prior_source_audit_sha256':sha(old_audit_path),'prior_report':str(archive/path.name),'prior_report_sha256':sha(archive/path.name),'reason':reason,'preserved_keys':preserved,'retired_keys':list(retired),'new_keys':[k for k in after if k not in preserved]})
 report.update(status='running',programs=after,attempts=kept,discarded=[mapping[x] for x in report['discarded'] if x in mapping],source_audit=str(new_path),source_audit_sha256=sha(new_path),protocol=new['timing_boundary'],baseline_endpoints=new['baseline_endpoints'],completed_benchmarks=[],orders={},updated_at=datetime.now(timezone.utc).isoformat())
 report.pop('active',None)
 planning_path=G/'results'/('runtime-planning-'+language+'.json')
 if planning_path.exists():report['runtime_planning_pilots']=json.loads(planning_path.read_text())['runtime_planning_pilots']
 save(path,report)
 print('RESUMED',language,'preserved',len(preserved),'executions;',sum(len(r['samples_seconds']) for r in after.values()),'role samples; retired',len(retired),flush=True)

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--languages',default=','.join(LANGUAGES));parser.add_argument('--archive',default=str(G/'evidence/harness-v1-partial'));parser.add_argument('--reason',default='Root-approved termination/group completion and source/library pin repair. Existing source, command, input, runtime and output evidence preserved only for identical executions; new/changed candidates start with no samples. Original guard was weaker but no bad sample or runtime relink was observed.');args=parser.parse_args()
 for language in args.languages.split(','):migrate(language,Path(args.archive).resolve(),args.reason)
if __name__=='__main__':main()
