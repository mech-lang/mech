"""Emit the published source identity and an otherwise identical control and candidate."""
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import subprocess
import sys

from construct import BASE, ENV, HERE, PACKET, PATCH, PUBLIC, PUBLISHED, QUAL, read, run, sha, write

EXPERIMENT = 'MECH_COMPACT_COALESCE_SPILL_SLOTS'
TAIL = 'MECH_COMPACT_ROOT_TAIL_PROFILE'


def source_flags(checkout):
    base = read(PUBLIC / 'recipe-rebuild/rebuild.json')['native_flags']
    assert EXPERIMENT not in base and not any('LLVM' in flag for flag in base)
    phase = 'MECH_COMPACT_PHASE_BLOCK_LAYOUT'
    profiles = []
    for entry in base[phase].split(';'):
        symbol, prior = entry.split('@')
        profile = checkout / PACKET / 'profiles' / Path(prior).name
        assert sha(profile) == sha(prior)
        profiles.append(f'{symbol}@{profile}')
    base[phase] = ';'.join(profiles)
    old = base[TAIL]
    profile = checkout / PACKET / 'profiles' / Path(old).name
    assert sha(profile) == sha(old)
    base[TAIL] = str(profile)
    return base


def read_sizes(path, metadata):
    spec = spec_from_file_location('mach_size', QUAL / 'reachability-census.py')
    macho = module_from_spec(spec)
    spec.loader.exec_module(macho)
    _, sections, _ = macho.load(path)
    return {
        'encoder_text_bytes': sections[macho.TEXT][2],
        'object_bytes': path.stat().st_size,
        'root_code_bytes': metadata['code_bytes'],
        'root_frame_bytes': metadata['stack_bytes'],
    }


def source_emit(out, compiler, source, recipe, options):
    out.mkdir()
    command = run([compiler, source, out, 'compact'], out / 'emit.log',
                  cwd=out, env=ENV | options | {'PATH': '/usr/bin:/bin'})
    files = {name: sha(out / name) for name in ('program.mecb', 'program.o', 'metadata.json')}
    assert files['program.mecb'] == recipe['bytecode_sha256']
    info = read(out / 'metadata.json')
    sizes = read_sizes(out / 'program.o', info)
    stats = [line for line in (out / 'emit.log').read_text().splitlines()
             if line.startswith('compact spill affinity:')]
    return {'emit': command, 'flags': options, 'hashes': files, 'sizes': sizes,
            'spill_statistics': stats}


def link(out, flags):
    old = flags[flags.index('-o') + 1]
    object_argument, = [arg for arg in flags if arg.endswith('/program.o')]
    command = [str(out / 'mech-worker') if arg == old else
               'link-arg=' + str(out / 'program.o') if arg == object_argument else arg
               for arg in flags]
    return run(command, out / 'link.log', cwd=out)


def qualify(out, checkout):
    fixture_file = QUAL / 'fixtures.json'
    source_cases = read(checkout / PACKET / 'fixtures.json')
    cases = read(fixture_file)
    assert len(cases) == len(source_cases) == len({case['name'] for case in cases}) == 81
    for local, source in zip(cases, source_cases):
        for field in ('name', 'input_bytes', 'raw_bytes', 'prefix_bytes', 'capacity', 'input_sha256', 'expected_sha256'):
            assert local[field] == source[field], (field, source['name'])
        assert sha(local['input']) == source['input_sha256']
        assert sha(local['expected']) == source['expected_sha256']
        if 'prefix_sha256' in source:
            assert sha(local['prefix']) == source['prefix_sha256']
    command = list(map(str, (out / 'mech-worker', out / 'metadata.json', fixture_file)))
    rows = []
    error_log = out / 'correctness.stderr'
    with error_log.open('w') as error, subprocess.Popen(
        command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=error, text=True,
        env=ENV, cwd=out,
    ) as worker:
        try:
            assert worker.stdout.readline().strip() == 'SESSION_READY'
            for i, fixture in enumerate(cases):
                worker.stdin.write(f'C\t{i}\t0\nNN')
                worker.stdin.flush()
                assert worker.stdout.readline().strip() == f'CASE_READY\t{fixture["name"]}'
                for _ in range(2):
                    row = read_sample(worker.stdout.readline())
                    assert row['case'] == fixture['name']
                    for field in ('input_bytes', 'raw_bytes', 'prefix_bytes', 'capacity'):
                        assert row[field] == fixture[field]
                    for field in ('exact_reference', 'independent_inflation', 'input_alias_unchanged',
                                  'borrowed_output_alias_unchanged'):
                        assert row[field], (row['case'], field)
                    row.pop('seconds')
                    rows.append(row)
                assert worker.stdout.readline().strip() == f'CASE_DONE\t{fixture["name"]}'
            worker.stdin.write('Q\n')
            worker.stdin.flush()
            assert worker.wait() == 0
        finally:
            if worker.poll() is None:
                worker.terminate()
    assert error_log.read_text().endswith('CLEANUP private_payload_bytes=0 live_owner_records=0\n')
    assert len(rows) == 162
    write(out / 'exact-calls.json', rows)
    return {'command': command, 'exact_calls': len(rows), 'fixture_inventory_sha256': sha(fixture_file)}


from json import loads as read_sample


def main():
    out = HERE / 'trial'
    checkout = out / 'checkout'
    record = read(out / 'build.json')
    compiler = Path(record['compiler'])
    assert sha(compiler) == record['compiler_sha256']
    source = checkout / PACKET / 'program.mec'
    recipe = read(checkout / PACKET / 'recipe.json')
    assert sha(source) == recipe['source_sha256']
    base_hashes = read(PUBLIC / 'verified/exact-source-identity.json')['shared-copies']['artifact_sha256']
    template = read(PUBLISHED / 'runtime.json')['link']
    report = {
        'source_base': BASE, 'input_source_sha256': sha(source),
        'patch_sha256': sha(PATCH), 'compiler_build': record, 'runs': {},
    }
    baseline = source_flags(checkout)
    published = source_emit(out / 'published', compiler, source, recipe, baseline)
    report['runs']['published'] = published
    write(out / 'summary.json', report)
    for name in ('program.o', 'metadata.json'):
        assert published['hashes'][name] == base_hashes[name], (name, published['hashes'])
    unpadded = {k: v for k, v in baseline.items() if k != TAIL}
    for label, options in (('control', unpadded), ('candidate', unpadded | {EXPERIMENT: '1'})):
        report['runs'][label] = source_emit(out / label, compiler, source, recipe, options)
        write(out / 'summary.json', report)
    control = report['runs']['control']['sizes']
    candidate = report['runs']['candidate']['sizes']
    report['static_encoder_savings'] = control['encoder_text_bytes'] - candidate['encoder_text_bytes']
    write(out / 'summary.json', report)
    print('control', control, flush=True)
    print('candidate', candidate, flush=True)
    print('allocator', report['runs']['candidate']['spill_statistics'], flush=True)
    if '--static-only' in sys.argv:
        return
    for label in ('published', 'control', 'candidate'):
        entry = report['runs'][label]
        trial = out / label
        entry['link'] = link(trial, template)
        worker = trial / 'mech-worker'
        entry['hashes']['mech-worker'] = sha(worker)
        if label == 'published':
            assert entry['hashes']['mech-worker'] == base_hashes['mech-worker']
        entry['correctness'] = qualify(trial, checkout)
        write(out / 'summary.json', report)


if __name__ == '__main__':
    main()
