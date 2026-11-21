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

def sources_and_tests(root):
    for folder, expected_calls in [('study',40725),('prior-scalar',20196)]:
        compiler=read(root/folder/'compiler-source/build.json')
        assert len(compiler['source_sha256'])==51
        for name,pin in compiler['source_sha256'].items():assert sha(root/folder/'compiler-source/source-inputs'/name)==pin
        execution=root/folder/'optimized-execution';records=read(execution/'result.json')
        assert records['environment']['MECH_HISTOGRAM_ENTRY_COUNTER_CAPACITY']=='1'
        if folder=='study':
            assert records['environment']['MECH_HISTOGRAM_GROUP_BYTE_VISITS']=='1'
            assert records['environment']['MECH_HISTOGRAM_VECTOR_COUNTER_SCAN']=='1'
        assert records['focused_native_calls']==expected_calls
        assert records['executable_sha256']==read(root/folder/'optimized-tests-source/build.json')['executables'][0]['sha256']
        for name,count in [('histogram',5),('scalar',79),('current_predicates',10),('identical_arguments',2),('selected_increments',4)]:
            assert next(r for r in records['runs'] if r['name']==name)['exit_code']==0
            assert f'test result: ok. {count} passed; 0 failed;' in (execution/(name+'.stdout')).read_text()
    reference=read(HERE.parent/'coalesced-argument-refinement/recipes.json')['roles']['compact-all-linear']
    disabled=read(root/'study/disabled/receipt.json');scalar=read(root/'study/capacity/receipt.json')
    assert disabled['artifacts']==reference['artifacts']
    assert scalar['artifacts']==read(root/'prior-scalar/capacity/receipt.json')['artifacts']
    for recipe in read(HERE/'recipes.json')['roles'].values():
        assert recipe['source_sha256']==reference['source_sha256']
        assert recipe['artifacts']['program.mecb']==reference['artifacts']['program.mecb']
        assert recipe['compiler_sha256']==read(root/'study/compiler-source/build.json')['compiler_sha256']
        assert recipe['profiles']==reference['profiles']
        for mode in ['normal','checked']:
            replay=read(root/'study/public-replays'/(recipe['local_role']+'-'+mode)/'reproduction.json')
            assert replay['compiler_sha256']==recipe['compiler_sha256']
            assert replay['source_sha256']==reference['source_sha256']
    original=read(root/'study/reference-code/receipt.json')['layout']['symbols']
    kernel=run_path(str(HERE.parent/'semantic-block-map/derive_static_register_weights.py'))['kernel']
    for role in read(HERE/'recipes.json')['roles']:
        folder=root/'study'/role;symbols=read(folder/'receipt.json')['layout']['symbols']
        for name in ['_mech_compact_numeric_475_vector','_mech_compact_numeric_3956_vector','_mech_compact_numeric_3009_no_growth_composed']:
            a,b=original[name],symbols[name]
            assert all(a[k]==b[k] for k in ['extent','sha256','relocations'])
            before=kernel(root/'study/reference-code/frontend/program.o',a['start'],a['extent'])
            after=kernel(folder/'frontend/program.o',b['start'],b['extent'])
            assert before==after and sha256(after).hexdigest()==b['sha256']


def compiler_bindings(root):
    plan=read(root/'builds/plan.json')
    for actor,job in plan['actors'].items():
        if not actor.startswith('mech-'):continue
        folder='reference-compiler' if actor=='mech-control-reference' else ('retained-compiler' if actor=='mech-retained-bounded' else 'compiler-source')
        build=root/'study'/folder/'build.json';record=read(build)
        assert sha(build)==job['compiler_source_overlay_manifest_sha256']
        assert record['compiler_sha256']==job['compiler_sha256']
        for name,pin in record['source_sha256'].items():assert sha(build.parent/'source-inputs'/name)==pin


def main_source_recipes(root):
    summary=read(HERE/'summary.json');role=summary['selected_role'];name=summary['selected_recipe']
    folder=root/'study/main-default-source-replays';records=read(folder/'verification.json');recipe=read(HERE/'recipes.json')['roles'][role];proof=records[name]
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
            if actor=='mech-selected':assert sha(root/'study/capacity-grouped/bound/mech-worker')==record['worker_sha256']
            path=local/('diagnostic-rows.json' if name=='runtime-sampling' else 'diagnostic-rows.json.gz');assert sha(path)==record['rows_sha256']
            stream=iter(read(path)) if path.suffix=='.json' else (json.loads(line) for line in gzip.open(path,'rt'))
            total=0
            for row in stream:
                assert row['case']==case and not row['warmup'] and not row['profile']
                assert row['independent_inflation'] and row['input_alias_unchanged'] and row['borrowed_output_alias_unchanged']
                if actor=='mech-selected':assert row['exact_reference']
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
        compiler_bindings(root)
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
            count, _, zero = raw.runtime(root / 'runtime', campaign['runs'], n, campaign['plan']['blocks_per_case'])
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
        sampled_observations=sampling(root)
        assert run_path(str(HERE/'derive_sampling.py'))['derive'](root/'study')==read(HERE/'sampling.json')
        import tarfile
        prior_manifest=read(HERE.parent/'coalesced-argument-refinement/manifest.json')
        spec=prior_manifest['archives']['native-and-builds.tar.gz']
        archive=HERE.parent/'coalesced-argument-refinement/native-and-builds.tar.gz'
        assert sha(archive)==spec['sha256']
        pin=spec['members']['study/runtime-sampling-small/libdeflate/sample.txt']
        with tarfile.open(archive,'r:gz') as saved:data=saved.extractfile(spec['objects'][pin]).read()
        assert sha256(data).hexdigest()==pin
        prior_report=root/'prior-C4KiB-sample.txt';prior_report.write_bytes(data)
        correction=read(HERE/'prior-sampling-correction.json')
        assert correction['source_report_sha256']==pin
        assert run_path(str(HERE/'derive_sampling.py'))['one'](prior_report,'libdeflate_deflate_compress')==correction['encoder']
        assert sampled_observations==200800
        parity = run_path(str(HERE / 'derive_parity.py'))
        value = parity['derive'](summary)
        assert value == read(HERE / 'parity.json')
        assert parity['render'](value) == (HERE / 'PARITY.md').read_text()
        result = {'passed': True, 'source_replays': replays, 'scalar_tests': 79,
                  'current_word_tests': 10, 'ssa_tests': 2, 'increment_tests': 4,
                  'diagnostic_sampled_observations':sampled_observations,'histogram_tests':5,'focused_native_calls':40725,'retained_scalar_focused_native_calls':20196,
                  'actual_main_source_recipes': main_recipes,
                  'saved_runtime_calls': calls, 'zero_duration_samples_retained': zeros,
                  'complete_build_orders': orders,
                  'deployment_executions': deployments, 'selected_default_changed': summary['selected_default_changed'],
                  'new_qualified_C_win': summary['score']['libdeflate']>63}
        (HERE / 'verification.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))

if __name__=='__main__':
    main()
