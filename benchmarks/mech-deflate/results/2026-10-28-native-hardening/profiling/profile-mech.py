import hashlib, json, os, signal, subprocess, time
from pathlib import Path

root = Path(__file__).resolve().parent
binary = Path('/private/tmp/mech-live-control-target/release/complete-reality')
artifact = Path('/private/tmp/mech-linear-restack-20261026/complete-compressor-choice-qualified.mbc')
env = dict(os.environ, MECH_DEFLATE_USE_SAVED='1', MECH_DEFLATE_ARTIFACT=str(artifact), MECH_DEFLATE_FULL_INPUT=str(root/'input'), MECH_DEFLATE_FULL_REFERENCE=str(root/'expected.deflate'))
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert len((root/'input').read_bytes()) == 8
start = time.monotonic()
with (root/'mech-baseline.log').open('w') as log:
    process = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    (root/'mech-process.json').write_text(json.dumps({'pid':process.pid, 'binary':str(binary), 'binary_sha256':digest(binary), 'artifact':str(artifact), 'artifact_sha256':digest(artifact), 'input_bytes':8, 'profiling_only':True}, indent=2)+'\n')
    print('Mech profiling process', process.pid, flush=True)
    sampled = False
    stopped = False
    while process.poll() is None:
        content = (root/'mech-baseline.log').read_text()
        if not sampled and 'complete-compression-start' in content:
            sampled = True
            print('Sampling compression phase for eight seconds', flush=True)
            sample = subprocess.run(['/usr/bin/sample', str(process.pid), '8', '1', '-file', str(root/'mech-baseline.sample.txt')], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            (root/'mech-sample-command.log').write_text(sample.stdout)
        if time.monotonic()-start > 180:
            stopped = True
            os.killpg(process.pid, signal.SIGTERM)
            break
        time.sleep(0.05)
    result = process.wait()
receipt = {'exit':result, 'wall_seconds':time.monotonic()-start, 'sampled_compression':sampled, 'profiling_wall_stop':stopped, 'compiler_budgets':'NoLimits unchanged; wall stop belongs only to this profiling experiment', 'compression_timing_is_profiled':True}
(root/'mech-profile-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt),flush=True)
print('\n'.join((root/'mech-baseline.log').read_text().splitlines()[-4:]),flush=True)
