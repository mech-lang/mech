"""Archive the current spill-pair experiment after both runtime orders and matched builds."""
from pathlib import Path
from hashlib import sha256
from shutil import copyfile
import csv,gzip,json,tarfile
HERE=Path(__file__).resolve().parent;LAB=HERE.parent;Q=LAB.parent;S=HERE/'retest-register-spills';trial=S/'paired-profiled'
PACKET=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116')
OUT=PACKET/'evidence/encoder-size-gap/current-register-spill-pairs';OUT.mkdir(exist_ok=True)
read=lambda p:json.loads(Path(p).read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,indent=2)+'\n')
def copy(a,b,z=False):
 b=OUT/b;b.parent.mkdir(parents=True,exist_ok=True)
 if z:b.write_bytes(gzip.compress(a.read_bytes(),mtime=0))
 else:copyfile(a,b)
def archive(dest,items):
 with tarfile.open(OUT/dest,'w:gz') as tar:
  for source,name in items:tar.add(source,arcname=name)
runtime=read(trial/'full/full-replay.json');gate=read(trial/'full/screen.json');build=read(S/'matched-builds/summary.json');receipt=read(trial/'receipt.json');proof=read(trial/'linked-proof.json');recoveries=read(trial/'recoveries/result.json')
assert gate['decision']=='Retain' and gate['rejected']==gate['unresolved']==[]
assert proof['peak_stack_bytes']<=5280 and recoveries['exact_recoveries']==12
for run in runtime['runs']:
 assert run['case_count']==81
 for peer,count in [('roc',81),('libdeflate',62)]:
  assert run['peers'][peer]['point_estimate_wins']['mech-experiment']>=count
  assert run['peers'][peer]['precision_supported_wins']['mech-experiment']>=count
# Preserve the old required fixture wins explicitly, not just each count.
oldloss=read(PACKET/'evidence/early-codebook-emission/runtime-targets/nearest.json')
# Old qualified forward/reverse summaries are the complete roster authority.
retained=[]
for order in ['forward','reverse']:
 old=read(Q/f'preserved-register-root-preserved-full-{order}/summary.json');new=read(Q/f'spill-pair-current-paired-profiled-full-{order}/summary.json');a={r['case']:r for r in old};b={r['case']:r for r in new};assert a.keys()==b.keys() and len(a)==81
 peers={}
 for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:
  required=[n for n,r in a.items() if r[key]['mech-experiment']<1];lost=[n for n in required if b[n][key]['mech-experiment']>=1]
  assert not lost,(order,peer,lost);peers[peer]={'required':required,'lost':lost}
 retained.append({'order':order,'peers':peers})
save(OUT/'required-wins.json',retained)
for name in ['plan.md','plan-initial.md','plan.json','plan-amendment.json','checker-match.json','profile-rebase.json','raw-graph-diagnostic.json']:
 copy(S/name,Path(name))
for p in (S/'profiles').glob('*.json'):copy(p,Path('profiles')/p.name)
for role in ['unprofiled-control','unprofiled-paired','unprofiled-checked','paired-profiled']:
 folder=S/role;copy(folder/'receipt.json',Path(role)/'receipt-as-run.json');copy(folder/'compile.log',Path(role)/'compile.log.gz',True)
 for n in ['program.o','metadata.json','program.mecb']:copy(folder/'frontend'/n,Path(role)/(n+'.gz'),True)
copy(S/'paired-profiled-checked/checker-match.json',Path('paired-profiled-checked/checker-match.json'))
copy(S/'paired-profiled-checked/compile.log',Path('paired-profiled-checked/compile.log.gz'),True)
copy(trial/'bound/runtime.json',Path('paired-profiled/runtime.json'))
copy(trial/'bound/mech-worker',Path('paired-profiled/mech-worker.gz'),True)
copy(trial/'linked-proof.json',Path('linked-proof.json'));copy(Path(proof['result']),Path('linked-checks.json'))
archive('linked-checks.tar.gz',[(S/'linked-current-paired-profiled','linked-current-paired-profiled'),(trial/'recoveries','recoveries')])
copy(S/'matcher-counts/counts.json',Path('matcher-counts/counts.json'))
copy(HERE/'history-review/matcher-counts/counts.json',Path('matcher-counts/control-counts.json'))
copy(S/'matcher-counts/plan.json',Path('matcher-counts/plan.json.gz'),True)
archive('matcher-counts/raw.tar.gz',[(p,p.name) for p in sorted((S/'matcher-counts').iterdir()) if p.name not in ['counts.json','plan.json']])
for kind in ['focus','full']:
 for name in ['plan.json','screen.json','environment.json','contract.json','fixture-content.json']:copy(trial/kind/name,Path(kind)/name)
 copy(trial/kind/(kind+'-replay.json'),Path(kind)/(kind+'-replay.json.gz'),True)
 for name in ['acquire.log','replay.log']:copy(trial/kind/name,Path(kind)/(name+'.gz'),True)
 plan=read(trial/kind/'plan.json')
 for run in plan['runs']:archive(kind+'/'+run['order']+'-observations.tar.gz',[(Q/run['id'],run['id'])])
