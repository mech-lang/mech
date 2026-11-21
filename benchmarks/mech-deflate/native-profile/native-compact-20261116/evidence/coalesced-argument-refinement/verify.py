"""Verify original-source replays, native gates and all saved elapsed-time observations."""
import argparse
import ast
import bisect
from collections import Counter
from hashlib import sha256
import json
import re
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
    module=run_path(str(HERE.parent/'semantic-block-map/derive_static_register_weights.py'))
    flow=run_path(str(HERE/'derive_layout.py'))['derive']
    for arm in ['predicates','transport','all']:
        generated,proof=module['derive'](root/'study'/(arm+'-raw'))
        saved=root/'study/register-profiles'/arm
        assert proof==read(saved/'derivation.json')
        for name,value in generated.items():assert value==read(saved/(name+'.json'))
        for mode in ['linear','sqrt']:
            name=arm+'-'+mode;folder=root/'study'/(name+'-raw');proof=read(folder/'flow-layout-derivation.json');shape=proof['shape'];counts=read(folder/'counts-475/counts.json')
            assert shape['object_sha256']==sha(folder/'frontend/program.o')
            assert proof['training_counts_sha256']==sha(folder/'counts-475/counts.json')
            assert proof['deriver_sha256']==sha(HERE/'derive_layout.py')
            profile,expected=flow(shape,counts)
            assert {k:proof[k] for k in expected}==json.loads(json.dumps(expected))
            path=root/'study/machine-layouts'/(name+'.json')
            assert profile==read(path) and sha(path)==proof['profile_sha256']
            atom=shape['atom'];body=module['kernel'](folder/'frontend/program.o',atom['start'],atom['extent'])
            assert sha256(body).hexdigest()==atom['sha256']


def counters(root):
    old = PREVIOUS / 'verify.py'
    source = old.read_text()
    tree = ast.parse(source)
    node = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'training_counts')
    text = ast.get_source_segment(source, node)
    start = text.index('    for role, suffix in [')
    end = text.index(':\n        folder = ', start)
    text = text[:start] + "    for role, suffix in [('predicates-raw', '475'), ('predicates-raw', '3956'), ('transport-raw', '475'), ('transport-raw', '3956'), ('all-raw', '475'), ('all-raw', '3956'), ('predicates-linear-raw', '475'), ('predicates-linear-raw', '3956'), ('predicates-sqrt-raw', '475'), ('predicates-sqrt-raw', '3956'), ('transport-linear-raw', '475'), ('transport-linear-raw', '3956'), ('transport-sqrt-raw', '475'), ('transport-sqrt-raw', '3956'), ('all-linear-raw', '475'), ('all-linear-raw', '3956'), ('all-sqrt-raw', '475'), ('all-sqrt-raw', '3956'), ('compact-all-linear', '475'), ('compact-all-linear', '3956')]" + text[end:]
    ns = dict(HERE=HERE, read=read, sha=sha, sha256=sha256, run_path=run_path)
    exec(compile(text, str(old), 'exec'), ns)
    return ns['training_counts'](root)


def sources_and_tests(root):
    compiler=read(root/'study/compiler-source/build.json')
    for name,pin in compiler['source_sha256'].items():assert sha(root/'study/compiler-source/source-inputs'/name)==pin
    execution=root/'study/optimized-execution';records=read(execution/'result.json')
    assert records['environment']['MECH_COMPACT_REFINE_COALESCED_ARGUMENTS']=='all'
    assert records['environment']['MECH_PROVE_BOUNDED_UNSIGNED_INCREMENTS']=='1'
    assert records['executable_sha256']==read(root/'study/optimized-tests-source/build.json')['executables'][0]['sha256']
    for name,count in [('scalar',79),('current_predicates',10),('identical_arguments',2),('selected_increments',4)]:
        assert next(r for r in records['runs'] if r['name']==name)['exit_code']==0
        assert f'test result: ok. {count} passed; 0 failed;' in (execution/(name+'.stdout')).read_text()
    prior=read(HERE.parent/'bounded-unsigned-increments/recipes.json')['roles']['bounded-linear'];disabled=root/'study/disabled'
    assert read(disabled/'receipt.json')['artifacts']==prior['artifacts']
    for name,pin in prior['artifacts'].items():assert sha(disabled/'frontend'/name)==sha(disabled/'checked-emission/frontend'/name)==pin
    for recipe in read(HERE/'recipes.json')['roles'].values():
        assert recipe['source_sha256']==prior['source_sha256']
        assert recipe['artifacts']['program.mecb']==prior['artifacts']['program.mecb']
        assert recipe['compiler_sha256']==compiler['compiler_sha256']
    assert read(root/'study/widths-raw/receipt.json')['artifacts']==read(root/'study/reference-raw/receipt.json')['artifacts']


