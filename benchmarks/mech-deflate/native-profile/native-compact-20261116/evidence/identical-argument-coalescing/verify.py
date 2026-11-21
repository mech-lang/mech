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
    derive = run_path(str(HERE.parent/'semantic-block-map/derive_static_register_weights.py'))['derive']
    for role, folder, saved in [(v,root/'study'/v,root/'study/register-profiles'/v.removesuffix('-raw')) for v in ['vector-raw','shared-vector-raw']] + [(v,root/'study/revisit'/v,root/'study/revisit-register-profiles'/v.removesuffix('-raw')) for v in ['narrow-raw','shift-raw','shared-narrow-raw','shared-shift-raw']]:
        generated,proof=derive(folder)
        assert proof==read(saved/'derivation.json')
        for name,value in generated.items(): assert value==read(saved/(name+'.json'))
    for arm in ['vector','all','shared-vector','shared-all']:
        for mode in ['linear','sqrt']:
            folder=root/'study'/(arm+'-'+mode+'-raw')
            proof=read(folder/'layout-rebase.json')
            assert not proof['ordered_graph_exact'] and proof['source_labels_and_successors_exact']
            reference=root/'reference-layouts'/mode
            old_path=reference/'original-layout.json'
            assert sha(old_path)==proof['reference_profile_sha256']
            old=read(old_path)
            before,after=proof['old_shape'],proof['new_shape']
            assert sha(reference/'frontend/program.o')==before['object_sha256']
            assert sha(folder/'frontend/program.o')==after['object_sha256']
            def labels(path):
                return [(int(i),int(at),ir,edges) for i,at,ir,edges in re.findall(
                    r'^;; REGALLOC_BLOCK (\d+) offset=(\d+) ir=(\S+) successors=(.*)$',path.read_text(),re.M)]
            left=labels(reference/'mech_compact_numeric_475_vector.vcode')
            right=labels(folder/'mech_compact_numeric_475_vector.vcode')
            assert [(i,ir,e) for i,_,ir,e in left]==[(i,ir,e) for i,_,ir,e in right]
            rank={head:i for i,head in enumerate(old['order'])}
            per_label={i:rank[before['heads'][bisect.bisect_right(before['heads'],at)-1]] for i,at,_,_ in left}
            offsets=sorted({at for _,at,_,_ in right})
            scores={}
            for index,head in enumerate(after['heads']):
                end=after['heads'][index+1] if index+1<len(after['heads']) else after['atom']['extent']
                identifiers=[i for i,at,_,_ in right if head<=at<end]
                if not identifiers:
                    at=offsets[max(0,bisect.bisect_right(offsets,head)-1)]
                    identifiers=[i for i,value,_,_ in right if value==at]
                scores[head]=statistics.median(per_label[i] for i in identifiers)
            assert {str(k):v for k,v in scores.items()}==proof['scores']
            generated=old|{'order':sorted(after['heads'],key=lambda h:(scores[h],h)),'literal_offset':after['pool']}
            path=root/'study/machine-layouts-v2'/(arm+'-'+mode+'.json')
            assert generated==read(path) and sha(path)==proof['destination_profile_sha256']


def counters(root):
    old = PREVIOUS / 'verify.py'
    source = old.read_text()
    tree = ast.parse(source)
    node = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'training_counts')
    text = ast.get_source_segment(source, node)
    start = text.index('    for role, suffix in [')
    end = text.index(':\n        folder = ', start)
    text = text[:start] + "    for role, suffix in [('vector-raw', '475'), ('vector-raw', '3956'), ('shared-vector-raw', '475'), ('shared-vector-raw', '3956'), ('vector-linear', '475'), ('vector-linear', '3956'), ('shared-vector-linear', '475'), ('shared-vector-linear', '3956'), ('revisit/narrow-raw', '475'), ('revisit/shift-raw', '475'), ('revisit/shared-narrow-raw', '475'), ('revisit/shared-shift-raw', '475'), ('revisit/shift-linear', '475'), ('revisit/shared-shift-sqrt', '475'), ('revisit/narrow-linear', '475'), ('revisit/shared-narrow-linear', '475')]" + text[end:]
    ns = dict(HERE=HERE, read=read, sha=sha, sha256=sha256, run_path=run_path)
    exec(compile(text, str(old), 'exec'), ns)
    return ns['training_counts'](root)


