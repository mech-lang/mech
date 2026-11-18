"""Attribute the exact 320-byte overlap case to CPU stack frames."""

from pathlib import Path
import runpy
from statistics import median

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
PUBLISHED = '8fb7fa9683a3034866104db196ca5a72d7d312da'
BASE_SCRIPT = ROADMAP / 'scorer-vector-latch/runtime-sample.py'
SAMPLE = runpy.run_path(str(BASE_SCRIPT), run_name='overlap_sample_source')
read, write, sha = (SAMPLE[name] for name in ('read', 'write', 'sha'))
output = HERE / 'samples'
GROUP = '320-overlap'


def main():
    pins = read(Q / 'pinned-peers/pins.json')
    fixtures = Q / 'fixtures.json'
    assert sha(fixtures) == pins['input_inventory_sha256']
    rows = read(fixtures)
    (index, case), = [(i, row) for i, row in enumerate(rows) if row['name'] == GROUP]
    assert sha(case['input']) == case['input_sha256']
    lib = pins['actors']['libdeflate']
    assert sha(lib['command'][0]) == lib['binary_sha256']
    actors = {'libdeflate': lib['command']}
    comparison = {
        'maintained': ROADMAP / 'frame-copy-v2/bound/worker-checks/shared-frame-copies',
        'bottom-tested': ROADMAP / 'scorer-vector-latch/latch-kept-root-layout/bound',
        'historical': Q / 'register-remat-and-compact-native/final-candidate',
    }
    for role, root in comparison.items():
        receipt = read(root / ('receipt.json' if role == 'historical' else 'runtime.json'))
        binary, metadata = [root / name for name in ('mech-worker', 'metadata.json')]
        assert sha(binary) == receipt['worker_sha256']
        assert sha(metadata) == receipt['metadata_sha256']
        actors[role] = list(map(str, (binary, metadata, fixtures)))

    # Use the same stack parser and child protocol as the published 4 KiB sample.
    source = BASE_SCRIPT.read_text()
    before = "('libdeflate compression', 'ordered scorer', 'match collection')"
    after = "('libdeflate compression', 'ordered scorer', 'match collection', 'checked encoder and runtime', 'other numeric phase')"
    assert source.count(before) == 1
    namespace = {'__file__': str(BASE_SCRIPT), '__name__': 'overlap_profile'}
    exec(compile(source.replace(before, after), str(BASE_SCRIPT), 'exec'), namespace)
    namespace['OUTPUT'] = output

    output.mkdir()
    schedule = list(actors) + list(reversed(actors))
    plan = {'source_pin': PUBLISHED, 'case': case, 'sampling_interval_ms': 1,
            'sample_duration_seconds': 3, 'schedule': schedule,
            'script_sha256': sha(Path(__file__)), 'base_script_sha256': sha(BASE_SCRIPT),
            'fixture_inventory_sha256': sha(fixtures), 'actor_commands': actors,
            'actor_sha256': {role: sha(command[0]) for role, command in actors.items()}}
    write(output / 'plan.json', plan)
    observations = []
    for sequence, role in enumerate(schedule, 1):
        label = f'round-{sequence}'
        row = namespace['acquire'](actors[role], label, role, case, index)
        for item in row['checks']:
            assert item['raw_bytes'] == case['raw_bytes']
            assert item['capacity'] == case['capacity']
        write(output / (label + '.json'), row)
        observations.append(row)
    result = {}
    for role in actors:
        acquired = [r for r in observations if r['role'] == role]
        grouped = [r['sample_counts'] for r in acquired]
        labels = set().union(*grouped)
        result[role] = {label: median(r.get(label, 0) for r in grouped) for label in sorted(labels)}
    write(output / 'result.json', {'plan_sha256': sha(output / 'plan.json'), 'median_samples': result,
                                  'observations': observations})
    print(result, flush=True)


if __name__ == '__main__':
    main()
