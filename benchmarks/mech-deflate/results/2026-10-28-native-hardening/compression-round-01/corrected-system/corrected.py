import subprocess,os,time,json,statistics,hashlib
from pathlib import Path
r=Path('/private/tmp/mech-compression-round-20261028/corrected-system');r.mkdir()
exe=Path('/private/tmp/mech-live-control-target/release/complete-reality')
assert hashlib.sha256(exe.read_bytes()).hexdigest()=='6c23c247bb0c661e3627a7ca833ca6f724139b1b4e3d13d1008239f0b1393346'
active=subprocess.check_output(['ps','-Ao','comm='],text=True).splitlines()
assert not [s for s in active if Path(s.strip()).name in ['cargo','rustc','compression-application']]
commands=[]
def run(label,cmd,env):
 start=time.time()
 with (r/(label+'.stdout')).open('w') as out,(r/(label+'.stderr')).open('w') as err:
  p=subprocess.Popen(cmd,env=env,stdout=out,stderr=err)
  while p.poll() is None:
   if time.time()-start>480:p.terminate();raise RuntimeError(label+' exceeded small-case process guard')
   time.sleep(0.1)
 commands.append({'label':label,'command':cmd,'exit':p.returncode,'process_seconds':time.time()-start})
 (r/'commands.json').write_text(json.dumps(commands,indent=2)+'\n')
 p.wait();assert p.returncode==0,(label,(r/(label+'.stderr')).read_text())
 print(label+' complete',flush=True)
run('retained',[str(exe)],dict(os.environ,MECH_DEFLATE_CALLER_WITNESS='1',MECH_DEFLATE_CALLER_CASE='eight-literals'))
# The retained binary regenerates its embedded source/bytecode. Check that the
# current compiler receives that identical complete-compressor artifact.
pin=json.loads(Path('/private/tmp/mech-compression-round-20261028/source-pin.json').read_text())
artifact=Path('/private/tmp/mech-linear-restack-20261026/complete-compressor-borrowed-caller.mbc')
assert hashlib.sha256(artifact.read_bytes()).hexdigest()==pin['artifact_sha256']
run('current',['/private/tmp/mech-compression-round-20261028/current-system',str(artifact),'3'],dict(os.environ))
old=[json.loads(l) for l in (r/'retained.stdout').read_text().splitlines() if l.startswith('{')]
new=[json.loads(l) for l in (r/'current.stdout').read_text().splitlines() if l.startswith('{')]
old=[x for x in old if x.get('witness')=='borrowed-caller-complete-compression' and x['artifact']=='decoded' and x['warmed']]
new=[x for x in new if not x['warmup']]
assert len(old)==len(new)==3
before=statistics.median(x['seconds'] for x in old);after=statistics.median(x['seconds'] for x in new)
ref=json.loads(Path('/private/tmp/mech-compression-round-20261028/roc.json').read_text())['compression_median_seconds']
result={'case':'eight-literals','input_bytes':8,'measuring_setup':'System with identical allocation probe on both Mech revisions; source/input/JIT/clear/reserve/rebinding/checks excluded','baseline_binary_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'baseline_compiler_head':'c1d20645d7b58f6b21847f51bfbf37225b6e6679','current_compiler_head':'c24984b402a1228cccf934a478965143b539f611','before_seconds':before,'after_seconds':after,'after_time_divided_by_before':after/before,'speedup':before/after,'roc_seconds':ref,'before_multiple_of_roc':before/ref,'after_multiple_of_roc':after/ref,'roc_comparison_limitation':'Roc uses its pinned mimalloc without Rust instrumentation; use these rows to isolate the Mech change, not as the matched-allocator goal baseline','before_observations':old,'after_observations':new,'qualification':'small diagnostic cohort; earlier compiler baseline includes all intervening hardening, not the frame patch alone'}
(r/'results.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2),flush=True)
