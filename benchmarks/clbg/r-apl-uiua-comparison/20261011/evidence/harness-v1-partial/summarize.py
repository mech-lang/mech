"""Admit validated full samples only; compare seconds and application LOC with Mech."""
import argparse,csv,json,math,statistics
from datetime import datetime,timezone
from pathlib import Path
from prepare import G,BENCHES,LANGUAGES,sha,save

def admitted(row,report):
 samples=row.get('samples_seconds',[]);validations=row.get('sample_validation',[])
 target=row.get('timing_alias_of') or row['key']
 evidence=[a for a in report.get('attempts',[]) if a['key']==target and a.get('full_workload') is True and a.get('valid_environment') is True and not a.get('error') and a.get('validation',{}).get('reference_match') is True and a.get('source_sha256')==row['source_sha256']]
 return (len(samples)>=3 and len(validations)==len(samples) and all(v.get('reference_match') is True for v in validations) and not row.get('full_failure') and all(isinstance(x,(int,float)) and x>0 for x in samples) and report.get('sample_count',0)>=3 and [a['seconds'] for a in evidence]==samples)

def summarize(languages=LANGUAGES):
 comparisons=[];coverage={};report_pins={}
 for language in languages:
  path=G/'results'/('timings-'+language+'.json')
  if not path.exists():continue
  report=json.loads(path.read_text());report_pins[str(path)]=sha(path)
  assert sha(report['source_audit'])==report['source_audit_sha256'],('Source audit changed',language)
  rows=report['programs'];coverage[language]={'status':report['status'],'prepared_benchmarks':10,'admitted_benchmarks':0,'failures':[],'pending':[]}
  for benchmark in BENCHES:
   candidates=[r for r in rows.values() if r['benchmark']==benchmark and r['language']==language]
   valid=[r for r in candidates if admitted(r,report)]
   failed=[{'key':r['key'],'failure':r['full_failure']} for r in candidates if r.get('full_failure')]
   pending=[{'key':r['key'],'valid_samples':len(r.get('samples_seconds',[])),'status':r['measurement_status']} for r in candidates if not admitted(r,report) and not r.get('full_failure')]
   coverage[language]['failures'].extend(failed);coverage[language]['pending'].extend(pending)
   baseline=report['baseline_endpoints'][benchmark];control=rows[baseline['control_key']]
   comparison={'language':language,'benchmark':benchmark,'prepared_candidates':len(candidates),'admitted_candidates':len(valid),'pending_candidates':pending,'failed_candidates':failed,'control_admitted':admitted(control,report),'coverage':'pending','fastest':None,'smallest':None}
   if valid and admitted(control,report):
    mech_seconds=statistics.median(control['samples_seconds'])
    fastest=min(valid,key=lambda r:(statistics.median(r['samples_seconds']),r['loc'],r['key']))
    smallest=min(valid,key=lambda r:(r['loc'],statistics.median(r['samples_seconds']),r['key']))
    def endpoint(row,role):
     seconds=statistics.median(row['samples_seconds']);mech_loc=baseline['fastest_loc' if role=='fastest' else 'smallest_loc']
     return {k:row[k] for k in ['key','id','source','source_sha256','command','loc']}|{'seconds':seconds,'samples_seconds':row['samples_seconds'],'stdev_seconds':statistics.stdev(row['samples_seconds']),'sample_validation':row['sample_validation'],'time_x_mech':seconds/mech_seconds,'loc_x_mech':row['loc']/mech_loc,'mech_seconds':mech_seconds,'mech_samples_seconds':control['samples_seconds'],'mech_loc':mech_loc,'control_key':control['key'],'timing_alias_of':row.get('timing_alias_of')}
    comparison.update(fastest=endpoint(fastest,'fastest'),smallest=endpoint(smallest,'smallest'),coverage='complete-prepared-candidates' if len(valid)==len(candidates) else 'partial-prepared-candidates')
    coverage[language]['admitted_benchmarks']+=1
   comparisons.append(comparison)
 summary={'status':'complete' if coverage and all(c['admitted_benchmarks']==10 and not c['pending'] for c in coverage.values()) else 'partial','updated_at':datetime.now(timezone.utc).isoformat(),'comparisons':comparisons,'coverage':coverage,'reports':report_pins,'admission_rule':'At least three unchanged canonical full-workload, output-validated and environment-valid fresh-process samples for the candidate and its freshly measured Mech AOT control. Partial suites and failed candidates remain explicit. Planning pilots and extrapolations are never admitted.','units':'Seconds; time_x_mech is candidate median / fresh Mech AOT median. LOC ratios compare fastest with Mech fastest LOC and smallest with Mech smallest LOC. Application code only; independent statements count separately.'}
 save(G/'results/summary.json',summary)
 flat=[];lines=['Language | Benchmark | Form | Seconds | × Mech time | LOC | × Mech LOC','--- | --- | --- | ---: | ---: | ---: | ---:']
 for comparison in comparisons:
  for role in ['fastest','smallest']:
   row=comparison[role]
   if row:
    record={'language':comparison['language'],'benchmark':comparison['benchmark'],'form':role,**{k:row[k] for k in ['seconds','time_x_mech','loc','loc_x_mech','key','source']}}
    flat.append(record);lines.append(f"{record['language']} | {record['benchmark']} | {role} | {row['seconds']:.6f}s | {row['time_x_mech']:.2f}× | {row['loc']} | {row['loc_x_mech']:.2f}×")
 for language,c in coverage.items():lines.append(f"\n{language}: {c['admitted_benchmarks']}/10 benchmarks admitted; {len(c['pending'])} candidate roles pending; {len(c['failures'])} failed roles archived.")
 (G/'results/summary.md').write_text('\n'.join(lines)+'\n')
 with (G/'results/summary.csv').open('w',newline='') as output:
  writer=csv.DictWriter(output,fieldnames=['language','benchmark','form','seconds','time_x_mech','loc','loc_x_mech','key','source']);writer.writeheader();writer.writerows(flat)
 return summary

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--languages',default=','.join(LANGUAGES));args=parser.parse_args()
 result=summarize(args.languages.split(','));print('SUMMARY',result['status'],[(k,v['admitted_benchmarks']) for k,v in result['coverage'].items()])
if __name__=='__main__':main()
