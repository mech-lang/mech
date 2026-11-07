from pathlib import Path
import subprocess,json,time,shutil,hashlib
ROOT=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');OPT=Path('/private/tmp/mech-related-child-entry-facts-integration-opt-20261107-v1');OLD=Path('/private/tmp/mech-related-static-native-module-runners-20261105-v1');OUT=Path(__file__).parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(cmd,path):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True);path.write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr
run(['python3',ROOT/'build-host.py','--compiler-build',OPT/'build.json','--source',OPT/'compiler-decoded.rs','--out',OPT/'compiler-decoded'],OPT/'compiler-decoded-build-run.json')
proofs={}
for name in ['after-4','after-8','compact-4','compact-8']:
 dest=OUT/(name+'-decoded-bundle');run([OPT/'compiler-decoded',OUT/name/'build/source.mec',dest],OUT/(name+'-decoded-compile.json'))
 pins={f.name:sha(f) for f in (OUT/name/'build').iterdir() if f.is_file()};assert len(pins)==13 and all(sha(dest/k)==v for k,v in pins.items());proofs[name]=dict(all13_bundle_files_byte_identical=True,files=pins)
 print(name,'original and decoded all13 bundle files exact',flush=True)
(OUT/'original-decoded-correspondence.json').write_text(json.dumps(proofs,indent=2)+'\n')
for name in [f'{family}-{suffix}' for family in ['after','compact'] for suffix in ['4-independent','8-independent','4-json','8-json','4-full-catalog']]:
 p=OUT/name;p.mkdir();base=name.split('-')[0]+('-8' if name.split('-')[1]=='8' else '-4');(p/'build').symlink_to(OUT/base/'build',target_is_directory=True);s=(OUT/base/'runner.rs').read_text();oldname=name.replace('after-4','direct-four').replace('after-8','direct-eight').replace('compact-4','direct-four').replace('compact-8','direct-eight');old=(OLD/oldname/'runner.rs').read_text();start=s.index('    let catalog =');end=s.index('    let compile_seconds =',start);a=old.index('    let catalog =');z=old.index('    let compile_seconds =',a);s=s[:start]+old[a:z]+s[end:];(p/'runner.rs').write_text(s)
 run(['python3',ROOT/'build-host.py','--compiler-build',OPT/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
 run(['python3',ROOT/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
 run(['python3',ROOT/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',OUT/('edges-'+name)],OUT/(name+'-edges-run.json'))
 print(name,'built, native link audited,31 exact edges',flush=True)