def huffman_flows(root):
    reference=read(root/'study/reference-raw/receipt.json')['layout']['symbols']
    for role in ['predicates-raw','transport-raw','all-raw']:
        after=read(root/'study'/role/'receipt.json')['layout']['symbols']
        for symbol in ['_mech_compact_numeric_3956_vector','_mech_compact_numeric_3009_no_growth_composed']:
            assert all(reference[symbol][k]==after[symbol][k] for k in ['extent','sha256','relocations'])
    for role in ['compact-all-linear','compact-predicates-linear']:
        proof=read(root/'study'/role/'private-body-identity.json');assert proof['all_selected_private_bodies_exact']
        a,b=[read(root/'study'/r/'receipt.json')['layout']['symbols'] for r in [role,proof['reference_role']]]
        for symbol in proof['private_symbols']:assert all(a[symbol][k]==b[symbol][k] for k in ['extent','sha256','relocations'])


def main_source_recipes(root):
    folder=root/'study/main-default-source-replays';records=read(folder/'verification.json');recipe=read(HERE/'recipes.json')['roles']['compact-all-linear'];proof=records['m1-coalesced-compact-all-linear']
    assert proof['default_invocation'] and proof['all_three_exact'] and proof['artifacts']==recipe['artifacts']
    assert proof['compiler_sha256']==recipe['compiler_sha256']
    assert proof['rebuild_py_sha256']==sha(folder/'rebuild-as-run.py')
    return 1


def sampling(root):
    import gzip
    observations=0
    for name,case in [('runtime-sampling','prefix-random-1048576'),('runtime-sampling-small','prefix-random-4096')]:
        folder=root/'study'/name;records=read(folder/'result.json')
        for actor,record in records.items():
            assert record['sampler_exit_code']==record['worker_exit_code']==0
            local=folder/actor;plan=read(local/'plan.json');assert plan['worker_sha256']==record['worker_sha256']
            assert sha(local/'sample.txt')==record['report_sha256']
            if actor=='mech-all-linear':assert sha(root/'study/all-linear/bound/mech-worker')==record['worker_sha256']
            path=local/('diagnostic-rows.json' if name=='runtime-sampling' else 'diagnostic-rows.json.gz');assert sha(path)==record['rows_sha256']
            stream=iter(read(path)) if path.suffix=='.json' else (json.loads(line) for line in gzip.open(path,'rt'))
            total=0
            for row in stream:
                assert row['case']==case and not row['warmup'] and not row['profile']
                assert row['independent_inflation'] and row['input_alias_unchanged'] and row['borrowed_output_alias_unchanged']
                if actor=='mech-all-linear':assert row['exact_reference']
                total+=1
            assert total==plan['calls'];observations+=total
    return observations

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
        huffman_flows(root)
        counter_cases = counters(root)
        previous['compiler_bindings'](root)
        main_recipes = main_source_recipes(root)
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
            n = 21
            count, _, zero = raw.runtime(root / 'runtime', campaign['runs'], n, 24)
            calls += count
            zeros += zero
            if True:
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
        sampled_observations = sampling(root)
        parity = run_path(str(HERE / 'derive_parity.py'))
        value = parity['derive'](summary)
        assert value == read(HERE / 'parity.json')
        assert parity['render'](value) == (HERE / 'PARITY.md').read_text()
        result = {'passed': True, 'source_replays': replays, 'scalar_tests': 79,
                  'current_word_tests': 10, 'ssa_tests': 2, 'increment_tests': 4,
                  'diagnostic_sampled_observations': sampled_observations,
                  'actual_main_source_recipes': main_recipes,
                  'saved_runtime_calls': calls, 'zero_duration_samples_retained': zeros,
                  'actual_training_counter_cases': counter_cases, 'complete_build_orders': orders,
                  'deployment_executions': deployments, 'selected_default_changed': summary['selected_default_changed'],
                  'new_qualified_C_win': False}
        (HERE / 'verification.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
