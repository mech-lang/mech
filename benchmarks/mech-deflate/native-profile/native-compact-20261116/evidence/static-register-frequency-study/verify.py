"""Verify frequency derivation, every candidate, raw runtime calls and builds."""
import ast
import argparse
import bisect
from collections import Counter
from hashlib import sha256
import json
import re
import shutil
from pathlib import Path
from runpy import run_path
import statistics
import struct
import tarfile
import tempfile
import zlib
import verify_runtime as raw

HERE = Path(__file__).resolve().parent
read = lambda p: json.loads(Path(p).read_text())
sha = lambda p: sha256(Path(p).read_bytes()).hexdigest()


def guides(root):
    study = root / 'study'
    shape = read(study / 'training-static-shape.json')
    profile = read(study / 'profiles/selected-old-layout.json')
    old_text = (study / 'training/mech_compact_numeric_475_vector.vcode').read_text()
    labels = lambda text: [(int(i), int(at)) for i, at in re.findall(r'^;; REGALLOC_BLOCK (\d+) offset=(\d+) ', text, re.M)]
    expected = lambda text: text.split(';; REGALLOC_INPUT_BEGIN\n', 1)[1].split(';; REGALLOC_INPUT_END\n', 1)[0]
    old_labels = labels(old_text)
    assert sorted(profile['order']) == shape['heads']
    rank = {head: index for index, head in enumerate(profile['order'])}
    old_rank = {i: rank[shape['heads'][bisect.bisect_right(shape['heads'], at) - 1]] for i, at in old_labels}
    for mode in ['linear-65536', 'sqrt-65536']:
        capture = study / (mode + '-capture')
        proof = read(capture / 'guide-proof.json')
        assert sha(study / 'profiles/selected-old-layout.json') == proof['old_profile_sha256']
        new_text = (capture / 'mech_compact_numeric_475_vector.vcode').read_text()
        assert sha(capture / 'mech_compact_numeric_475_vector.vcode') == proof['new_vcode_sha256']
        assert expected(old_text) == expected(new_text)
        new_labels = labels(new_text)
        assert [i for i, at in old_labels] == [i for i, at in new_labels]
        receipt = read(study / (mode + '-raw/receipt.json'))
        for name, pin in receipt['artifacts'].items():
            assert sha(capture / 'frontend' / name) == pin
        new_shape = read(study / (mode + '-raw/layout-proof.json'))['shape']
        assert new_shape == proof['new_shape'] and shape == proof['old_shape']
        heads = new_shape['heads']
        offsets = sorted(set(at for i, at in new_labels))
        scores = {}
        for index, head in enumerate(heads):
            end = heads[index + 1] if index + 1 < len(heads) else new_shape['atom']['extent']
            ids = [i for i, at in new_labels if head <= at < end]
            if not ids:
                previous = offsets[bisect.bisect_right(offsets, head) - 1]
                ids = [i for i, at in new_labels if at == previous]
            scores[head] = statistics.median(old_rank[i] for i in ids)
        derived = profile | {'order': sorted(heads, key=lambda head: (scores[head], head)), 'literal_offset': new_shape['pool']}
        assert derived == read(study / 'profiles' / (mode + '-previous-layout-guide.json'))
        assert json.loads(json.dumps(scores)) == proof['scores']


