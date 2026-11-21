"""Verify original-source replays, native gates and all saved elapsed-time observations."""
import argparse
import ast
from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
from runpy import run_path
import statistics
import struct
import sys
import tempfile
import zlib

HERE = Path(__file__).resolve().parent
PREVIOUS = HERE.parent / 'argument-jump-forwarding'
sys.path.insert(0, str(PREVIOUS))
import verify_runtime as raw

read = lambda p: json.loads(Path(p).read_text())
sha = lambda p: sha256(Path(p).read_bytes()).hexdigest()


def helpers():
    path = HERE.parent / 'encoder-size-gap/packet-family-root-study/verify.py'
    source = path.read_text()
    tree = ast.parse(source)
    nodes = [n for n in tree.body if isinstance(n, ast.FunctionDef)
             and n.name in ['text_bytes', 'native', 'complete_builds']]
    # Raw discovery arms were checked through their public checked replay.
    text = '\n\n'.join(ast.get_source_segment(source, n) for n in nodes)
    text = text.replace("sha(role/'checked-emission/frontend'/file)",
                        "sha(root/'study/public-replays'/(name+'-checked')/'frontend'/file)")
    namespace = dict(HERE=HERE, read=read, sha=sha, sha256=sha256, Counter=Counter,
                     Path=Path, generate=lambda source: source, struct=struct,
                     statistics=statistics, zlib=zlib, raw=raw)
    exec(compile(text, str(path), 'exec'), namespace)
    return namespace


def profiles(root):
    derive = run_path(str(HERE.parent / 'semantic-block-map/derive_static_register_weights.py'))['derive']
    for role in ['raw-control', 'shared-raw']:
        folder = root / 'study' / role
        generated, proof = derive(folder)
        saved = root / 'study/register-profiles' / role
        assert proof == read(saved / 'derivation.json')
        for name, value in generated.items():
            assert value == read(saved / (name + '.json'))
    for role in ['raw-control-sqrt-raw', 'shared-raw-sqrt-raw',
                 'raw-control-linear-raw', 'shared-raw-linear-raw']:
        proof = read(root / 'study' / role / 'layout-rebase.json')
        assert proof['ordered_graph_exact'] or proof.get('source_label_guide', False)


def counters(root):
    old = PREVIOUS / 'verify.py'
    source = old.read_text()
    tree = ast.parse(source)
    node = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'training_counts')
    text = ast.get_source_segment(source, node)
    start = text.index('    for role, suffix in [')
    end = text.index(':\n        folder = ', start)
    text = text[:start] + "    for role, suffix in [('raw-control', '475'), ('raw-control', '3956'), ('shared-raw', '475'), ('shared-raw', '3956'), ('shared-raw-linear', '475'), ('shared-raw-sqrt', '475')]" + text[end:]
    ns = dict(HERE=HERE, read=read, sha=sha, sha256=sha256, run_path=run_path)
    exec(compile(text, str(old), 'exec'), ns)
    return ns['training_counts'](root)


