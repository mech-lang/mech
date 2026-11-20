"""Verify exact artifacts, preserved adverse data and every prior named peer win."""
from pathlib import Path
from hashlib import sha256
from runpy import run_path
import gzip,json,subprocess,sys,tarfile,tempfile
E=Path(__file__).resolve().parent;P=E.parents[2];read=lambda p:json.loads(p.read_text());digest=lambda b:sha256(b).hexdigest()
sys.path.insert(0,str(P));import qualification_timings as t
from qualification_records import Evidence,Findings,load_fixtures

def main():
 c=read(E/'candidate.json');result=read(E/'result.json');assert c['encoder_text_bytes']==63612
 replay=E/'current-source-replay'
 for name,pin in read(replay/'files.json').items():
  raw=(replay/name).read_bytes();assert len(raw)==pin['bytes'] and digest(raw)==pin['sha256']
 build=read(replay/'compiler-build.json');assert build['exit_code']==0 and build['source_base']==c['source_integration_commit']
 for name,pin in build['source_sha256'].items():assert digest((replay/'source-inputs'/name).read_bytes())==pin
 public=read(replay/'source-reproduction.json');assert public['source_sha256']==c['source_sha256'] and [x['mode'] for x in public['variants']]==['normal','checked']
 for mode in public['variants']:assert mode['compiler_sha256']==build['compiler_sha256'] and mode['artifacts']==c['artifact_sha256']
 named=read(replay/'named-rebuild.json');assert named['variant']=='m1-bounded-byte-histogram' and named['compiler_sha256']==build['compiler_sha256']
 assert named['object_sha256']==named['expected_object_sha256']==c['artifact_sha256']['program.o'] and named['metadata_sha256']==named['expected_metadata_sha256']==c['artifact_sha256']['metadata.json'] and named['bytecode_sha256']==c['artifact_sha256']['program.mecb'] and named['object_text_bytes']==63612
 for name in ['program.o','metadata.json']:assert digest(gzip.decompress((E/(name+'.gz')).read_bytes()))==c['artifact_sha256'][name]
 assert digest(gzip.decompress((E/'worker-mech-worker.gz').read_bytes()))==c['worker_sha256']
 assert digest((P/'program.mec').read_bytes())==c['source_sha256']
 text_size=run_path(str(P/'evidence/vector-mismatch-20261119/verify.py'),run_name='histogram_text_size')['text_size'];assert text_size(gzip.decompress((E/'program.o.gz').read_bytes()))==63612
 for profile in c['profiles']:assert digest((E/profile['path']).read_bytes())==profile['sha256']
 for name,pin in c['source_inputs'].items():assert digest(subprocess.check_output(['git','show',c['source_integration_commit']+':'+name],cwd=P))==pin
 policy=read(P/'qualification-policy-v3.json');fixtures=load_fixtures(Evidence(P,Findings()),policy)
 with tempfile.TemporaryDirectory(prefix='mech-histogram-replay-') as temp:
  q=Path(temp);prefix=Path('consolidation-995/natural4k-scorer-repair')
  for archive,members in read(E/'archive-files.json').items():
   with tarfile.open(E/archive) as tar:
    assert sorted(tar.getnames())==sorted(members)
    for name,pin in members.items():
     part=Path(name);assert not part.is_absolute() and '..' not in part.parts
     raw=tar.extractfile(name).read();assert len(raw)==pin['bytes'] and digest(raw)==pin['sha256'];target=q/part if archive=='raw-runtime.tar.gz' else q/prefix/part;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
  n=q/prefix;role='bounded-histogram-word'
  for name in ['qualification-policy-v3.json','qualification_records.py','qualification_timings.py']:assert (P/name).read_bytes()==(n/'qualification-source'/name).read_bytes()
  tests=read(n/'bounded-histogram-repository-tests/result.json');assert tests['exit_code']==0 and digest((n/'bounded-histogram-repository-tests/test.log').read_bytes())==tests['test_log_sha256'];assert '2 passed; 0 failed' in (n/'bounded-histogram-repository-tests/test.log').read_text()
  for name,pin in tests['source_sha256'].items():assert digest(subprocess.check_output(['git','show',c['source_integration_commit']+':'+name],cwd=P))==pin
  checked=read(n/'bounded-histogram-word-checked/result.json');assert checked['exit']==0 and checked['matches'] and checked['artifacts']==c['artifact_sha256']
  recovery=read(n/role/'promotion-recoveries/result.json');assert recovery['exact_recoveries']==12 and recovery['benchmark_worker_sha256']==c['worker_sha256']
  linked=read(n/role/'linked-proof.json');path=Path(linked['result']);part=Path(*path.parts[path.parts.index('natural4k-scorer-repair')+1:]);raw=(n/part).read_bytes();assert digest(raw)==linked['sha256'];proof=json.loads(raw);assert proof['matches_success_observations']==162 and len(proof['checked_failure_cases'])==6 and proof['peak_stack_bytes'][role]==4864
  full=read(n/role/'full/full-replay.json');plan=read(n/role/'full/plan.json');findings=Findings();evidence=Evidence(q,findings);old=[]
  for entry in plan['runs']:
   actual=t.runtime_run(full['receipts'][entry['id']],fixtures,evidence,plan['identities'],policy,entry);assert actual;old.append(actual['cases']);assert read(q/entry['id']/'pins.json')['actors']['mech-experiment']['binary_sha256']==c['worker_sha256']
  repeat=read(n/'bounded-histogram-reverse192/result.json');folder=q/Path(repeat['folder']).name;assert read(folder/'complete.json')['balanced_blocks_per_case']==192;assert read(folder/'pins.json')['actors']['mech-experiment']['binary_sha256']==c['worker_sha256'];new={}
  for row in repeat['results']:
   name=row['case'];raw=json.loads(gzip.decompress((folder/name/'observations.json.gz').read_bytes()));new[name]=t.case_blocks(raw,fixtures[name],192)[0]
  assert set(new)==set(fixtures)
  required=read(n/'word-complements-prior-wins.json')['required_previous_individual_wins'];expected=read(n/'bounded-histogram-pooled-individual-wins.json');assert expected['all_previous_individual_wins_in_both_orders']
  for label,groups in [('forward96',[old[0]]),('original_reverse96',[old[1]]),('repeat_reverse192',[new]),('pooled_reverse288',[old[1],new])]:
   for peer in ['roc','libdeflate']:
    unsupported=[]
    for index,name in enumerate(fixtures):
     if name not in required[peer]:continue
     parts=[]
     for group in groups:parts.extend(t.paired(group[name],'mech-experiment',peer))
     ci=t.interval(parts,index,policy);assert ci==expected[label][peer]['intervals'][name]
     if ci['estimate']>=0 or ci['upper']>=0:unsupported.append(name)
    assert unsupported==expected[label][peer]['unsupported']
    if label!='original_reverse96':assert not unsupported
  assert expected['original_reverse96']['roc']['unsupported']==['prefix-natural-65536','prefix-natural-65537']
  assert findings.rejected==findings.unresolved==[]
 print('Verified 63,612 B, exact source/artifacts/worker; all prior 81 Roc and 62 C wins in forward96 and reverse288 including every original block; repeat192 independently supports all wins; outputs, failures, 12 recoveries, 4864 B stack and native tests.')
if __name__=='__main__':main()
