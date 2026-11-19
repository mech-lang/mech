"""Strictly replay all five actors, then compare pairs within actor-order strata."""
from pathlib import Path
from hashlib import sha256
import gzip,json,sys
N=Path(__file__).resolve().parent;P=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116');sys.path.insert(0,str(P));import qualification_timings as t
from qualification_records import Evidence,Findings,load_fixtures
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(p.read_bytes()).hexdigest();policy=read(P/'qualification-policy-v3.json');fixtures=load_fixtures(Evidence(P,Findings()),policy);out=N/'finalist-precision-20261119';data=read(out/'result.json');plan=data['plan']
t.ACTORS=tuple(plan['actors']);t.MECH=('mech-control','mech-experiment','mech-vector')
pairs=[('mech-experiment','mech-control'),('mech-vector','mech-control'),('mech-experiment','mech-vector'),('mech-experiment','libdeflate'),('mech-vector','libdeflate'),('mech-experiment','roc'),('mech-vector','roc')];runs=[]
for order_index,run in enumerate(data['runs']):
 root=Path(run['folder']);pins=read(root/'pins.json');complete=read(root/'complete.json');assert complete['cases']==19 and complete['balanced_blocks_per_case']==120
 for actor,binding in plan['bindings'].items():assert pins['actors'][actor]['binary_sha256']==binding['mech-worker']['sha256'] and pins['actors'][actor]['metadata_sha256']==binding['metadata.json']['sha256']
 rows=[]
 for index,summary in enumerate(run['results']):
  name=summary['case'];raw_path=root/name/'observations.json.gz';raw=json.loads(gzip.decompress(raw_path.read_bytes()));blocks,timings,peak,identity=t.case_blocks(raw,fixtures[name],120);results={}
  for pair_index,(a,b) in enumerate(pairs):
   strata=t.paired(blocks,a,b);assert list(map(len,strata))==[60,60]
   confidence=t.interval(strata,order_index*100000+index*100+pair_index,policy)
   results[a+'/'+b]={'ratio':confidence['estimate']+1,'ratio_lower':confidence['lower']+1,'ratio_upper':confidence['upper']+1,'coverage':policy['bootstrap_coverage'],'blocks':120,'numerator_first':60,'denominator_first':60}
  rows.append({'case':name,'raw_sha256':sha(raw_path),'pairs':results})
 runs.append({'order':run['order'],'cases':rows});print('Replayed',run['order'],flush=True)
report={'analysis_source_sha256':sha(Path(__file__)),'policy_sha256':sha(P/'qualification-policy-v3.json'),'analysis':runs,'scope':'Five-actor 19-case precision comparison; every raw timed call and balanced permutation checked. Pairwise geometric means and confidence intervals resample whole blocks separately by numerator/denominator order. No full-score promotion or attribution to isolated component flags.'}
(out/'precision.json').write_text(json.dumps(report,indent=2)+'\n')
