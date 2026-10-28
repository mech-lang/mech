import subprocess,json,time,os,hashlib,statistics,datetime
from pathlib import Path
p=Path(__file__).parent
w=Path('/private/tmp/mech-deflate-reality-check')
mech=Path('/private/tmp/mech-live-control-target/release/component-reality')
assert mech.is_file(),mech
env={**os.environ,'MECH_REALITY_REPS':'5','MECH_REALITY_CASE':'all'}
rows=[];runs=[]
cases=['initialization','search','skip','six-position']
def run(engine,round,cmd):
 start=time.monotonic()
 r=subprocess.run([str(c) for c in cmd],cwd=w,env=env,text=True,capture_output=True)
 filename=f'{engine}-round-{round}.log'
 (p/filename).write_text(r.stdout+r.stderr)
 meta={'engine':engine,'round':round,'command':[str(c) for c in cmd],'returncode':r.returncode,'wall_seconds':time.monotonic()-start,'raw':filename}
 runs.append(meta);print(json.dumps(meta),flush=True)
 assert r.returncode==0,meta
 got=[]
 for line in r.stdout.splitlines():
  if engine.startswith('roc'):
   if not line.startswith('sample\t'):continue
   _,case,rep,ns=line.split('\t');d={'engine':'roc','case':cases[int(case)],'rep':int(rep),'nanoseconds':int(ns),'warmup':rep=='0'}
  else:
   i=line.find('{"');
   if i<0:continue
   try:d=json.loads(line[i:])
   except json.JSONDecodeError:continue
   if 'nanoseconds' not in d:continue
   d.setdefault('warmup',d['rep']==0)
  d['round']=round;rows.append(d);got.append(d)
 if engine=='mech':
  assert len(got)==24,len(got)
  for case in cases:
   subset=[d for d in got if d['case']==case];assert [d['rep'] for d in subset]==list(range(6)),subset
 else:
  assert [d['rep'] for d in got]==list(range(8)),got
 for d in got:assert d['nanoseconds']>0

def mech_run(round):run('mech',round,[mech])
def refs(round,order):
 for case in order:run('roc-'+str(case),round,[p/'roc-components',case])
 run('rust',round,[p/'rust-init','262144','1500513'])
started=datetime.datetime.now(datetime.timezone.utc).isoformat()
refs(0,range(4));mech_run(0)
mech_run(1);refs(1,reversed(range(4)))
summary=[]
for engine,case in sorted({(d['engine'],d['case'])for d in rows}):
 samples=[d for d in rows if d['engine']==engine and d['case']==case and not d['warmup']]
 values=[d['nanoseconds']for d in samples]
 rec={'engine':engine,'case':case,'samples':len(values),'median_ns':statistics.median(values),'min_ns':min(values),'max_ns':max(values)}
 for key in ['allocations','requested_bytes']:
  if key in samples[0]:rec['median_'+key]=statistics.median([d[key]for d in samples])
 summary.append(rec)
result={'started':started,'finished':datetime.datetime.now(datetime.timezone.utc).isoformat(),'mech_executable':str(mech),'mech_executable_sha256':hashlib.sha256(mech.read_bytes()).hexdigest(),'runs':runs,'observations':rows,'summary':summary}
(p/'measurements.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(summary,indent=2),flush=True)
