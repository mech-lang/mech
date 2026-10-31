from pathlib import Path
import subprocess,time,json,hashlib,statistics,gzip
r=Path('/private/tmp/mech-compression-round-20261028')
active=subprocess.check_output(['ps','-Ao','comm='],text=True).splitlines()
assert not [s for s in active if Path(s.strip()).name in ['rustc','cargo','complete-reality','compression-application']]
roc=Path('/private/tmp/mech-deflate-profiling-20261028/roc-profile')
assert hashlib.sha256(roc.read_bytes()).hexdigest()=='3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324'
cmd=[str(roc),'12'];start=time.time();p=subprocess.run(cmd,capture_output=True);(r/'roc.stdout').write_bytes(p.stdout);(r/'roc.stderr').write_bytes(p.stderr);p.check_returncode()
rows=[line.split('\t') for line in p.stdout.decode().splitlines() if line.startswith('rep\troc-deflate\t')]
assert len(rows)==20000
ref=statistics.median(int(row[3]) for row in rows)/1e9
(r/'roc.json').write_text(json.dumps({'command':cmd,'exit':p.returncode,'whole_process_seconds':time.time()-start,'compression_median_seconds':ref,'samples':len(rows),'binary_sha256':hashlib.sha256(roc.read_bytes()).hexdigest()},indent=2)+'\n')
print('REFERENCE_SECONDS',ref,flush=True)
cmd=[str(r/'current'),'/private/tmp/mech-linear-restack-20261026/complete-compressor-borrowed-caller.mbc','3']
with (r/'current.stdout').open('w') as out,(r/'current.stderr').open('w') as err:
 p=subprocess.Popen(cmd,stdout=out,stderr=err)
 sampled=False;started=time.time()
 while p.poll() is None:
  content=(r/'current.stderr').read_text()
  if 'MEASURING 4' in content and not sampled:
   sampled=True
   scmd=['sample',str(p.pid),'2','-file',str(r/'current.sample.txt')]
   s=subprocess.run(scmd,capture_output=True,text=True)
   (r/'sample-command.json').write_text(json.dumps({'command':scmd,'exit':s.returncode,'output':s.stdout+s.stderr},indent=2)+'\n')
  if time.time()-started>240:
   p.terminate();raise RuntimeError('small-case diagnostic exceeded 240-second process guard')
  time.sleep(0.05)
 code=p.wait()
assert code==0,(code,(r/'current.stderr').read_text())
obs=[json.loads(line) for line in (r/'current.stdout').read_text().splitlines()]
selected=[x for x in obs if not x['warmup'] and not x['profile']]
assert len(selected)==3
median=statistics.median(x['seconds'] for x in selected)
result={'baseline':'Roc = 1x','case':'eight-literals','level':12,'input_bytes':8,'roc_seconds':ref,'mech_seconds':median,'mech_multiple_of_roc':median/ref,'mech_observations':selected,'warmups':1,'profile_observations_excluded':1,'profile_started_after_primary':sampled,'compiler_head':'c24984b40','source_pin':json.loads((r/'source-pin.json').read_text()),'timing_boundary':'compression call only; output capacity13/clear/prefix/input setup/JIT/validation/output disposal excluded','allocator':'same pinned mimalloc2.1.7, no Rust allocation instrumentation','qualification':'small-scale diagnostic, not full corpus or a statistically paired full campaign','runtime_policy':'existing diagnostic NoLimits unchanged','exit':code,'cleanup':(r/'current.stderr').read_text().endswith('CLEANUP bytes=0 nodes=0\n')}
(r/'results.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2),flush=True)
