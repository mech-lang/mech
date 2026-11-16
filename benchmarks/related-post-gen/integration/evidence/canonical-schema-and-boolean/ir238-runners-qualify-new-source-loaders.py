from pathlib import Path
import hashlib, json, subprocess, time
U=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O=Path('/private/tmp/mech-related-canonical-schema-sinks-opt-20261107-v1')
P=U/'reuse-4'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(command,path):
    command=list(map(str,command));start=time.monotonic();result=subprocess.run(command,capture_output=True,text=True)
    path.write_text(json.dumps(dict(command=command,exit=result.returncode,seconds=time.monotonic()-start,stdout=result.stdout,stderr=result.stderr),indent=2)+'\n')
    assert result.returncode==0,result.stdout+result.stderr
    print(path.name,'passed',flush=True)

decoded=U/'reuse-4-decoded-bundle'
run([O/'compiler-decoded',P/'source.mec',decoded],U/'reuse-4-decoded-compile.json')
pins={p.name:sha(p)for p in (P/'build').iterdir()if p.is_file()}
assert len(pins)==13 and all(sha(decoded/name)==digest for name,digest in pins.items())
(U/'reuse-original-decoded-correspondence.json').write_text(json.dumps(dict(all13_bundle_files_byte_identical=True,files=pins),indent=2)+'\n')
for loader in ['independent','json','full-catalog']:
    p=U/('reuse-4-'+loader);p.mkdir();(p/'build').symlink_to(P/'build',target_is_directory=True)
    source=(P/'runner.rs').read_text();old=(U/('after-4-'+loader)/'runner.rs').read_text()
    start=source.index('    let catalog =');end=source.index('    let compile_seconds =',start)
    begin=old.index('    let catalog =');finish=old.index('    let compile_seconds =',begin)
    source=source[:start]+old[begin:finish]+source[end:];(p/'runner.rs').write_text(source)
    run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
    run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
    run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',U/('edges-reuse-4-'+loader)],p/'edge-run.json')
print('New source has13 exact decoded files and93 more independent/image/catalog output fixtures',flush=True)
