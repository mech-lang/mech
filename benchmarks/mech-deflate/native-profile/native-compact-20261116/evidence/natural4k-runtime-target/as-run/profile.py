"""Sample the exact qualified Mech and pinned C workers; diagnostic, not a latency run."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess,time
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent;Q=LAB.parent
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
selected=LAB/'preserved-registers/preserved/bound'
peers=read(Q/'pinned-peers/pins.json')
actors={'mech':{'command':[str(selected/'mech-worker'),str(selected/'metadata.json'),str(Q/'fixtures.json')],'sha256':'5afce59aa83c03df81c693d78e9a65f17df1c88c64a7e04c490c58ecc4fffba0'},'libdeflate':{'command':peers['actors']['libdeflate']['command'],'sha256':peers['actors']['libdeflate']['binary_sha256']}}
fixtures=read(Q/'fixtures.json');name='prefix-natural-4096';index=next(i for i,c in enumerate(fixtures) if c['name']==name)
plan={'source_commit':'b104d014da5da9b87ef270909507e9bfe35558f8','case':name,'actors':actors,'scope':'Exact published workers, repeated W commands; sample includes caller validation outside compression and is not latency. Attribute compression separately.','load_before':os.getloadavg(),'sampling_seconds':10,'sampling_interval_ms':1,'warmup_commands':80}
(ROOT/'plan.json').write_text(json.dumps(plan,indent=2)+'\n')
for label,actor in actors.items():
 assert sha(actor['command'][0])==actor['sha256']
 folder=ROOT/label;folder.mkdir()
 with (folder/'stderr.log').open('w') as error, (folder/'protocol.log').open('w') as protocol:
  child=subprocess.Popen(actor['command'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=error,text=True)
  try:
   assert child.stdout.readline().strip()=='SESSION_READY'
   child.stdin.write(f'C\t{index}\t0\n');child.stdin.flush()
   assert child.stdout.readline().strip()==f'CASE_READY\t{name}'
   child.stdin.write('W'*80+'NN');child.stdin.flush()
   time.sleep(1)
   with (folder/'sample.log').open('w') as log:
    result=subprocess.run(['/usr/bin/sample',str(child.pid),'10','1','-file',str(folder/'sample.txt')],stdout=log,stderr=log)
   rows=[]
   while True:
    line=child.stdout.readline();assert line,(label,child.poll())
    protocol.write(line)
    if line.startswith('{'):rows.append(json.loads(line))
    if line.strip()==f'CASE_DONE\t{name}':break
   assert len(rows)==2 and all(r['independent_inflation'] and r['input_alias_unchanged'] and r['borrowed_output_alias_unchanged'] for r in rows)
   child.stdin.write('Q\n');child.stdin.flush();assert child.wait(timeout=10)==0
   record={'worker_sha256':sha(actor['command'][0]),'command':actor['command'],'sample_exit_code':result.returncode,'sample_sha256':sha(folder/'sample.txt') if (folder/'sample.txt').exists() else None,'verified_calls':rows,'load_after':os.getloadavg()}
   (folder/'result.json').write_text(json.dumps(record,indent=2)+'\n');print(label,'sample',result.returncode,flush=True)
  finally:
   if child.poll() is None:child.terminate();child.wait(timeout=10)
