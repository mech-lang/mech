"""Verify the selected artifact, raw full orders and every prior individual peer win."""
from pathlib import Path
from hashlib import sha256
from runpy import run_path
import gzip,json,sys,tarfile,tempfile
H=Path(__file__).resolve().parent;P=H.parents[2]
read=lambda p:json.loads(p.read_text());digest=lambda b:sha256(b).hexdigest()

def main():
 candidate=read(H/'candidate.json');assert candidate['encoder_text_bytes']==63388
 current=read(H/'current-source-relink.json');assert current['selected_worker']['worker_sha256']==candidate['worker_sha256']
 import subprocess
 tests=P/'evidence/review-followup-20261119/repository-tests';tested=read(tests/'result.json')
 assert tested['exit_code']==0 and digest((tests/'test.log').read_bytes())==tested['test_log_sha256']
 assert '10 passed; 0 failed' in (tests/'test.log').read_text()
 for name,pin in tested['source_sha256'].items():assert digest(subprocess.check_output(['git','show','00765f3be:'+name],cwd=P))==pin
 for name,pin in current['source_inputs'].items():assert digest(subprocess.check_output(['git','show',current['source_commit']+':'+name],cwd=P))==pin
 assert digest((P/'program.mec').read_bytes())==candidate['source_sha256']
 text_size=run_path(str(P/'evidence/vector-mismatch-20261119/verify.py'),run_name='text_section_verifier')['text_size']
 assert text_size(gzip.decompress((H/'program.o.gz').read_bytes()))==candidate['encoder_text_bytes']
 for name in ['program.o','metadata.json']:
  assert digest(gzip.decompress((H/(name+'.gz')).read_bytes()))==candidate['artifact_sha256'][name]
 assert digest(gzip.decompress((H/'worker-mech-worker.gz').read_bytes()))==candidate['worker_sha256']
 assert digest(gzip.decompress((H/'worker-metadata.json.gz').read_bytes()))==candidate['artifact_sha256']['metadata.json']
 for profile in candidate['profiles']:assert digest((H/profile['path']).read_bytes())==profile['sha256']
 named=read(H/'named-rebuild.json');assert named['full_integrated_source_identity'] and named['object_text_bytes']==63388
 assert named['object_sha256']==candidate['artifact_sha256']['program.o'] and named['metadata_sha256']==candidate['artifact_sha256']['metadata.json'] and named['bytecode_sha256']==candidate['artifact_sha256']['program.mecb']
 replay=read(H/'source-reproduction.json');assert len(replay['variants'])==2
 assert {x['mode'] for x in replay['variants']}=={'normal','checked'}
 assert all(x['artifacts']==candidate['artifact_sha256'] for x in replay['variants'])
 sys.path.insert(0,str(P));from qualification_records import Evidence,Findings,load_fixtures,fixture_contract
 from qualification_timings import runtime_run,report_run,paired,interval
 policy=read(P/'qualification-policy-v3.json');fixtures=load_fixtures(Evidence(P,Findings()),policy)
 contracts={}
 for name,fixture in fixtures.items():contracts.setdefault(fixture_contract(fixture),set()).add(name)
 manifest=read(H/'archive-files.json');prefix=Path('consolidation-995/natural4k-scorer-repair');role='word-current-complements-profiled'
 with tempfile.TemporaryDirectory(prefix='mech-word-qualification-') as temp:
  q=Path(temp)
  for archive,entries in manifest.items():
   with tarfile.open(H/archive) as tar:
    assert sorted(tar.getnames())==sorted(entries)
    for name,pin in entries.items():
     part=Path(name);assert not part.is_absolute() and '..' not in part.parts
     raw=tar.extractfile(name).read();assert len(raw)==pin['bytes'] and digest(raw)==pin['sha256']
     target=q/part if archive=='raw-runtime.tar.gz' else q/prefix/part
     target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
  root=q/prefix
  for name in ['qualification-policy-v3.json','qualification_records.py','qualification_timings.py']:
   assert (P/name).read_bytes()==(root/'qualification-source'/name).read_bytes()
  record=read(root/role/'full/full-replay.json');assert record['decision']=='Retain' and record['rejected']==record['unresolved']==[]
  prior=read(root/'word-complements-prior-wins.json');old=P/'evidence/encoder-size-gap/immutable-current-read-study/placement/required-wins.json';assert digest(old.read_bytes())==prior['source_sha256']
  for peer in ['roc','libdeflate']:
   expected=set.union(*(set(row['peers'][peer]['candidate']) for row in read(old)))
   assert expected==set(prior['required_previous_individual_wins'][peer])
  proof=read(root/role/'linked-proof.json');linked=Path(proof['result']);relative='/'.join(linked.parts[linked.parts.index('natural4k-scorer-repair')+1:]);payload=(root/relative).read_bytes();assert digest(payload)==proof['sha256'];checked=json.loads(payload);assert checked['matches_success_observations']==162 and len(checked['checked_failure_cases'])==6;assert checked['peak_stack_bytes'][role]==proof['peak_stack_bytes']==4864
  recovered=read(root/role/'promotion-recoveries/result.json');assert recovered['exact_recoveries']==12 and recovered['benchmark_worker_sha256']==candidate['worker_sha256']
  rows=read(root/role/'promotion-recoveries/checked-recoveries.json');assert len(rows)==12
  assert all(all(row[key] for key in ['exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for row in rows)
  allocation=read(root/'word-complements-checked/result.json');assert allocation['exit']==0 and allocation['matches'] and allocation['artifacts']==candidate['artifact_sha256']
  plan=read(root/role/'full/plan.json');findings=Findings();evidence=Evidence(q,findings)
  for entry in plan['runs']:
   receipt=record['receipts'][entry['id']];actual=runtime_run(receipt,fixtures,evidence,plan['identities'],policy,entry);assert actual is not None
   assert read(q/entry['id']/'pins.json')['actors']['mech-experiment']['binary_sha256']==candidate['worker_sha256']
   observed=report_run(actual,policy,contracts);expected=next(x for x in record['runs'] if x['id']==entry['id'])
   assert observed['peers']==expected['peers']
   for peer,floor in [('roc',81),('libdeflate',62)]:
    for key in ['point_estimate_wins','precision_supported_wins']:assert observed['peers'][peer][key]['mech-experiment']==expected['peers'][peer][key]['mech-experiment']==floor
    for index,(name,ratios) in enumerate(actual['ratios'].items()):
     if name not in prior['required_previous_individual_wins'][peer]:continue
     assert ratios[peer]['mech-experiment']<1;parts=paired(actual['cases'][name],'mech-experiment',peer)
     assert max(map(max,parts))<0 or interval(parts,index,policy)['upper']<0,(entry['order'],peer,name)
  assert findings.rejected==findings.unresolved==[]
  for run in read(root/'word-complements-full-results.json')['runs']:
   folder=q/Path(run['run']).name;rows=read(folder/'summary.json');assert len(rows)==81 and digest((folder/'summary.json').read_bytes())==run['summary_sha256']
   for key,floor in [('paired_x_roc',81),('paired_x_libdeflate',62)]:assert sum(row[key]['mech-experiment']<1 for row in rows)==floor
 print('Verified 63,388 B; 81 Roc / 62 C point and precision wins in both full 96-block orders; every old individual win; outputs, recovery, failures, stack, allocation checks and archive hashes.')

if __name__=='__main__':main()
