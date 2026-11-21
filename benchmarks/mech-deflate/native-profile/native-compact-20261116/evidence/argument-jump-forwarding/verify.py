"""Check source-built argument-forwarding candidates and all saved measurements."""
import argparse
import ast
from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
from runpy import run_path
import shutil
import statistics
import struct
import tarfile
import tempfile
import zlib
import verify_runtime as raw

HERE = Path(__file__).resolve().parent
read = lambda p: json.loads(Path(p).read_text())
sha = lambda p: sha256(Path(p).read_bytes()).hexdigest()


def extract(root, spec, archive_path):
    assert sha(archive_path) == spec['sha256']
    with tarfile.open(archive_path) as archive:
        assert all(m.isfile() and not Path(m.name).is_absolute() and '..' not in Path(m.name).parts for m in archive.getmembers())
        archive.extractall(root, filter='data')
    if spec.get('storage') == 'sha256':
        for pin, member in spec['objects'].items():
            assert sha(root / member) == pin
        for member, pin in spec['members'].items():
            assert not Path(member).is_absolute() and '..' not in Path(member).parts
            target = root / member
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(root / spec['objects'][pin], target)
    assert all(sha(root / member) == pin for member, pin in spec['members'].items())


def native_sources(root):
    baseline = read(HERE.parent / 'encoder-size-gap/packet-family-root-study/recipes.json')['roles']['combined']
    disabled = root / 'study/disabled'
    record = read(disabled / 'receipt.json')
    assert record['artifacts'] == baseline['artifacts']
    for name, pin in baseline['artifacts'].items():
        assert sha(disabled / 'frontend' / name) == sha(disabled / 'checked-emission/frontend' / name) == pin
    assert sha(disabled / 'bound/mech-worker') == baseline['worker_sha256']
    assert 'test result: ok. 79 passed; 0 failed;' in (root / 'study/scalar-tests-v2/scalar.stdout.log').read_text()
    assert 'test result: ok. 5 passed; 0 failed;' in (root / 'study/scalar-tests-v2/argument_jumps.stdout.log').read_text()
    tests = read(root / 'study/scalar-tests-v2/execution-result.json')
    assert all(value['exit_code'] == 0 and value['flag'] == 'all' for value in tests.values())
    compiler = read(root / 'study/compiler-source/build.json')
    for name, pin in compiler['source_sha256'].items():
        assert sha(root / 'study/compiler-source/source-inputs' / name) == pin
    rejection = read(root / 'study/profile-reuse-probe/result.json')
    assert not rejection['existing_profiles_accepted'] and rejection['exit_code'] != 0


def profile_derivation(root):
    source = HERE.parent / 'semantic-block-map/derive_static_register_weights.py'
    derive = run_path(str(source))['derive']
    folder = root / 'study/vector-raw'
    profiles, proof = derive(folder)
    assert proof == read(root / 'study/register-profiles/derivation.json')
    for name, value in profiles.items():
        assert value == read(root / 'study/register-profiles' / (name + '.json'))
    legacy = run_path(str(HERE.parent / 'measured-register-weight-study/derive_register_weights.py'))['derive'](folder)['linear']
    neighbor = read(root / 'study/register-profiles/neighbor-4096.json')
    assert {k: v for k, v in neighbor.items() if k != 'scope'} == {k: v for k, v in legacy.items() if k != 'scope'}
    assert 'not actual block frequencies' in neighbor['scope']
    assert len(profiles['linear-4096']['block_frequencies']) == 1127
    for mode in ['linear', 'sqrt', 'neighbor']:
        role = root / 'study' / ('vector-' + mode + '-raw')
        rebase = role / 'layout-rebase.json'
        if rebase.exists():
            value = read(rebase)
            assert value['ordered_graph_exact'] and value['old']['code_blocks'] == value['new']['code_blocks']
            old = read(folder / 'counts-475-scorer-profile.json')
            assert sha(folder / 'counts-475-scorer-profile.json') == value['source_profile_sha256']
            mapping = dict(zip(value['old']['code_heads'], value['new']['code_heads'], strict=True))
            result = old | {'order': [mapping[h] for h in old['order'] if h in mapping] + value['new']['zero_literal_padding'], 'literal_offset': value['new']['pool']}
            actual = root / 'study/register-profiles' / (mode + '-rebased-layout.json')
            assert result == read(actual) and sha(actual) == value['destination_profile_sha256']
        combined = root / 'study' / ('all-' + mode)
        proof = read(combined / 'private-body-identity.json')
        assert proof['all_selected_private_bodies_exact']
        a = read(root / 'study' / ('vector-' + mode) / 'receipt.json')
        b = read(combined / 'receipt.json')
        for symbol in proof['private_symbols']:
            assert all(a['layout']['symbols'][symbol][k] == b['layout']['symbols'][symbol][k] for k in ['extent', 'sha256', 'relocations'])


