from pathlib import Path
import json,os,subprocess,time
P=Path(__file__).parent;O=Path('/private/tmp/mech-related-positional-metadata-opt-20261107-v1')
def run(cmd,name,env=None):
    cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})})
    (P/(name+'.json')).write_text(json.dumps(dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-5000:]+r.stderr[-5000:];return r
engine=next(s for s in json.loads((O/'build.json').read_text())['stages']if s['crate']=='mech_engine');cmd=engine['command'][:];cmd[cmd.index('--crate-type=rlib')]='--test';cmd[-1]=str(P/'engine-tests')
for i,a in enumerate(cmd):
    if a.startswith('metadata='):cmd[i]=a+'_positional_tests'
run(cmd,'test-build',engine['environment'])
new=run([P/'engine-tests','positional','--test-threads=1','--nocapture'],'new-tests')
all_image=run([P/'engine-tests','build_image','program_image','--test-threads=1'],'all-image-tests')
(P/'summary.json').write_text(json.dumps(dict(new_tests_passed=True,all_image_tests_passed=True,new_summary=[x for x in new.stdout.splitlines()if x.startswith('test result:')],image_summary=[x for x in all_image.stdout.splitlines()if x.startswith('test result:')]),indent=2)+'\n')
