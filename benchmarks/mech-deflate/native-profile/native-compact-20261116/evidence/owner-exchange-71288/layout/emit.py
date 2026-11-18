"""Emit and verify a source-built owner-exchange worker with the reference stub residue."""
from argparse import ArgumentParser
from hashlib import file_digest
from importlib.util import module_from_spec, spec_from_file_location
import json
import os
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification')
WORKERS = Q / 'backend-work-hardened/worker-checks'
PREBUILT = Q / 'match-cache-offset-slot/compile_profile'
PIN = '9742feccb44ca2b1c052abaa286df802ae053c1e756c44dd4d93b381654478fb'
SIZE_CONTROL = WORKERS / 'size-control'
PRIOR = WORKERS / 'owner-exchange'


def sha(path):
    with Path(path).open('rb') as handle:
        return file_digest(handle, 'sha256').hexdigest()


def load(path):
    return json.loads(path.read_text())


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def run(command, log, flags=None):
    env = {key: val for key, val in os.environ.items() if not key.startswith('MECH_')}
    cmd = list(map(str, command))
    with log.open('wb') as stream:
        subprocess.run(cmd, cwd=Q, env=env | (flags or {}), stdout=stream, stderr=subprocess.STDOUT, check=True)
    return cmd


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument('label')
    args = cli.parse_args()
    assert '/' not in args.label and args.label not in ('', '.', '..')
    out = HERE / args.label
    out.mkdir()
    spec = spec_from_file_location('macho', Q / 'reachability-census.py')
    macho = module_from_spec(spec)
    spec.loader.exec_module(macho)

    def text(path, name=macho.TEXT):
        return macho.load(path)[1][name]

    control_link = SIZE_CONTROL / 'mech-worker'
    candidate_link = PRIOR / 'mech-worker'
    control_stubs = text(control_link, (b'__TEXT', b'__stubs'))[1]
    candidate_stubs = text(candidate_link, (b'__TEXT', b'__stubs'))[1]
    align = 128
    extra = (control_stubs - candidate_stubs) % align
    assert extra == 80
    code = load(PRIOR / 'metadata.json')['code_bytes']
    assert code == 50708
    tail = out / 'checked-root-128-residue.json'
    write(tail, {'code_bytes': code, 'reference_code_bytes': code + extra, 'alignment': align, 'nop_fill': True})

    runtime = load(PRIOR / 'runtime.json')
    flags = runtime['flags'].copy()
    assert 'MECH_COMPACT_ROOT_TAIL_PROFILE' not in flags
    assert flags['MECH_COMPACT_FUNCTION_ALIGNMENT_LOG2'] == '4'
    assert flags['MECH_COMPACT_BRANCH_ON_ZERO'] == '1'
    flags['MECH_COMPACT_ROOT_TAIL_PROFILE'] = str(tail)
    source = PRIOR / 'source.mec'
    pinned = load(PRIOR / 'result.json')['sha256']
    control = load(SIZE_CONTROL / 'result.json')
    assert sha(PREBUILT) == PIN
    assert sha(source) == sha(SIZE_CONTROL / 'source.mec')
    emit = run([PREBUILT, source, out, 'compact'], out / 'emit.log', flags)
    for name in ('program.mecb', 'metadata.json'):
        assert sha(out / name) == pinned[name], name
    assert text(out / 'program.o')[2] == text(PRIOR / 'program.o')[2] + extra

    command = runtime['link']
    assert command[command.index('-o') + 1] == str(PRIOR / 'mech-worker')
    assert command.count('link-arg=' + str(PRIOR / 'program.o')) == 1
    link = [arg.replace(str(PRIOR / name), str(out / name)) if name else arg
            for arg in command
            for name in [next((name for name in ('mech-worker', 'program.o') if arg.endswith('/' + name)), None)]]
    assert link[link.index('-o') + 1] == str(out / 'mech-worker')
    run(link, out / 'link.log')
    worker = out / 'mech-worker'
    new_stubs = text(worker, (b'__TEXT', b'__stubs'))[1]
    assert new_stubs == candidate_stubs + extra
    assert new_stubs % align == control_stubs % align
    worker_text = text(worker)[2]
    assert worker_text == text(candidate_link)[2] + extra
    provenance = {'flags': flags, 'emit': emit, 'link': link}
    write(out / 'runtime.json', runtime | provenance)

    script = (Q / 'verify-worker.py').read_text()
    target = "fixture_path = root/'fixtures.json'"
    assert script.count(target) == 1
    copied = out / 'verify-worker.py'
    copied.write_text(script.replace(target, f"fixture_path = Path({str(Q / 'fixtures.json')!r})"))
    verified = run([sys.executable, '-B', copied, worker, out / 'metadata.json', 'screen'], out / 'verify.log')
    verify = load(out / 'screen-qualification.json')
    hashes = {filename: sha(out / filename) for filename in ('program.mecb', 'program.o','metadata.json','mech-worker')}
    assert verify['complete_smokes'] == 162 and verify['worker_sha256'] == hashes['mech-worker']
    encoder = text(out / 'program.o')[2]
    report = provenance | {
        'compiler_sha256': PIN,
        'source_sha256': sha(source),
        'prior_rejection_path': str(Q / 'backend-work-hardened/source-owner-exchange-71288-focus/focus-replay.json'),
        'prior_rejection_sha256': sha(Q / 'backend-work-hardened/source-owner-exchange-71288-focus/focus-replay.json'),
        'size_control_hashes': control['sha256'],
        'prior_candidate_hashes': pinned,
        'hashes': hashes,
        'fixed_layout_padding': extra,
        'encoder_text_bytes': encoder,
        'encoder_text_x_roc': encoder / 57696,
        'encoder_text_x_libdeflate': encoder / 37080,
        'encoder_text_savings': control['metrics']['encoder_text_bytes'] - encoder,
        'object_bytes': (out / 'program.o').stat().st_size,
        'linked_text_bytes': worker_text,
        'linked_file_bytes': worker.stat().st_size,
        'stub_locations': {key: hex(value) for key, value in
                           (('size_control', control_stubs), ('prior_candidate', candidate_stubs), ('padded', new_stubs))},
        'verified': verify | {'command': verified},
    }
    write(out / 'proof.json', report)
    print(encoder, report['encoder_text_savings'], hashes['mech-worker'], verify['complete_smokes'], flush=True)


if __name__ == '__main__':
    main()
