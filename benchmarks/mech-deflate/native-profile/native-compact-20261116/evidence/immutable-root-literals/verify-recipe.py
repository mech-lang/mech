"""Verify the published recipe with the exact compiler used in the comparison."""

from hashlib import sha256
import os
from pathlib import Path
import subprocess
import sys

from build import BINARY, HERE, MODE, OUTPUT, TARGET, read, save, sha
from publish import RECIPE, checkout, source_files

build = read(OUTPUT / 'result.json')
assert build['exact']
files = source_files(build)
runtime = TARGET / 'release/compile_profile'
assert sha(runtime) == sha(BINARY) == build['compiler_sha256']
stamp = TARGET / 'release/.fingerprint/mech-gpu-687d78bc75175c08/invoked.timestamp'
original = {name: ((checkout / name).stat(), (checkout / name).read_bytes()) for name in files}
out = HERE / 'promotion'
out.mkdir()
rebuild = out / 'rebuilt'
command = [sys.executable, '-B', checkout / RECIPE, rebuild,
           '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
           '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
import_time = stamp.stat().st_mtime_ns - 1_000_000_000
try:
    for name, data in files.items():
        path = checkout / name
        path.write_bytes(data)
        os.utime(path, ns=(original[name][0].st_atime_ns, import_time))
    subprocess.run(['rustfmt', '+nightly-2026-03-03', '--edition', '2024', '--check',
                    *[checkout / name for name in files if name.suffix == '.rs']], cwd=checkout, check=True)
    with (out / 'rebuild.log').open('w') as log:
        subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
finally:
    for name, (timestamp, data) in original.items():
        path = checkout / name
        assert path.read_bytes() in (data, files[name])
        path.write_bytes(data)
        os.utime(path, ns=(timestamp.st_atime_ns, timestamp.st_mtime_ns))

receipt = read(rebuild / 'rebuild.json')
assert receipt['full_integrated_source_identity'] and receipt['native_flags'][MODE] == '1'
source_log = (rebuild / 'direct-source.log').read_text()
assert 'direct scalar literals=1389' in source_log and 'direct literal attempt withdrew' not in source_log
assert sha(runtime) == build['compiler_sha256']
artifacts = {'bytecode': sha(rebuild / 'frontend/program.mecb'), 'encoder': sha(rebuild / 'native/program.o'),
             'metadata': sha(rebuild / 'native/metadata.json')}
assert artifacts == build['disabled']
proof = {'command': list(map(str, command)), 'recipe_sha256': sha256(files[RECIPE]).hexdigest(),
         'files': {str(path): sha256(data).hexdigest() for path, data in files.items() if path != RECIPE},
         'compiler_sha256': sha(runtime), 'artifacts': artifacts, 'flags': receipt['native_flags'],
         'object_text_bytes': receipt['object_text_bytes'], 'rebuild_sha256': sha(rebuild / 'rebuild.json'),
         'source_log_sha256': sha(rebuild / 'direct-source.log')}
save(out / 'recipe.json', proof)
print('frame-copy rebuild exact', receipt['object_text_bytes'], proof['recipe_sha256'])