copy(S/'matched-builds/plan.json',Path('build-plan.json'));copy(S/'matched-builds/summary.json',Path('deployment.json'));copy(S/'matched-builds/comparison.json',Path('build-comparison.json.gz'),True)
archive('build-logs.tar.gz',[(p,str(p.relative_to(S))) for p in (S/'matched-builds').rglob('*') if p.is_file() and (p.suffix in ['.log','.json','.txt'] or p.name.endswith('stderr')) and p.name not in ['comparison.json','summary.json','plan.json']])
for n in ['retest-spill-pairs.py','rebase-spill-pair-profiles-v1.py','rebase-spill-pair-profiles.py','retest-register-spills-qualify.py','retest-register-spills-recoveries.py','count-paired-matcher.py','measure-register-spills.py','publish-register-spills.py']:
 copy(HERE/n,Path('as-run')/n)
copy(HERE/'rebase-spill-pair-profiles.log',Path('as-run/failed-strict-rebase.log'))
base=read(PACKET/'evidence/encoder-size-gap/sparse-guard-rounds/candidate.json');candidate=base|{'review_parent':'388e1087c401cd3b9f87a31b10652372ab720096','source_commit':'388e1087c401cd3b9f87a31b10652372ab720096','compiler_sha256':receipt['compiler_sha256'],'encoder_text_bytes':receipt['text_bytes'],'artifact_sha256':{n:receipt[k] for n,k in [('program.o','object_sha256'),('metadata.json','metadata_sha256'),('program.mecb','bytecode_sha256')]},'worker_sha256':read(trial/'bound/runtime.json')['worker_sha256'],'profiles':[],'override_flags':base['override_flags']|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':'1'},'native_flags_as_run':receipt['flags'],'qualification_source_commit':'388e1087c401cd3b9f87a31b10652372ab720096','qualification_compiler_sha256':receipt['compiler_sha256'],'qualification_basis':'Fresh actual-worker both complete orders, allocation checker, outputs/errors/recoveries, stack and matched complete builds. See result.json.','status':'Retained source-built current spill pairs; old small-slowdown-only veto is not applied.'}
for value in receipt['flags']['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';'):
 symbol,path=value.split('@',1);candidate['profiles'].append({'symbol':symbol,'path':'profiles/'+Path(path).name,'sha256':sha(path)})
save(OUT/'candidate.json',candidate)
control=read(HERE/'history-review/matcher-counts/counts.json');paired=read(S/'matcher-counts/counts.json');counter=[]
for name,a in control['cases'].items():
 b=paired['cases'][name];counter.append({'case':name,'calls':a['calls'],'control':a['counts'],'paired':b['counts'],'instructions_removed':a['counts']['instructions']-b['counts']['instructions'],'stack_load_instructions_removed':a['counts']['stack_loads']-b['counts']['stack_loads']})
rows=[]
for order in ['forward','reverse']:
 for row in read(Q/f'spill-pair-current-paired-profiled-full-{order}/summary.json'):
  rows.append({'order':order,'case':row['case'],'candidate_control':row['paired_x_roc']['mech-experiment']/row['paired_x_roc']['mech-control'],'candidate_roc':row['paired_x_roc']['mech-experiment'],'candidate_c':row['paired_x_libdeflate']['mech-experiment'],'control_c':row['paired_x_libdeflate']['mech-control'],'medians_seconds':row['median_seconds']})
save(OUT/'runtime-case-ratios.json',rows)
result={'source_commit':candidate['source_commit'],'compiler_sha256':receipt['compiler_sha256'],'object_sha256':receipt['object_sha256'],'metadata_sha256':receipt['metadata_sha256'],'bytecode_sha256':receipt['bytecode_sha256'],'worker_sha256':candidate['worker_sha256'],'decision':gate['decision'],'encoder_bytes':receipt['text_bytes'],'encoder_bytes_saved':63804-receipt['text_bytes'],'encoder_x_libdeflate':receipt['text_bytes']/37080,'encoder_x_roc':receipt['text_bytes']/57696,'root_bytes':read(trial/'frontend/metadata.json')['code_bytes'],'root_stack_bytes':read(trial/'frontend/metadata.json')['stack_bytes'],'peak_stack_bytes':proof['peak_stack_bytes'],'source_success_observations':162,'source_failure_cases':recoveries['cases'],'source_exact_recoveries':recoveries['exact_recoveries'],'wins':[{k:r[k] for k in ['case_count','order','peers']} for r in runtime['runs']],'counter_differences':counter,'complete_builds':build,'runtime_qualification_policy':3,'recipe':'m1-register-spill-pairs','limitation':'No new C win. Keep working on remaining collector checks and the scorer canary; instruction-count changes are untimed diagnostics.'}
save(OUT/'result.json',result)
print('Archived fresh spill-pair qualification and matched complete builds:',receipt['text_bytes'],candidate['worker_sha256'])
