from pathlib import Path
import json, os, subprocess, time

for epoch, label, version in [(249, 'integration', 'v1'), (250, 'hoisting', 'v2')]:
    source = Path('/private/tmp/mech-related-invariant-capture-'+label+'-compiler-20261107')
    out = Path('/private/tmp/mech-related-invariant-capture-'+label+'-tests-20261107-'+version)/'upstream-interval-integration'
    out.mkdir()
    opt = Path('/private/tmp/mech-related-invariant-capture-'+label+'-opt-20261107-'+version)
    build = json.loads((opt/'build.json').read_text())
    host = json.loads((opt/'compiler.build.json').read_text())
    command = host['command'][:]
    command[command.index(host['source_path'])] = str(source/'hosts/gpu/tests/canonical_artifact_program.rs')
    command[command.index('-o')+1] = str(out/'tests')
    command += ['--test', '--cfg', 'feature="canonical-native-program"', '--crate-name', f'canonical_interval_{epoch}']
    externs = {}
    gpu = next(x for x in build['stages'] if x['crate']=='mech_gpu')['command']
    for i, arg in enumerate(gpu[:-1]):
        if arg == '--extern':
            name, path = gpu[i+1].split('=',1)
            externs[name] = path
    for stage in build['stages']:
        externs[stage['crate']] = stage['command'][-1]
    existing = {command[i+1].split('=',1)[0] for i,arg in enumerate(command[:-1]) if arg=='--extern'}
    for name,path in externs.items():
        if name not in existing:
            command += ['--extern', name+'='+path]
    def run(cmd, name):
        start=time.monotonic()
        x=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,'RUSTC_BOOTSTRAP':'1'})
        receipt=dict(command=cmd,exit=x.returncode,seconds=time.monotonic()-start,stdout=x.stdout,stderr=x.stderr,compiler=build.get('source_commit'))
        (out/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n')
        assert x.returncode==0, x.stderr
        return x.stdout
    run(command,'build')
    listed=run([str(out/'tests'),'unsigned_interval_assertion_keeps_boundaries_error_order_and_recovery','--list'],'list')
    assert listed.count(': test')==1,listed
    text=run([str(out/'tests'),'unsigned_interval_assertion_keeps_boundaries_error_order_and_recovery','--test-threads=1'],'run')
    assert '1 passed; 0 failed' in text,text
    print(epoch,'actual new upstream lexical integration fixture passed',flush=True)
