"""Capture current checked-root IR and source roles from the fixed worker build."""
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess

here = Path(__file__).resolve().parent
baseline = here.parent / 'function-compiler/candidate'


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def capture():
    control = json.loads((baseline / 'rebuild/rebuild.json').read_text())
    assert control['full_integrated_source_identity']
    compiler = baseline / 'compile_profile'
    assert sha(compiler) == (baseline / 'compiler.sha256').read_text().rstrip()
    source = Path(control['phases']['source_to_native']['command'][1])
    out = here / 'observed'
    ir = out / 'ir'
    out.mkdir()
    ir.mkdir()
    flags = control['native_flags'] | {'MECH_COMPACT_ROUTE_DUMP': str(ir),
                                      'MECH_COMPACT_LAYOUT_REGION_REPORT': str(ir / 'block-roles.tsv')}
    command = [str(compiler), str(source), str(out / 'compiled'), 'compact']
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    with (out / 'compile.log').open('wb') as log:
        subprocess.run(command, env=env | flags | {'PATH': '/usr/bin:/bin'}, stdout=log, stderr=log, check=True)
    exact = {name: sha(out / 'compiled' / name) for name in ('program.mecb', 'program.o', 'metadata.json')}
    assert exact == {'program.mecb': sha(baseline / 'rebuild/frontend/program.mecb'),
                     'program.o': control['object_sha256'], 'metadata.json': control['metadata_sha256']}
    record = {'compiler_sha256': sha(compiler), 'source_sha256': sha(source), 'command': command,
              'flags': flags, 'artifact_sha256': exact, 'dump_sha256': {p.name: sha(p) for p in sorted(ir.iterdir())},
              'scope': 'Structural census only. Timings collected with the dump enabled are excluded.'}
    (out / 'plan.json').write_text(json.dumps(record, indent=2) + '\n')
    print(record['artifact_sha256'], list(record['dump_sha256']), flush=True)


if __name__ == '__main__':
    capture()
