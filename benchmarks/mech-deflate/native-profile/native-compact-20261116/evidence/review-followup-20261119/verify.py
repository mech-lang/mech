"""Replay raw finalist precision, preserve the adverse run and verify counters."""
from pathlib import Path
from hashlib import sha256
from collections import Counter
import gzip,json,subprocess,sys,tarfile,tempfile
E=Path(__file__).resolve().parent;P=E.parents[1];read=lambda p:json.loads(p.read_text());sha=lambda b:sha256(b).hexdigest()
sys.path.insert(0,str(P));import qualification_timings as t
from qualification_records import Evidence,Findings,load_fixtures

def main():
 policy=read(P/'qualification-policy-v3.json');fixtures=load_fixtures(Evidence(P,Findings()),policy)
 source=read(E/'repository-tests/result.json');assert source['exit_code']==0
 assert sha((E/'repository-tests/test.log').read_bytes())==source['test_log_sha256']
 assert '10 passed; 0 failed' in (E/'repository-tests/test.log').read_text()
 for name,pin in source['source_sha256'].items():assert sha(subprocess.check_output(['git','show','00765f3be:'+name],cwd=P))==pin
 replay=read(E/'no-state-source-reproduction.json');assert len(replay['variants'])==4
 candidate=read(P/'evidence/encoder-size-gap/word-complement-current-read-study/candidate.json')
 assert all(row['artifacts']==candidate['artifact_sha256'] and row['worker_sha256']==candidate['worker_sha256'] and row['no_reuse_metadata'] for row in replay['variants'])
 plan=read(E/'finalists/plan.json');runs=read(E/'finalists/result.json')['runs'];precision=read(E/'finalists/precision.json')
 t.ACTORS=tuple(plan['actors']);t.MECH=('mech-control','mech-experiment','mech-vector')
 pairs=[('mech-experiment','mech-control'),('mech-vector','mech-control'),('mech-experiment','mech-vector'),('mech-experiment','libdeflate'),('mech-vector','libdeflate'),('mech-experiment','roc'),('mech-vector','roc')]
 with tempfile.TemporaryDirectory(prefix='mech-review-replay-') as temp:
  q=Path(temp)
  for archive,members in read(E/'archive-files.json').items():
   with tarfile.open(E/archive) as tar:
    assert sorted(tar.getnames())==sorted(members)
    for name,pin in members.items():
     part=Path(name);assert not part.is_absolute() and '..' not in part.parts
     raw=tar.extractfile(name).read();assert len(raw)==pin['bytes'] and sha(raw)==pin['sha256'];target=q/part;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
  for oi,run in enumerate(runs):
   folder=q/Path(run['folder']).name;pins=read(folder/'pins.json');assert read(folder/'complete.json')['balanced_blocks_per_case']==120
   for actor,binding in plan['bindings'].items():assert pins['actors'][actor]['binary_sha256']==binding['mech-worker']['sha256'] and pins['actors'][actor]['metadata_sha256']==binding['metadata.json']['sha256']
   calculated=[]
   for index,row in enumerate(run['results']):
    name=row['case'];path=folder/name/'observations.json.gz';raw=json.loads(gzip.decompress(path.read_bytes()));blocks,_,_,_=t.case_blocks(raw,fixtures[name],120);results={}
    for pi,(a,b) in enumerate(pairs):
     strata=t.paired(blocks,a,b);assert list(map(len,strata))==[60,60];c=t.interval(strata,oi*100000+index*100+pi,policy)
     results[a+'/'+b]={'ratio':c['estimate']+1,'ratio_lower':c['lower']+1,'ratio_upper':c['upper']+1,'coverage':policy['bootstrap_coverage'],'blocks':120,'numerator_first':60,'denominator_first':60}
    calculated.append({'case':name,'raw_sha256':sha(path.read_bytes()),'pairs':results})
   assert calculated==precision['analysis'][oi]['cases']
  original=q/'vector-mismatch-clean-panel-word-reuse-complements-profiled-reverse/prefix-random-1048576/observations.json.gz';raw=json.loads(gzip.decompress(original.read_bytes()));assert len(raw)==24
  strata=t.paired([(tuple(x['order']),x['means']) for x in raw],'mech-experiment','roc');assert abs(t.estimate(strata)+1-1.0153144397854954)<1e-12
  repeat=read(E/'regime-repeat/result.json');assert repeat['complete'];folder=q/Path(repeat['folder']).name
  assert read(folder/'complete.json')['balanced_blocks_per_case']==24
  for row in repeat['results']:assert len(json.loads(gzip.decompress((folder/row['case']/'observations.json.gz').read_bytes())))==24
 for folder in ['winner-frequency','first-word-frequency']:
  data=read(E/folder/'result.json');leaders=data['leaders']
  assert sha(gzip.decompress((E/folder/'program.o.gz').read_bytes()))==data['diagnostic_object_sha256']
  for case,expected in data['cases'].items():
   raw=read(E/folder/('observations-'+case+'.json'));assert len(raw)==2
   assert all(all(row[k] for k in ['exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for row in raw)
   a,b=[x['block_counts'] for x in raw];assert a==b
   if folder=='winner-frequency':
    total=a[leaders.index(data['site'])];winner=a[1023];assert total==expected['vector_reductions'] and winner==expected['vector_winners'] and 0<=winner<=total
   else:
    sites=data['sites']
    for row in expected['sites']:
     pc=row['site'];site=sites[pc];total=a[leaders.index(pc)];fail=a[site['slot']];guard=hex(site['length_guard']);equal=a[leaders.index(guard)];short=a[sites[guard]['slot']]
     assert total==row['probes'] and fail==row['first_word_mismatches'] and equal==total-fail==row['equal_first_words'] and short==row['short_scalar_tails'] and equal-short==row['vector_entries_after_first_word']
 counts=read(E/'histogram-counts/counts.json')
 for row in counts['cases'].values():
  assert sum(b['visits']*len(b['instructions']) for b in row['blocks'])==row['counts']['instructions']
 print('Verified both 120-block finalist orders and all confidence intervals, retained adverse original and repeat, raw conditional counters, histogram instruction totals and 10 repository tests.')
if __name__=='__main__':main()
