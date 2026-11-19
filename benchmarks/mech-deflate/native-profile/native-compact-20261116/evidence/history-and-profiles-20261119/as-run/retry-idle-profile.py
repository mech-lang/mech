"""Retain the idle capture and reacquire its encoder samples with a longer command stream."""
from pathlib import Path
from hashlib import sha256
import json,subprocess,time
ROOT=Path(__file__).resolve().parent;OUT=ROOT/'history-review';plan=json.loads((OUT/'plan.json').read_text());group=next(g for g in plan['distinct_contracts'] if g['case']=='prefix-random-8192');actor=plan['actors']['libdeflate'];folder=OUT/'profiles'/group['case']/'libdeflate'
old=OUT/'idle-attempt'/group['case']/'libdeflate';old.parent.mkdir(parents=True,exist_ok=True);folder.rename(old);folder.mkdir()
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();assert sha(actor['command'][0])==actor['sha256']
with (folder/'stderr.log').open('w') as error,(folder/'protocol.log').open('w') as protocol:
 child=subprocess.Popen(actor['command'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=error,text=True)
 try:
  assert child.stdout.readline().strip()=='SESSION_READY';child.stdin.write(f'C\t{group["index"]}\t0\n');child.stdin.flush();assert child.stdout.readline().strip()==f'CASE_READY\t{group["case"]}'
  child.stdin.write('W'*80+'NN');child.stdin.flush();time.sleep(.4)
  with (folder/'sample.log').open('w') as log:done=subprocess.run(['/usr/bin/sample',str(child.pid),'3','1','-file',str(folder/'sample.txt')],stdout=log,stderr=log)
  rows=[]
  while True:
   line=child.stdout.readline();assert line;protocol.write(line)
   if line.startswith('{'):rows.append(json.loads(line))
   if line.strip()==f'CASE_DONE\t{group["case"]}':break
  assert len(rows)==2 and all(all(r[k] for k in ['independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for r in rows)
  child.stdin.write('Q\n');child.stdin.flush();assert child.wait(timeout=10)==0
  r={'case':group['case'],'aliases':group['aliases'],'actor':'libdeflate','worker_sha256':sha(actor['command'][0]),'command':actor['command'],'sample_exit_code':done.returncode,'sample_sha256':sha(folder/'sample.txt'),'verified_calls':rows,'warmup_commands':80,'retry_reason':'The initial short stream had finished before sampling; its capture contained only waiting. Terminal process was verified. Keep that attempt and extend the command stream for a valid encoder capture.'}
  (folder/'result.json').write_text(json.dumps(r,indent=2)+'\n');print('Reacquired previously idle C capture',done.returncode,flush=True)
 finally:
  if child.poll() is None:child.terminate();child.wait(timeout=10)
