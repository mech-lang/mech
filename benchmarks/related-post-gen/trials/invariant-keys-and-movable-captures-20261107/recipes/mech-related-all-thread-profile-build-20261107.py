from pathlib import Path
import json,subprocess,hashlib,shutil,time,os
P=Path('/private/tmp/mech-related-capture-all-thread-profile-20261107-v1');P.mkdir(exist_ok=True);R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');OLD=Path('/private/tmp/mech-related-selected-and-numeric-aliases-pc-profile-20261107-v2/selected-ir242')
(P/'sampler-stub.c').write_text('int sam_init(void){return 0;}\nvoid sam_enable(int value){(void)value;}\nint sam_finish(const char*a,const char*b,const char*c){(void)a;(void)b;(void)c;return 0;}\n')
def run(c,p):
 c=list(map(str,c));t=time.monotonic();x=subprocess.run(c,capture_output=True,text=True,env={**os.environ,'RUSTC_BOOTSTRAP':'1'});p.write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');assert x.returncode==0,x.stdout+x.stderr
run(['xcrun','clang','-O2','-c',P/'sampler-stub.c','-o',P/'sampler-stub.o'],P/'stub-build.json')
for name,b in [('selected-ir242',Path('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1/reuse-4')),('candidate-ir248',Path('/private/tmp/mech-related-movable-capture-reads-runners-20261107-v3/reuse-4'))]:
 p=P/name;p.mkdir();(p/'build').symlink_to(b/'build',target_is_directory=True);shutil.copyfile(OLD/'runner.rs',p/'runner.rs')
 d=json.loads((b/'runner-aot.build.json').read_text());cmd=d['command'][:];assert cmd.count(d['source_path'])==1;cmd[cmd.index(d['source_path'])]=str(p/'runner.rs');cmd[cmd.index('-o')+1]=str(p/'runner-profile');cmd[1:1]=['-C','link-arg='+str(P/'sampler-stub.o')]
 (p/'build-command.json').write_text(json.dumps(dict(base_build=str(b/'runner-aot.build.json'),command=cmd),indent=2)+'\n');run(cmd,p/'build.json');run(['python3',R/'audit-native-link.py','--object',b/'build/object.o','--executable',p/'runner-profile','--output',p/'native-link-audit.json'],p/'audit-run.json');print(name,'driver built, original native Object audited',flush=True)
