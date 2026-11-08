from pathlib import Path
import hashlib, importlib.util, json, os, shutil, subprocess, time

P = Path(__file__).parent
S = Path('/private/tmp/mech-related-specialized-rows-profile-compiler-20261108')
R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O = Path('/private/tmp/mech-related-latest-deflate-opt-20261108-v2')
N = Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1')
RT = P / 'runtime'; RT.mkdir(exist_ok=True)
def read(p): return json.loads(Path(p).read_text())
def write(p, d): Path(p).write_text(json.dumps(d, indent=2) + '\n')
def run(cmd, path, env=None):
    cmd = list(map(str, cmd)); start = time.monotonic()
    result = subprocess.run(cmd, capture_output=True, text=True, env={**os.environ, **(env or {})})
    write(path, dict(command=cmd, environment=env or {}, exit=result.returncode,
                     seconds=time.monotonic()-start, stdout=result.stdout, stderr=result.stderr,
                     diagnostic_only=True, promotion_timing_eligible=False))
    print(path.name, result.returncode, flush=True)
    assert result.returncode == 0, result.stdout[-5000:] + result.stderr[-5000:]
    return result

build = read(O/'build.json')
prior = read(N/'runtime/runtime-build.json')
old_source = '/private/tmp/mech-related-latest-deflate-compiler-20261108'
cmd = [a.replace(old_source, str(S)) for a in prior['command']]
cmd[-1] = str(RT/'libmech_gpu.rlib')
cmd[1:1] = ['--cfg', 'mech_native_entry_intervals']
for i, arg in enumerate(cmd):
    if arg.startswith('metadata='): cmd[i] = 'metadata=related_rows_current_thread_diagnostic_v1'
env = {k:v.replace(old_source, str(S)) for k,v in prior['environment'].items()}
run(cmd, RT/'runtime-build.json', env)
assert not any('cranelift_' in a or 'libloading=' in a for a in cmd)
spec = importlib.util.spec_from_file_location('helper', R/'build-compiler.py')
helper = importlib.util.module_from_spec(spec); spec.loader.exec_module(helper)
files = helper.source_inventory(S)
source_hash = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
write(RT/'source-inventory.json', dict(root=str(S), sha256=source_hash, files=files))
gpu = next(x for x in build['stages'] if x['crate']=='mech_gpu')
gpu.update(command=cmd, environment=env, exit=0, reused=False,
           library_sha256=hashlib.sha256((RT/'libmech_gpu.rlib').read_bytes()).hexdigest())
build.update(source=str(S), source_sha256=source_hash, runtime_only_GPU=True,
             diagnostic_only=True, ordinary_Object_compiler='ccfadbdf0af69f741237f5d1099a3c3eef79386a')
write(RT/'build.json', build)
for name in ['plain', 'both-rows']:
    root=P/name; root.mkdir(exist_ok=True)
    shutil.copyfile('/private/tmp/mech-related-native-thread-counters-20261107/candidate/runner.rs', root/'runner.rs')
    (root/'build').symlink_to(N/name/'build', target_is_directory=True)
    run(['python3',R/'build-host.py','--compiler-build',RT/'build.json','--source',root/'runner.rs',
         '--out',root/'runner-profile','--bundle',root/'build','--linked'],root/'runner-build.json')
    run(['python3',R/'audit-native-link.py','--object',root/'build/object.o','--executable',root/'runner-profile',
         '--output',root/'native-link-proof.json'],root/'link-audit.json')
    audit=read(root/'native-link-proof.json'); original=read(N/name/'native-link-proof.json')
    assert audit['status']=='passed' and audit['normalized_text_sha256']==original['normalized_text_sha256']
    run([root/'runner-profile',root/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',
         root/'pilot-output.json','1'],root/'pilot-run.json',{'MECH_INTERVAL_COUNTERS':'1','MECH_INTERVAL_HELPERS':'1'})
    assert read(root/'pilot-output.json')==read('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json')
    trace=read(root/'pilot-output.json.intervals.json')['capture']
    assert trace['dropped']==trace['unmatched_ends']==trace['counter_errors']==0
    assert all(e['closed'] and e['start_counters'] and e['end_counters'] for e in trace['events'])
    print(name, 'same native Object; exact output and balanced counter intervals', flush=True)