def native_counts(root, kernel):
    roles = ['selected-control', *read(HERE / 'recipes.json')['roles']]
    calls = None
    for role in roles:
        folder = root / 'study' / role
        counts_path = folder / 'counts-475/counts.json'
        counts = read(counts_path)
        canonical_path = counts_path.with_name('canonical-counts.json')
        canonical = read(canonical_path)
        receipt = read(folder / 'receipt.json')
        assert sha(canonical_path) == counts['canonical_counts_sha256']
        assert sha(folder / 'frontend/program.o') == counts['object_sha256'] == receipt['artifacts']['program.o']
        canonical_object = folder / 'canonical-counter-input/frontend/program.o'
        assert sha(canonical_object) == canonical['object_sha256']
        atom = receipt['layout']['symbols'][counts['symbol']]
        assert not atom['relocations']
        body = kernel(folder / 'frontend/program.o', atom['start'], atom['extent'])
        assert sha256(body).hexdigest() == atom['sha256']
        assert body == kernel(canonical_object, atom['start'] - counts['address_translation'], atom['extent'])
        plan = read(counts_path.with_name('plan.json'))
        assert plan['source_worker_sha256'] == plan['canonical_worker_sha256'] == sha(folder / 'bound/mech-worker') == sha(folder / 'canonical-counter-input/bound/mech-worker')
        for case in canonical['cases'].values():
            for block in case['blocks']:
                block['address'] = hex(int(block['address'], 16) + counts['address_translation'])
        assert canonical['cases'] == counts['cases']
        current_calls = {case: row['calls'] for case, row in counts['cases'].items()}
        if calls is None:
            calls = current_calls
        assert calls == current_calls and len(calls) == 9
        for row in counts['cases'].values():
            assert row['counts']['instructions'] == sum(b['visits'] * len(b['instructions']) for b in row['blocks'])
    return len(roles) * 9


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--extract-to', type=Path, help='Retain the reconstructed, verified evidence tree at a new directory')
    args = parser.parse_args()
    manifest = read(HERE / 'manifest.json')
    for name, pin in manifest['helper_sha256'].items():
        assert sha(HERE / name) == pin
    # Reuse the published resource/error/recovery and complete-build checks.
    source_path = HERE.parent / 'encoder-size-gap/packet-family-root-study/verify.py'
    source = source_path.read_text()
    tree = ast.parse(source)
    nodes = [n for n in tree.body if isinstance(n, ast.FunctionDef)
             and n.name in ['native', 'complete_builds', 'text_bytes']]
    namespace = dict(HERE=HERE, read=read, sha=sha, sha256=sha256, generate=lambda text: text,
                     Counter=Counter, Path=Path, struct=struct, raw=raw,
                     statistics=statistics, zlib=zlib)
    exec(compile(ast.Module(body=nodes, type_ignores=[]), str(source_path), 'exec'), namespace)
    with tempfile.TemporaryDirectory() as directory:
        root = args.extract_to.resolve() if args.extract_to else Path(directory)
        if args.extract_to:
            root.mkdir(parents=True, exist_ok=False)
        for archive_name, spec in manifest['archives'].items():
            assert sha(HERE / archive_name) == spec['sha256']
            with tarfile.open(HERE / archive_name) as archive:
                assert all(m.isfile() and not Path(m.name).is_absolute() and '..' not in Path(m.name).parts for m in archive.getmembers())
                archive.extractall(root, filter='data')
            if spec.get('storage') == 'sha256':
                for pin, name in spec['objects'].items():
                    assert sha(root / name) == pin
                for name, pin in spec['members'].items():
                    assert not Path(name).is_absolute() and '..' not in Path(name).parts
                    target = root / name
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(root / spec['objects'][pin], target)
            assert all(sha(root / name) == pin for name, pin in spec['members'].items())
        summary = read(HERE / 'summary.json')
        replays = namespace['native'](root)
        derivation = run_path(str(HERE.parent / 'semantic-block-map/derive_static_register_weights.py'))
        profiles, proof = derivation['derive'](root / 'study/training')
        assert proof == read(root / 'study/complete-profiles/derivation.json')
        assert proof['static_heads'] == read(root / 'study/training-static-shape.json')['code_heads']
        for name, profile in profiles.items():
            assert profile == read(root / 'study/profiles' / (name + '.json'))
            assert profile == read(root / 'study/complete-profiles' / (name + '.json'))
            assert all(1 <= value <= 1 << 20 for value in profile['block_frequencies'])
        fixtures = {r['name']: r for r in read(root / 'runtime/fixtures.json')}
        oracle = {r['case']: r for r in read(root / 'runtime/C-output-oracle.json')}

        def validate(actor, value, row):
            fixture = fixtures[row['case']]
            assert all(value[k] == fixture[k] for k in ['input_bytes', 'prefix_bytes'])
            if actor == 'libdeflate':
                assert value['level'] == 12 and value['version'] == '1.26'
                assert all(value[k] == oracle[row['case']][k] for k in ['output_sha256', 'capacity', 'raw_bytes', 'matches_reference_bytes'])
            else:
                assert value['raw_bytes'] == fixture['raw_bytes'] and value['capacity'] == fixture['capacity']

        raw.VALUE_VALIDATOR = validate
        calls = zeros = 0
        for name, campaign in summary['panels'].items():
            assert campaign['complete'] and len(campaign['runs']) == 2
            count, _, zero = raw.runtime(root / 'runtime', campaign['runs'], 21, 24)
            calls += count
            zeros += zero
            for run in campaign['runs']:
                for peer in ['roc', 'libdeflate']:
                    lost = [r['case'] for r in run['results'] if r['paired_x_' + peer]['mech-control'] < 1 <= r['paired_x_' + peer]['mech-experiment']]
                    assert lost == summary['panel_losses'][name][run['order']][peer]
        assert summary['selected_default_changed'] is False
        assert summary['new_qualified_C_win'] is False
        counter_cases = native_counts(root, derivation['kernel'])
        guides(root)
        compiler = read(root / 'study/compiler-source/build.json')
        for name, pin in compiler['source_sha256'].items():
            assert sha(root / 'study/compiler-source/source-inputs' / name) == pin
        for actor, job in read(root / 'builds/plan.json')['actors'].items():
            if actor.startswith('mech-'):
                assert job['compiler_sha256'] == compiler['compiler_sha256']
                assert job['compile'][0] == compiler['compiler']
                assert job['compiler_source_commit'] == compiler['source_base']
                assert job['compiler_source_overlay_manifest_sha256'] == sha(root / 'study/compiler-source/build.json')
        # The complete-build checker derives medians/paired ratios from every
        # result and independently inflates each saved deployment stream.
        build_source = ast.get_source_segment(source, next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'complete_builds'))
        build_source = build_source.replace('assert actors == 6 and orders == len(comparison[\'observations\']) == 12',
                                            "assert actors == 8 and orders == len(comparison['observations']) == 16")
        exec(compile(build_source, str(source_path), 'exec'), namespace)
        orders, deployments = namespace['complete_builds'](root, summary)
        result = {'passed': True, 'source_replays': replays, 'all_saved_runtime_calls': calls,
                  'zero_duration_samples_retained': zeros, 'complete_build_orders': orders,
                  'deployment_executions': deployments, 'selected_default_changed': False,
                  'actual_native_counter_cases': counter_cases,
                  'selected_score': {'Roc': 81, 'libdeflate': 63}, 'new_C_win': False}
        (HERE / 'verification.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
