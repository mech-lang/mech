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
    generated, proof = derive(root/'study/bounded-raw')
    assert proof == read(root/'study/register-profiles/derivation.json')
    for name, value in generated.items():
        assert value == read(root/'study/register-profiles'/(name+'.json'))
    for mode in ['linear','sqrt']:
        folder=root/'study'/('bounded-'+mode+'-raw')
        proof=read(folder/'flow-layout-derivation.json')
        shape=proof['shape']; profile=read(root/'study/machine-layouts'/('bounded-'+mode+'.json'))
        assert shape['object_sha256']==sha(folder/'frontend/program.o')
        assert sorted(profile['order'])==shape['heads']
        trace=read(folder/'counts-475/counts.json')
        graph=[tuple(dict.fromkeys(v for v in row[1:] if v is not None)) for row in shape['code_blocks']]
        incoming={dst:[(src,dst) for src,targets in enumerate(graph) if dst in targets] for dst in range(len(graph))}
        for case,sample in trace['cases'].items():
            visits={int(b['address'],16)-shape['atom']['start']:b['visits'] for b in sample['blocks']}
            counts=[visits.get(h,0) for h in shape['code_heads']]
            assert counts[0]==sample['calls']
            constraints=[(counts[src],[(src,dst) for dst in targets]) for src,targets in enumerate(graph) if targets]
            constraints += [(counts[dst]-(sample['calls'] if dst==0 else 0),incoming[dst]) for dst in range(len(graph))]
            flows={};changed=True
            while changed:
                changed=False
                for total,edges in constraints:
                    remaining=total-sum(flows[e] for e in edges if e in flows);assert remaining>=0
                    missing=[e for e in edges if e not in flows]
                    if not missing:assert remaining==0
                    elif remaining==0 or len(missing)==1:
                        for edge in missing:flows[edge]=0 if remaining==0 else remaining;changed=True
            edges=[(src,dst) for src,targets in enumerate(graph) for dst in targets]
            unresolved=[edge for edge in edges if edge not in flows];scores=dict(flows)
            for src,dst in unresolved:
                out=counts[src]-sum(flows.get((src,target),0) for target in graph[src])
                inc=counts[dst]-sum(flows.get(edge,0) for edge in incoming[dst])
                scores[src,dst]=min(out/sum((src,target) not in flows for target in graph[src]),inc/sum(edge not in flows for edge in incoming[dst]))
            expected=proof['cases'][case]
            assert expected['function_calls']==sample['calls']
            assert expected['exact_constrained_edges']==len(flows) and expected['total_edges']==len(edges)
            assert expected['unresolved_edges']==[list(e) for e in unresolved]
            assert expected['constraint_flows']=={str(e):v for e,v in flows.items()}
            assert expected['layout_priority_scores']=={str(e):v for e,v in scores.items()}
        guide=read(folder/'source-label-layout-derivation.json')
        reference=root/'reference-layouts'/mode
        old=read(reference/'original-layout.json')
        assert sha(reference/'original-layout.json')==guide['reference_profile_sha256']
        assert sha(reference/'mech_compact_numeric_475_vector.vcode')==guide['reference_vcode_sha256']
        assert sha(folder/'mech_compact_numeric_475_vector.vcode')==guide['candidate_vcode_sha256']
        before,after=guide['reference_shape'],guide['candidate_shape']
        assert sha(reference/'frontend/program.o')==before['object_sha256']
        assert after==shape
        def labels(p):
            return [(int(i),int(at),ir,edges) for i,at,ir,edges in re.findall(r'^;; REGALLOC_BLOCK (\d+) offset=(\d+) ir=(\S+) successors=(.*)$',p.read_text(),re.M)]
        left,right=labels(reference/'mech_compact_numeric_475_vector.vcode'),labels(folder/'mech_compact_numeric_475_vector.vcode')
        ranks={h:i for i,h in enumerate(old['order'])};by_ir={}
        for i,at,ir,edges in left:
            if ir!='None':by_ir.setdefault(ir,[]).append(ranks[before['heads'][bisect.bisect_right(before['heads'],at)-1]]/len(before['heads']))
        fallback={h:i/len(shape['heads']) for i,h in enumerate(profile['order'])};offsets=sorted({at for _,at,_,_ in right});scores={};matches={}
        for index,head in enumerate(shape['heads']):
            end=shape['heads'][index+1] if index+1<len(shape['heads']) else shape['atom']['extent'];irs=[ir for _,at,ir,_ in right if head<=at<end]
            if not irs:
                at=offsets[max(0,bisect.bisect_right(offsets,head)-1)];irs=[ir for _,where,ir,_ in right if where==at]
            values=[rank for ir in irs for rank in by_ir.get(ir,[])];scores[head]=statistics.median(values) if values else fallback[head];matches[head]=bool(values)
        assert guide['scores']=={str(k):v for k,v in scores.items()} and guide['matched']=={str(k):v for k,v in matches.items()}
        assert guide['matched_heads']==sum(matches.values()) and guide['total_heads']==len(matches)
        expected=profile|{'order':sorted(shape['heads'],key=lambda h:(scores[h],h))}
        path=root/'study/machine-layouts'/('bounded-'+mode+'-guide.json')
        assert expected==read(path) and sha(path)==guide['profile_sha256']