def training_counts(root):
    """Bind diagnostic training counts to actual uninstrumented native bodies."""
    kernel = run_path(str(HERE.parent / 'semantic-block-map/derive_static_register_weights.py'))['kernel']
    cases = 0
    for role, suffix in [('vector-raw', '475'), ('vector-raw', '3956'),
                         ('vector-linear-raw', '475'), ('vector-sqrt-raw', '475'),
                         ('vector-neighbor-raw', '475')]:
        folder = root / 'study' / role
        path = folder / ('counts-' + suffix) / 'counts.json'
        counts = read(path)
        canonical = read(path.with_name('canonical-counts.json'))
        receipt = read(folder / 'receipt.json')
        assert sha(path.with_name('canonical-counts.json')) == counts['canonical_counts_sha256']
        assert sha(folder / 'frontend/program.o') == counts['object_sha256'] == receipt['artifacts']['program.o']
        original = folder / 'canonical-counter-input/frontend/program.o'
        assert sha(original) == canonical['object_sha256']
        atom = receipt['layout']['symbols'][counts['symbol']]
        body = kernel(folder / 'frontend/program.o', atom['start'], atom['extent'])
        assert sha256(body).hexdigest() == atom['sha256']
        assert body == kernel(original, atom['start'] - counts['address_translation'], atom['extent'])
        plan = read(path.with_name('plan.json'))
        assert plan['source_worker_sha256'] == plan['canonical_worker_sha256']
        assert sha(folder / 'bound/mech-worker') == sha(folder / 'canonical-counter-input/bound/mech-worker') == plan['source_worker_sha256']
        for sample in canonical['cases'].values():
            for block in sample['blocks']:
                block['address'] = hex(int(block['address'], 16) + counts['address_translation'])
        assert canonical['cases'] == counts['cases'] and len(counts['cases']) == 9
        for sample in counts['cases'].values():
            assert sample['counts']['instructions'] == sum(b['visits'] * len(b['instructions']) for b in sample['blocks'])
        cases += len(counts['cases'])
    return cases


def compiler_bindings(root):
    plan = read(root / 'builds/plan.json')
    for actor, job in plan['actors'].items():
        if not actor.startswith('mech-'):
            continue
        name = 'reference-compiler' if actor == 'mech-control-reference' else 'compiler-source'
        source = root / 'study' / name / 'build.json'
        compiler = read(source)
        assert job['compiler_sha256'] == compiler['compiler_sha256']
        assert job['compile'][0] == compiler['compiler']
        assert job['compiler_source_commit'] == compiler['source_base']
        assert job['compiler_source_overlay_manifest_sha256'] == sha(source)
        for path, pin in compiler['source_sha256'].items():
            assert sha(source.parent / 'source-inputs' / path) == pin
    comparison = read(root / 'builds/comparison.json')
    for actor, phases in comparison['phases'].items():
        for phase, value in phases.items():
            raw.close(value, statistics.median(next(p['seconds'] for p in trial[actor]['compiler_phases'] if p['phase'] == phase)
                                              for trial in comparison['observations']))
    peers = read(root / 'study/peer-source-inputs/manifest.json')
    for actor, files in peers.items():
        assert plan['actors'][actor]['codec_source_inputs_sha256'] == {path: value['sha256'] for path, value in files.items()}
        for value in files.values():
            assert sha(root / 'study/peer-source-inputs' / value['file']) == value['sha256']


