"""Profile the exact selected Mech and C workers on every remaining distinct losing contract."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess,time
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent;Q=LAB.parent
PACKET=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116')
OUT=ROOT/'history-review';OUT.mkdir(exist_ok=True)
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
nearest=read(PACKET/'evidence/early-codebook-emission/runtime-targets/nearest.json');fixtures=read(Q/'fixtures.json');selected=LAB/'preserved-registers/preserved/bound';peers=read(Q/'pinned-peers/pins.json')
actors={'mech':{'command':[str(selected/'mech-worker'),str(selected/'metadata.json'),str(Q/'fixtures.json')],'sha256':'5afce59aa83c03df81c693d78e9a65f17df1c88c64a7e04c490c58ecc4fffba0'},'libdeflate':{'command':peers['actors']['libdeflate']['command'],'sha256':peers['actors']['libdeflate']['binary_sha256']}}
groups={}
for row in nearest['cases']:
 index=next(i for i,f in enumerate(fixtures) if f['name']==row['case']);f=fixtures[index]
 key=tuple(sha(f[k]) for k in ['input','prefix','expected'])+(f['capacity'],)
 groups.setdefault(key,{'index':index,'case':row['case'],'aliases':[]})['aliases'].append(row['case'])
plan={'source_commit':'388e1087c401cd3b9f87a31b10652372ab720096','qualified_reference_worker':actors['mech']['sha256'],'actors':actors,'named_losing_cases':len(nearest['cases']),'distinct_contracts':list(groups.values()),'scope':'Untimed diagnostic samples from repeated complete compressions. Sampling also sees excluded preparation and validation. Do not multiply fractions by qualified medians to predict elapsed savings. No changed-worker qualification.','sample_seconds':3,'interval_ms':1,'warmup_commands':24,'load_before':os.getloadavg()}
(OUT/'plan.json').write_text(json.dumps(plan,indent=2)+'\n')
(OUT/'plan.md').write_text('''# History and suite profiling — before experiment\n\n1. Preserve the full goal: 1x libdeflate code and complete build time, holding 81 Roc/62 C and seeking more runtime wins. Rank remaining losses by absolute excess time, multiplicative gap, shared hot paths and expected suite coverage.\n2. Read the historical source patches, admission limits, actual failures, stack/counter evidence and both-order results. Reconsider candidates rejected by the old micro-slowdown rule. Match each old mechanism to current retained advances before choosing a combination.\n3. Profile every distinct remaining C-losing contract with the exact qualified Mech/C workers. Collect actual allocation/copy/zeroing counters and distinguish scorer, matcher, controller, ownership and excluded validation stacks. Aliased contracts may share a capture; retain all names.\n4. Use existing qualified medians for gap ranking. Sampling and two verification calls are diagnostics only. Source changes still require actual-worker outputs/errors/recovery, stack <=5280, both focused/full orders, every existing peer win and matched complete compile-time/text accounting.\n5. Do not pick the next change solely by the smallest ratio gap. Prefer a proven shared bottleneck and a bounded combination whose old failure mechanism is addressed by current code. Keep adverse results and publish the decision in PR995.\n''')
for group in groups.values():
 for label,actor in actors.items():
  assert sha(actor['command'][0])==actor['sha256'];folder=OUT/'profiles'/group['case']/label
  if (folder/'result.json').exists():continue
  folder.mkdir(parents=True,exist_ok=True)
  with (folder/'stderr.log').open('w') as error,(folder/'protocol.log').open('w') as protocol:
   child=subprocess.Popen(actor['command'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=error,text=True)
   try:
    assert child.stdout.readline().strip()=='SESSION_READY'
    child.stdin.write(f'C\t{group["index"]}\t0\n');child.stdin.flush();assert child.stdout.readline().strip()==f'CASE_READY\t{group["case"]}'
    child.stdin.write('W'*24+'NN');child.stdin.flush();time.sleep(.4)
    with (folder/'sample.log').open('w') as log:done=subprocess.run(['/usr/bin/sample',str(child.pid),'3','1','-file',str(folder/'sample.txt')],stdout=log,stderr=log)
    rows=[]
    while True:
     line=child.stdout.readline();assert line,(group['case'],label,child.poll());protocol.write(line)
     if line.startswith('{'):rows.append(json.loads(line))
     if line.strip()==f'CASE_DONE\t{group["case"]}':break
    assert len(rows)==2 and all(all(r[k] for k in ['independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for r in rows)
    child.stdin.write('Q\n');child.stdin.flush();assert child.wait(timeout=10)==0
    record={'case':group['case'],'aliases':group['aliases'],'actor':label,'worker_sha256':sha(actor['command'][0]),'command':actor['command'],'sample_exit_code':done.returncode,'sample_sha256':sha(folder/'sample.txt') if (folder/'sample.txt').exists() else None,'verified_calls':rows,'load_after':os.getloadavg()}
    (folder/'result.json').write_text(json.dumps(record,indent=2)+'\n');print(group['case'],label,'sample',done.returncode,flush=True)
   finally:
    if child.poll() is None:child.terminate();child.wait(timeout=10)
(OUT/'complete.json').write_text(json.dumps({'profiles':len(groups)*2,'named_cases':len(nearest['cases']),'distinct_contracts':len(groups),'load_after':os.getloadavg()},indent=2)+'\n')