def counters(root):
    old = PREVIOUS / 'verify.py'
    source = old.read_text()
    tree = ast.parse(source)
    node = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'training_counts')
    text = ast.get_source_segment(source, node)
    start = text.index('    for role, suffix in [')
    end = text.index(':\n        folder = ', start)
    text = text[:start] + "    for role, suffix in [('bounded-raw', '475'), ('bounded-raw', '3956'), ('bounded-linear-raw', '475'), ('bounded-linear-raw', '3956'), ('bounded-sqrt-raw', '475'), ('bounded-sqrt-raw', '3956'), ('bounded-linear', '475'), ('bounded-linear', '3956')]" + text[end:]
    ns = dict(HERE=HERE, read=read, sha=sha, sha256=sha256, run_path=run_path)
    exec(compile(text, str(old), 'exec'), ns)
    return ns['training_counts'](root)


def sources_and_tests(root):
    compiler=read(root/'study/compiler-source/build.json')
    for name,pin in compiler['source_sha256'].items():assert sha(root/'study/compiler-source/source-inputs'/name)==pin
    execution=root/'study/optimized-execution';records=read(execution/'result.json')
    assert records['environment']['MECH_PROVE_BOUNDED_UNSIGNED_INCREMENTS']=='1'
    assert records['executable_sha256']==read(root/'study/optimized-tests-source/build.json')['executables'][0]['sha256']
    for name,count in [('scalar',79),('selected_increments',4),('word_ranges',3)]:
        assert next(r for r in records['runs'] if r['name']==name)['exit_code']==0
        assert f'test result: ok. {count} passed; 0 failed;' in (execution/(name+'.stdout')).read_text()
    focus=root/'study/focused-tests-v2';plan=read(focus/'plan.json')
    assert read(focus/'execution.json')['exit_code']==0
    assert 'test result: ok. 6 passed; 0 failed;' in (focus/'execution.stdout').read_text()
    for name,pin in plan['source_sha256'].items():assert sha(focus/'source-inputs'/name)==pin
    assert sha(focus/'witness.rs')==plan['witness_sha256']
    prior=read(HERE.parent/'identical-argument-coalescing/recipes.json')['roles']['shared-vector-linear'];disabled=root/'study/disabled'
    assert read(disabled/'receipt.json')['artifacts']==prior['artifacts']
    for name,pin in prior['artifacts'].items():assert sha(disabled/'frontend'/name)==sha(disabled/'checked-emission/frontend'/name)==pin
    for recipe in read(HERE/'recipes.json')['roles'].values():
        assert recipe['source_sha256']==prior['source_sha256']
        assert recipe['artifacts']['program.mecb']==prior['artifacts']['program.mecb']
        assert recipe['compiler_sha256']==compiler['compiler_sha256']


def huffman_flows(root):
    for before,after in [('reference-raw','bounded-raw'),('reference-selected','bounded-linear')]:
        a,b=[read(root/'study'/role/'receipt.json')['layout']['symbols'] for role in [before,after]]
        for symbol in ['_mech_compact_numeric_3956_vector','_mech_compact_numeric_3009_no_growth_composed']:
            assert all(a[symbol][k]==b[symbol][k] for k in ['extent','sha256','relocations'])
    a=read(root/'study/reference-raw/receipt.json')['layout']['symbols'];b=read(root/'study/bounded-raw/receipt.json')['layout']['symbols'];assert set(a)==set(b)
    assert [name for name in sorted(a) if any(a[name][k]!=b[name][k] for k in ['extent','sha256','relocations'])]==['_mech_compact_numeric_475_vector']


def main_source_recipes(root):
    folder=root/'study/main-default-source-replays';records=read(folder/'verification.json');recipe=read(HERE/'recipes.json')['roles']['bounded-linear'];proof=records['m1-bounded-increment-vector-linear']
    assert proof['default_invocation'] and proof['all_three_exact'] and proof['artifacts']==recipe['artifacts']
    assert proof['compiler_sha256']==recipe['compiler_sha256']
    assert proof['rebuild_py_sha256']==sha(folder/'rebuild-as-run.py')
    return 1


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
                  'focused_tests': 6, 'native_increment_tests': 3, 'focused_native_calls': 169600,
                  'actual_main_source_recipes': main_recipes,
                  'saved_runtime_calls': calls, 'zero_duration_samples_retained': zeros,
                  'actual_training_counter_cases': counter_cases, 'complete_build_orders': orders,
                  'deployment_executions': deployments, 'selected_default_changed': summary['selected_default_changed'],
                  'new_qualified_C_win': False}
        (HERE / 'verification.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
