from pathlib import Path
import json,hashlib,subprocess,time
U=Path(__file__).parent;P=U/'rust-insert-reuse-4';R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path('/private/tmp/mech-related-canonical-schema-sinks-opt-20261107-v1')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(c,p):
 c=list(map(str,c));t=time.monotonic();r=subprocess.run(c,capture_output=True,text=True);p.write_text(json.dumps(dict(command=c,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr;print(p.name,'passed',flush=True)
decoded=U/'rust-insert-reuse-4-decoded-bundle';run([O/'compiler-decoded',P/'source.mec',decoded],P/'decoded-compile.json');pins={p.name:sha(p)for p in (P/'build').iterdir()if p.is_file()};assert len(pins)==13 and all(sha(decoded/name)==digest for name,digest in pins.items());(P/'decoded-correspondence.json').write_text(json.dumps(dict(all13_bundle_files_byte_identical=True,files=pins),indent=2)+'\n')
for loader in ['independent','json','full-catalog']:
 p=U/('rust-insert-reuse-4-'+loader);p.mkdir();(p/'build').symlink_to(P/'build',target_is_directory=True)
 source=(P/'runner.rs').read_text();old=(U/('after-4-'+loader)/'runner.rs').read_text();a=source.index('    let catalog =');b=source.index('    let compile_seconds =',a);c=old.index('    let catalog =');d=old.index('    let compile_seconds =',c);(p/'runner.rs').write_text(source[:a]+old[c:d]+source[b:])
 run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
 run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
 run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',U/('edges-rust-insert-reuse-4-'+loader)],p/'edge-run.json')
print('Rust-style insertion source passes four loaders and13 identical decoded files',flush=True)