def sources_and_tests(root):
    compiler = read(root / 'study/compiler-source/build.json')
    for name, pin in compiler['source_sha256'].items():
        assert sha(root / 'study/compiler-source/source-inputs' / name) == pin
    scalar = read(root / 'study/scalar-execution/result.json')
    assert scalar['exit_code'] == 0 and scalar['flags']['MECH_ARM64_CCMP_SHARED_BOOLEAN'] == '1'
    assert 'test result: ok. 79 passed; 0 failed;' in (root / 'study/scalar-execution/stdout.log').read_text()
    focus = root / 'study/focused-witness-v3'
    plan, execution = read(focus / 'plan.json'), read(focus / 'execution.json')
    assert execution['flag'] == '1' and execution['exit_code'] == 0
    assert 'test result: ok. 2 passed; 0 failed;' in (focus / 'execution.stdout').read_text()
    name = 'hosts/gpu/src/native_artifact/program/codegen/shared_boolean_tests.rs'
    assert plan['source_sha256'] == compiler['source_sha256'][name]
    assert len(plan['crate_sha256']) == 4
    prior = read(PREVIOUS / 'recipes.json')['roles']['all-sqrt']
    for role in ['disabled', 'raw-control-sqrt']:
        receipt = read(root / 'study' / role / 'receipt.json')
        assert receipt['artifacts'] == prior['artifacts']
        for name, pin in prior['artifacts'].items():
            assert sha(root / 'study' / role / 'frontend' / name) == pin
    assert sha(root / 'study/raw-control-sqrt/bound/mech-worker') == prior['worker_sha256']
    for recipe in read(HERE / 'recipes.json')['roles'].values():
        assert recipe['source_sha256'] == prior['source_sha256']
        assert recipe['artifacts']['program.mecb'] == prior['artifacts']['program.mecb']


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--extract-to', type=Path)
    args = parser.parse_args()
    manifest = read(HERE / 'manifest.json')
    for name, pin in manifest['helper_sha256'].items():
        assert sha(HERE / name) == pin
    previous = run_path(str(PREVIOUS / 'verify.py'), run_name='shared_previous_verifier')
    functions = helpers()
    with tempfile.TemporaryDirectory() as directory:
        root = args.extract_to.resolve() if args.extract_to else Path(directory)
        if args.extract_to:
            root.mkdir(parents=True, exist_ok=False)
        for name, spec in manifest['archives'].items():
            previous['extract'](root, spec, HERE / name)
        replays = functions['native'](root)
        sources_and_tests(root)
        profiles(root)
        counter_cases = counters(root)
        previous['compiler_bindings'](root)
        summary = read(HERE / 'summary.json')
        fixtures = {v['name']: v for v in read(root / 'runtime/fixtures.json')}
        oracle = {v['case']: v for v in read(root / 'runtime/C-output-oracle.json')}

        def validate(actor, value, row):
            fixture = fixtures[row['case']]
            assert all(value[k] == fixture[k] for k in ['input_bytes', 'prefix_bytes'])
            assert not value['warmup'] and not value['profile']
            if actor == 'libdeflate':
                assert value['level'] == 12 and value['version'] == '1.26'
                assert all(value[k] == oracle[row['case']][k]
                           for k in ['output_sha256', 'capacity', 'raw_bytes', 'matches_reference_bytes'])
            else:
                assert value['raw_bytes'] == fixture['raw_bytes'] and value['capacity'] == fixture['capacity']

        raw.VALUE_VALIDATOR = validate
        calls = zeros = 0
        policy = read(root / 'runtime/working-selection-policy.json')
        recipes = read(HERE / 'recipes.json')['roles']
        for role, campaign in summary['panels'].items():
            assert campaign['complete'] and len(campaign['runs']) == 2
            n = 11 if role == 'unprofiled' else 21
            count, _, zero = raw.runtime(root / 'runtime', campaign['runs'], n, 24)
            calls += count
            zeros += zero
            if role != 'unprofiled':
                for run in campaign['runs']:
                    cases = {r['case'] for r in run['results']}
                    for peer, required in policy['required_peer_wins'].items():
                        wins = {r['case'] for r in run['results']
                                if r['paired_x_' + peer]['mech-experiment'] < 1}
                        assert sorted((set(required) & cases) - wins) == run['required_named_missing'][peer]
        for role, campaign in summary['full_qualification'].items():
            assert campaign['complete'] and campaign['point_passed'] and len(campaign['runs']) == 2
            assert campaign['plan']['policy_sha256'] == sha(root / 'runtime/working-selection-policy.json')
            assert campaign['plan']['candidate_worker'] == recipes[role]['worker_sha256']
            for run in campaign['runs']:
                for peer, names in policy['required_peer_wins'].items():
                    wins = {r['case'] for r in run['results'] if r['paired_x_' + peer]['mech-experiment'] < 1}
                    missing = sorted(set(names) - wins)
                    result = {'wins': len(wins), 'required_missing': missing,
                              'passed': not missing and len(wins) >= policy['peer_win_floors'][peer]}
                    assert run['peers'][peer] == result and result['passed']
            count, _, zero = raw.runtime(root / 'runtime', campaign['runs'], 81, 96)
            calls += count
            zeros += zero
        orders, deployments = functions['complete_builds'](root, summary)
        parity = run_path(str(HERE / 'derive_parity.py'))
        value = parity['derive'](summary)
        assert value == read(HERE / 'parity.json')
        assert parity['render'](value) == (HERE / 'PARITY.md').read_text()
        result = {'passed': True, 'source_replays': replays, 'scalar_tests': 79,
                  'focused_native_tests': 2, 'focused_native_calls': 103424,
                  'saved_runtime_calls': calls, 'zero_duration_samples_retained': zeros,
                  'actual_training_counter_cases': counter_cases, 'complete_build_orders': orders,
                  'deployment_executions': deployments, 'selected_default_changed': False,
                  'new_qualified_C_win': False}
        (HERE / 'verification.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
