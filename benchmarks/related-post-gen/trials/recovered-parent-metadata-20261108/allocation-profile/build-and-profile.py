from pathlib import Path
import hashlib, json, os, shutil, subprocess, time
ROOT=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path(__file__).parent
B=Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1')
BUILD=Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1/runtime/build.json')
INPUT=Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json')
EXPECTED=Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(command,receipt,env=None):
    command=list(map(str,command));start=time.monotonic();r=subprocess.run(command,capture_output=True,text=True,env={**os.environ,**(env or {})})
    receipt.write_text(json.dumps(dict(command=command,exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
    assert r.returncode==0,r.stdout+r.stderr
for name in ['plain','both-rows']:
    p=O/name;p.mkdir(exist_ok=True);(p/'build').symlink_to(B/name/'build',target_is_directory=True)
    s=(B/name/'runner.rs').read_text();at=s.index('use mech_core::');s=s[:at]+f'#[path = "{O}/allocation_profile.rs"] mod allocation_profile;\n#[global_allocator]\nstatic PROFILE_ALLOCATOR: allocation_profile::Tracked = allocation_profile::Tracked;\n'+s[at:]
    assert s.count('fn run() {')==1;s=s.replace('fn run() {','fn run() {\n    allocation_profile::prewarm();\n    let profile_output = std::env::var_os("MECH_ALLOCATION_PROFILE").expect("profile path");')
    assert s.count('    let post_json_start = Instant::now();')==1;s=s.replace('    let post_json_start = Instant::now();','    allocation_profile::begin();\n    let post_json_start = Instant::now();')
    assert s.count('        let post_json_seconds = post_json_start.elapsed().as_secs_f64();')==1;s=s.replace('        let post_json_seconds = post_json_start.elapsed().as_secs_f64();','        let post_json_seconds = post_json_start.elapsed().as_secs_f64();\n        allocation_profile::stop_and_write(&profile_output);')
    (p/'runner.rs').write_text(s)
    run(['python3',ROOT/'build-host.py','--compiler-build',BUILD,'--source',p/'runner.rs','--out',p/'runner-profile','--bundle',p/'build','--linked'],p/'host-build-run.json')
    run(['python3',ROOT/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-profile','--output',p/'native-link-audit.json'],p/'audit-run.json')
    output=p/'related_posts_profile.json';run([p/'runner-profile',p/'build/source.mec',INPUT,output,'1'],p/'profile-run.json',{'MECH_ALLOCATION_PROFILE':str(p/'allocations.jsonl')})
    assert json.loads(output.read_text())==json.loads(EXPECTED.read_text())
    r=json.loads((p/'profile-run.json').read_text());r.update(exact_standard_rust_output=True,output_sha256=sha(output),expected_output_sha256=sha(EXPECTED),diagnostic_timings_excluded=True);(p/'profile-run.json').write_text(json.dumps(r,indent=2)+'\n')
    h=json.loads((p/'allocations.jsonl').read_text().splitlines()[0]);assert h['buffer_dropped']==0 and h['reentrant_dropped']==0
    print(name,'exact Rust output',h['recorded'],'allocation stacks',flush=True)