def sources_and_tests(root):
    compiler = read(root / 'study/compiler-source/build.json')
    for name, pin in compiler['source_sha256'].items():
        assert sha(root / 'study/compiler-source/source-inputs' / name) == pin
    execution = root / 'study/optimized-execution'
    records = read(execution / 'result.json')
    for name, count in [('scalar',79), ('identical_arguments',2)]:
        assert records[name]['exit_code'] == 0
        assert records[name]['flags']['MECH_COMPACT_COALESCE_IDENTICAL_ARGUMENTS'] == 'all'
        assert f'test result: ok. {count} passed; 0 failed;' in (execution / (name+'.stdout')).read_text()
    focus = root / 'study/focused-witness-v2'
    assert read(focus/'execution.json')['exit_code'] == 0
    assert 'test result: ok. 2 passed; 0 failed;' in (focus/'execution.stdout').read_text()
    prior = read(PREVIOUS / 'recipes.json')['roles']['all-sqrt']
    receipt = read(root / 'study/disabled/receipt.json')
    assert receipt['artifacts'] == prior['artifacts']
    for name,pin in prior['artifacts'].items():
        assert sha(root/'study/disabled/frontend'/name) == pin
    for recipe in read(HERE/'recipes.json')['roles'].values():
        assert recipe['source_sha256'] == prior['source_sha256']
        assert recipe['artifacts']['program.mecb'] == prior['artifacts']['program.mecb']


def huffman_flows(root):
    for role in ['vector-raw', 'shared-vector-raw']:
        folder = root / 'study' / role
        proof = read(root / 'study/huffman-layouts' / (role + '-proof.json'))
        profile_path = root / 'study/huffman-layouts' / (role + '.json')
        profile = read(profile_path)
        assert sha(profile_path) == proof['profile_sha256']
        shape = proof['shape']
        assert shape['object_sha256'] == sha(folder / 'frontend/program.o')
        assert shape['symbol'] == 'mech_compact_numeric_3956_vector'
        assert sorted(profile['order']) == shape['heads']
        trace = read(folder / 'counts-3956/counts.json')
        graph = [tuple(dict.fromkeys(v for v in row[1:] if v is not None)) for row in shape['code_blocks']]
        for case, sample in trace['cases'].items():
            visits = {int(b['address'], 16) - shape['atom']['start']: b['visits'] for b in sample['blocks']}
            counts = [visits.get(head, 0) for head in shape['code_heads']]
            constraints = [(counts[src], [(src, dst) for dst in targets])
                           for src, targets in enumerate(graph) if targets]
            constraints += [(counts[dst] - (sample['calls'] if dst == 0 else 0),
                             [(src, dst) for src, targets in enumerate(graph) if dst in targets])
                            for dst in range(len(graph))]
            flows = {}
            changed = True
            while changed:
                changed = False
                for total, edges in constraints:
                    left = total - sum(flows[e] for e in edges if e in flows)
                    assert left >= 0
                    unknown = [edge for edge in edges if edge not in flows]
                    if not unknown:
                        assert left == 0
                    elif left == 0 or len(unknown) == 1:
                        for edge in unknown:
                            flows[edge] = 0 if left == 0 else left
                            changed = True
            expected = proof['cases'][case]
            assert not expected['unresolved_edges']
            assert len(flows) == expected['total_edges'] == expected['exact_constrained_edges']
            assert {str(edge): value for edge, value in flows.items()} == expected['constraint_flows']


def main_source_recipes(root):
    folder = root / 'study/main-default-source-replays'
    records = read(folder / 'verification.json')
    for role in ['vector-linear', 'shared-vector-linear']:
        recipe = read(HERE / 'recipes.json')['roles'][role]
        proof = records['m1-identical-' + role]
        assert proof['all_three_exact'] and proof['artifacts'] == recipe['artifacts']
        assert proof['compiler_sha256'] == recipe['compiler_sha256']
        assert proof['rebuild_py_sha256'] == sha(folder / 'rebuild-as-run.py')
        result = read(folder / role / 'rebuild.json')
        assert result['full_integrated_source_identity'] and result['variant'] == 'm1-identical-' + role
        assert sha(folder / role / 'frontend/program.mecb') == recipe['artifacts']['program.mecb']
        for name in ['program.o', 'metadata.json']:
            assert sha(folder / role / 'native' / name) == recipe['artifacts'][name]
    assert records['m1-identical-shared-vector-linear']['default_invocation']
    return len(records)


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
        parity = run_path(str(HERE / 'derive_parity.py'))
        value = parity['derive'](summary)
        assert value == read(HERE / 'parity.json')
        assert parity['render'](value) == (HERE / 'PARITY.md').read_text()
        result = {'passed': True, 'source_replays': replays, 'scalar_tests': 79,
                  'focused_native_tests': 2, 'focused_native_calls': 7360,
                  'actual_main_source_recipes': main_recipes,
                  'saved_runtime_calls': calls, 'zero_duration_samples_retained': zeros,
                  'actual_training_counter_cases': counter_cases, 'complete_build_orders': orders,
                  'deployment_executions': deployments, 'selected_default_changed': summary['selected_default_changed'],
                  'new_qualified_C_win': False}
        (HERE / 'verification.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