def main_source_recipes(root):
    folder = root / 'study/main-default-source-replays'
    records = read(folder / 'verification.json')
    for role in ['vector-neighbor', 'all-sqrt']:
        recipe = read(HERE / 'recipes.json')['roles'][role]
        proof = records['m1-argument-' + role]
        assert proof['all_three_exact'] and proof['artifacts'] == recipe['artifacts']
        assert proof['compiler_sha256'] == recipe['compiler_sha256']
        assert proof['rebuild_py_sha256'] == sha(folder / 'rebuild-as-run.py')
        result = read(folder / role / 'rebuild.json')
        assert result['full_integrated_source_identity'] and result['variant'] == 'm1-argument-' + role
        assert sha(folder / role / 'frontend/program.mecb') == recipe['artifacts']['program.mecb']
        for name in ['program.o', 'metadata.json']:
            assert sha(folder / role / 'native' / name) == recipe['artifacts'][name]
    assert records['m1-argument-all-sqrt']['default_invocation']
    return len(records)


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--extract-to', type=Path)
    args = parser.parse_args()
    manifest = read(HERE / 'manifest.json')
    for name, pin in manifest['helper_sha256'].items():
        assert sha(HERE / name) == pin
    helper = HERE.parent / 'encoder-size-gap/packet-family-root-study/verify.py'
    tree = ast.parse(helper.read_text())
    nodes = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in ['native', 'text_bytes', 'complete_builds']]
    namespace = dict(HERE=HERE, read=read, sha=sha, sha256=sha256, Counter=Counter, Path=Path,
                     generate=lambda source: source, struct=struct, statistics=statistics, zlib=zlib, raw=raw)
    exec(compile(ast.Module(body=nodes, type_ignores=[]), str(helper), 'exec'), namespace)
    with tempfile.TemporaryDirectory() as directory:
        root = args.extract_to.resolve() if args.extract_to else Path(directory)
        if args.extract_to:
            root.mkdir(parents=True, exist_ok=False)
        for name, spec in manifest['archives'].items():
            extract(root, spec, HERE / name)
        native_sources(root)
        replays = namespace['native'](root)
        profile_derivation(root)
        counter_cases = training_counts(root)
        compiler_bindings(root)
        main_recipes = main_source_recipes(root)
        summary = read(HERE / 'summary.json')
        parity = run_path(str(HERE / 'derive_parity.py'))
        value = parity['derive'](summary)
        assert value == read(HERE / 'parity.json')
        assert parity['render'](value) == (HERE / 'PARITY.md').read_text()
        fixtures = {value['name']: value for value in read(root / 'runtime/fixtures.json')}
        oracle = {value['case']: value for value in read(root / 'runtime/C-output-oracle.json')}

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
        policy = read(root / 'runtime/working-selection-policy.json')
        for role, campaign in summary['panels'].items():
            assert campaign['complete'] and len(campaign['runs']) == 2
            for run in campaign['runs']:
                for peer, required in policy['required_peer_wins'].items():
                    key = 'paired_x_' + peer
                    losses = [r['case'] for r in run['results'] if r[key]['mech-control'] < 1 <= r[key]['mech-experiment']]
                    assert losses == summary['panel_losses'][role][run['order']][peer]
                    cases = {r['case'] for r in run['results']}
                    wins = {r['case'] for r in run['results'] if r[key]['mech-experiment'] < 1}
                    assert sorted((set(required) & cases) - wins) == summary['required_named_panel_misses'][role][run['order']][peer]
            count, _, zero = raw.runtime(root / 'runtime', campaign['runs'], 21, 24)
            calls += count
            zeros += zero
        for role, campaign in summary['full_qualification'].items():
            assert campaign['complete'] and len(campaign['runs']) == 2
            assert sha(root / 'runtime/working-selection-policy.json') == campaign['plan']['policy_sha256']
            assert campaign['plan']['candidate_worker'] == read(HERE / 'recipes.json')['roles'][role]['worker_sha256']
            for run in campaign['runs']:
                for peer, names in policy['required_peer_wins'].items():
                    wins = {r['case'] for r in run['results'] if r['paired_x_' + peer]['mech-experiment'] < 1}
                    missing = sorted(set(names) - wins)
                    assert run['peers'][peer] == {'wins': len(wins), 'required_missing': missing,
                                                   'passed': not missing and len(wins) >= policy['peer_win_floors'][peer]}
            point = all(v['passed'] for run in campaign['runs'] for v in run['peers'].values())
            assert point == campaign['point_passed']
            count, _, zero = raw.runtime(root / 'runtime', campaign['runs'], 81, 96)
            calls += count
            zeros += zero
        # The completed cohort declares and checks its own actor/order counts.
        source = ast.get_source_segment(helper.read_text(), next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'complete_builds'))
        source = source.replace("assert actors == 6 and orders == len(comparison['observations']) == 12", "assert actors == 12 and orders == len(comparison['observations']) == 12")
        source = source.replace('dict.fromkeys(plan[\'actors\'],2)', "dict.fromkeys(plan['actors'],1)")
        exec(compile(source, str(helper), 'exec'), namespace)
        orders, deployments = namespace['complete_builds'](root, summary)
        result = {'passed': True, 'source_replays': replays, 'scalar_tests': 79, 'SSA_tests': 5,
                  'actual_main_named_recipes': main_recipes,
                  'saved_runtime_calls': calls, 'zero_duration_samples_retained': zeros,
                  'actual_training_counter_cases': counter_cases,
                  'complete_build_orders': orders, 'deployment_executions': deployments,
                  'selected_default_changed': summary['selected_default_changed'], 'new_qualified_C_win': False}
        (HERE / 'verification.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
