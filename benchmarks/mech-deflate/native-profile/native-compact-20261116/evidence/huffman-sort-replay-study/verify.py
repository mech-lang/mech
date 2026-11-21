"""Portable checks of source, exact native rebuilds and every saved observation."""
from collections import Counter
from hashlib import sha256
from itertools import permutations
import json
import math
from pathlib import Path
import statistics
import tarfile
import tempfile
import zlib
import generate
import generate_c
import generate_codec
from build import text_size
import verify_runtime as raw

HERE = Path(__file__).resolve().parent
read = lambda p:json.loads(Path(p).read_text())
sha = lambda p:sha256(Path(p).read_bytes()).hexdigest()


def standalone(root, summary):
    base = root/'standalone'
    build = read(base/'native-v2/build.json')
    assert build == summary['standalone_build'] and build['complete']
    vectors = read(base/'native-v2/vectors.json')
    assert sha(base/'native-v2/vectors.json') == build['vectors_sha256']
    assert sum(len(c['vectors']) for c in vectors) == 212
    original = (root/'original-program.mec').read_text()
    assert sha(root/'original-program.mec') == generate.SOURCE
    assert generate_c.generate((base/'original-C.c').read_text()) == (base/'native-v2/sort.c').read_text()
    for case in vectors:
        capture = read(base/'captured-C-inputs'/(case['name']+'.json'))['frequency_vectors']
        assert len(capture) == len(case['vectors'])
        for index, vector in enumerate(case['vectors']):
            freqs = vector['freqs']
            assert freqs == capture[index]['freqs'] and len(freqs) in (19,32,288)
            assert sum(freqs) < 1<<22
            assert vector['expected'] == sorted((f<<10)|i for i,f in enumerate(freqs) if f)
    for role, recipe in build['roles'].items():
        assert (base/'native-v2'/(role+'.mec')).read_text() == generate.generate(original,role)
        for name, pin in recipe['artifacts'].items():
            assert sha(base/'native-v2'/role/name) == pin
            assert sha(base/'native-v2'/(role+'-checked')/name) == pin
        assert text_size(base/'native-v2'/role/'program.o') == recipe['text_bytes']
        for mode in ('timing','counts'):
            if mode+'_worker' in recipe:
                assert sha(base/'native-v2'/Path(recipe[mode+'_worker']).name) == recipe[mode+'_worker_sha256']
    assert sha(base/'native-v2/sort.c') == build['C_source_sha256']
    for mode,pin in build['C_objects'].items():
        assert sha(base/'native-v2'/('sort-'+mode+'.o')) == pin
    checked = 0
    for mode in ('smoke','work','timing'):
        acquisition = read(base/mode/'results.json')
        assert acquisition['complete']
        assert acquisition['plan']['build_sha256'] == sha(base/'native-v2/build.json')
        if mode != 'smoke':
            assert acquisition == summary['standalone_'+mode]
        expected = 18 if mode == 'timing' else 9
        assert len(acquisition['records']) == expected
        for row in acquisition['records']:
            case = next(c for c in vectors if c['name'] == row['case'])
            actors = acquisition['plan']['actors']
            blocks = row['blocks']
            assert len(blocks) == (24 if mode == 'timing' else 1)
            if mode == 'timing':
                assert Counter(tuple(b['order']) for b in blocks) == Counter(permutations(actors))
                assert all(v['warmup_seconds'] >= .2 and v['warmup_calls'] > 0 for v in row['warmups'].values())
            for block in blocks:
                assert set(block['observations']) == set(actors)
                for actor,value in block['observations'].items():
                    assert value['case'] == row['case'] and value['actor'] == actor
                    if mode == 'work':
                        assert len(value['work']) == len(case['vectors'])
                        for vector, work in zip(case['vectors'], value['work']):
                            assert work['frequency_reads'] == len(vector['freqs'])*(2 if actor == 'c-native' else 1)
                            assert all(work[k] >= 0 for k in ('counter_updates','key_comparisons','output_writes'))
                    else:
                        assert value['exact_sorted_keys'] and value['canaries_unchanged']
                        assert value['verified_calls'] == value['repeats']*len(case['vectors'])
                        checked += value['verified_calls']
            if mode == 'timing':
                for actor in actors:
                    raw.close(row['median_ns_per_captured_sequence'][actor],statistics.median(
                        b['observations'][actor]['batch_nanoseconds']/acquisition['plan']['repeats'] for b in blocks))
                    raw.close(row['paired_x_C_native'][actor],math.exp(statistics.mean(math.log(
                        b['observations'][actor]['batch_nanoseconds']/b['observations']['c-native']['batch_nanoseconds']) for b in blocks)))
    return checked


def source_checks(root):
    original = (root/'original-program.mec').read_text()
    recipes = read(HERE/'recipes.json')['roles']
    for key, recipe in recipes.items():
        role = recipe['local_role']
        source = original if recipe['generator_role']=='control' else generate_codec.generate(original,recipe['generator_role'])
        assert sha256(source.encode()).hexdigest() == recipe['source_sha256']
        if key != 'source-control':
            assert (HERE/('program-'+key+'.mec')).read_text() == source
        for profile in recipe['profiles']:
            assert sha(HERE/profile['file']) == profile['sha256']
        receipt = read(root/role/'receipt.json')
        assert receipt['artifacts'] == recipe['artifacts']
        assert receipt['source_sha256'] == recipe['source_sha256']
        assert text_size(root/role/'frontend/program.o') == recipe['text_bytes']
        for name,pin in recipe['artifacts'].items():
            assert sha(root/role/'frontend'/name) == pin
            assert sha(root/role/'checked-emission/frontend'/name) == pin
        raw.matcher_bytes(root/role/'frontend/program.o',receipt)
        assert sha(root/role/'bound/mech-worker') == recipe['worker_sha256']
        for mode in ('normal','checked'):
            folder = root/'source-plan/public-replays'/(key+'-'+mode)
            replay = read(folder/'reproduction.json')
            assert replay['all_three_exact'] and replay['artifacts'] == recipe['artifacts']
            assert (folder/'program.mec').read_text() == source
            for name,pin in recipe['artifacts'].items():
                assert sha(folder/'frontend'/name) == pin
        if key == 'source-control':
            reuse = read(root/'control-gates/reuse.json')
            assert reuse['same_worker'] and reuse['worker_sha256'] == recipe['worker_sha256']
            assert reuse['actual_new_control_artifacts'] == reuse['previous_control_artifacts'] == recipe['artifacts']
            gate = root/'control-gates/source-search-control-profiled'
        else:
            gate = root/role
        recovery = read(gate/'promotion-recoveries/result.json')
        assert recovery['cases'] == 6 and recovery['observations'] == recovery['exact_recoveries'] == 12
        assert recovery['object_sha256'] == recipe['artifacts']['program.o']
        assert recovery['metadata_sha256'] == recipe['artifacts']['metadata.json']
        assert recovery['benchmark_worker_sha256'] == recipe['worker_sha256']
        proof = read(gate/'linked-proof.json')
        assert proof['peak_stack_bytes'] <= 5280 and recipe['measured_peak_stack_bytes'] <= 5280
        if key == 'source-control':
            report = root/'control-gates/linked-proofs/result.json'
        else:
            parts = Path(proof['result']).parts
            start = next(i for i,p in enumerate(parts) if p.startswith('linked-input-window-current-'))
            report = root/Path(*parts[start:])
        assert sha(report) == proof['sha256']
        details = read(report)
        assert details['matches_success_observations'] == 162 and len(details['checked_failure_cases']) == 6
        actor = 'source-search-control-profiled' if key == 'source-control' else role
        assert details['peak_stack_bytes'][actor] == recipe['measured_peak_stack_bytes']
        assert max(details['peak_stack_bytes'].values()) == recipe['qualification_pair_peak_stack_bytes'] <= 5280
        if key != 'source-control':
            mapfile = root/'source-plan'/Path(proof['source_origin_mapping']).name
            assert sha(mapfile) == proof['source_origin_mapping_sha256']
            mapping = read(mapfile)
            assert mapping['candidate_source_sha256'] == recipe['source_sha256']
            failures = read(report.with_name('checked-failures.json'))['cases']
            for value in failures.values():
                before, after = value['per_actor_outcomes']
                assert before['status'] == after['status'] == 103426
                assert mapping['source_mapping'][str(before['origin'])]['site'] == after['origin']
                # These are allocation counters captured before caller Drop.
                # The owned extent must match; cumulative requested bytes can
                # include the new counter tables that have already retired.
                assert before['live_at_failure'] == after['live_at_failure']
                requests = after['requests']-before['requests']
                added = after['bytes']-before['bytes']
                assert requests >= 0 and added >= 0
                if key.startswith('hybrid-'):
                    assert added == requests*288*(4 if key=='hybrid-u32' else 8)
                else:
                    solutions = [(a,b,requests-a-b) for a in range(requests+1) for b in range(requests-a+1)
                        if added == 8*(288*a+32*b+19*(requests-a-b))]
                    assert len(solutions) == 1
            failure_source = (report.parent/'failure-support/checked-exit-failure-worker.rs').read_text()
            assert 'assert_eq!(output, [0; 6], "failed call published output")' in failure_source
            assert 'drop(heap);' in failure_source
            staged = report.parent.parent/'worker-checks'/role
            assert 'CLEANUP private_payload_bytes=0 live_owner_records=0' in (staged/'smokes.stderr').read_text()
            deltas = read(report.with_name('source-resource-deltas.json'))
            assert len(deltas) == 162
            for value in deltas:
                count = value['added_counter_allocations']
                assert count >= 0
                if key.startswith('hybrid-'):
                    width = 4 if key == 'hybrid-u32' else 8
                    assert value['added_zero_initialized_bytes'] == count*288*width
                else:
                    a,b,c = value['counts_288_32_19']
                    assert a+b+c == count and value['added_zero_initialized_bytes'] == 8*(a*288+b*32+c*19)
    return recipes


def complete_builds(root, summary):
    folder = root/'builds'
    plan, comparison = read(folder/'plan.json'),read(folder/'comparison.json')
    assert comparison['plan_sha256'] == sha(folder/'plan.json')
    assert len(plan['orders']) == len(comparison['observations']) == 14
    assert len(plan['actors']) == 7
    for position in range(7):
        assert Counter(order[position] for order in plan['orders']) == dict.fromkeys(plan['actors'],2)
    observations = []
    for index,order in enumerate(plan['orders'],1):
        record = {actor:read(folder/f'order-{index}'/actor/'result.json') for actor in order}
        assert record == comparison['observations'][index-1]
        for actor,value in record.items():
            assert value['generated_artifacts'] == plan['actors'][actor]['artifact_sha256']
            assert value['stripped_executable'] == comparison['warmup'][actor]['stripped_executable']
            assert sha(folder/f'order-{index}'/actor/'compressor') == value['stripped_executable']['sha256']
            if actor.startswith('mech-'):
                for tag,name in [('encoder','program.o'),('metadata','metadata.json'),('bytecode','program.mecb')]:
                    assert sha(folder/f'order-{index}'/actor/'frontend'/name) == value['generated_artifacts'][tag]
        observations.append(record)
    for actor,cuts in comparison['medians'].items():
        for cut,value in cuts.items():
            raw.close(value,statistics.median(r[actor]['seconds'][cut] for r in observations))
    for actor,peers in comparison['paired_ratios'].items():
        for peer,value in peers.items():
            raw.close(value,statistics.median(r[actor]['seconds']['source_through_link']/r[peer]['seconds']['source_through_link'] for r in observations))
    assert summary['builds'] == read(folder/'summary.json')
    smoke = read(folder/'deployment-smokes.json')
    assert smoke['checked_executions'] == len(smoke['records']) == 567
    for actor,pin in smoke['actors_sha256'].items():
        assert sha(folder/'warmup'/actor/'compressor') == pin
    assert all(r['independent_inflation'] and (r['actor']=='libdeflate' or r['matches_selected_bytes']) for r in smoke['records'])
    return len(observations)


def main():
    summary, manifests = read(HERE/'summary.json'),read(HERE/'archives.json')
    assert summary['archives'] == manifests and not summary['selected_default_changed']
    assert summary['C_next_milestone'] == 64 and summary['C_ultimate_target'] == 81
    with tempfile.TemporaryDirectory(prefix='mech-huffman-verify-') as temp:
        root = Path(temp)
        for name,pin in manifests.items():
            assert sha(HERE/name) == pin['sha256']
            with tarfile.open(HERE/name) as archive:
                members = archive.getmembers()
                assert {m.name for m in members} == set(pin['members'])
                for member in members:
                    assert member.isfile() and not member.name.startswith('/') and '..' not in Path(member.name).parts
                    assert sha256(archive.extractfile(member).read()).hexdigest() == pin['members'][member.name]
                archive.extractall(root,filter='data')
        micro_calls = standalone(root,summary)
        recipes = source_checks(root)
        runtime = root/'runtime'
        inventory = {c['name']:c for c in read(runtime/'fixtures-original.json')}
        oracle = {c['case']:c for c in read(runtime/'C-oracle.json')}
        portable = read(runtime/'fixtures-portable.json')
        for name,fixture in portable.items():
            values = {tag:(root/v['member']).read_bytes() for tag,v in fixture.items()}
            assert all(sha256(values[k]).hexdigest()==v['sha256'] for k,v in fixture.items())
            prefix = values['prefix']
            assert values['expected'].startswith(prefix)
            assert zlib.decompress(values['expected'][len(prefix):],-15) == values['input']
        def validate(actor,value,row):
            fixture = inventory[row['case']]
            assert value['input_bytes'] == fixture['input_bytes'] and value['prefix_bytes'] == fixture['prefix_bytes']
            assert not value['warmup'] and not value['profile']
            if actor == 'libdeflate':
                expected = oracle[row['case']]
                assert value['level'] == 12 and value['version'] == '1.26'
                assert all(value[k]==expected[k] for k in ('output_sha256','capacity','raw_bytes','matches_reference_bytes'))
            else:
                assert value['raw_bytes'] == fixture['raw_bytes'] and value['capacity'] == fixture['capacity']
        raw.VALUE_VALIDATOR = validate
        full = summary['full_qualification']
        assert full['complete'] and full['point_passed'] and not full['confidence_assessed']
        full_calls,outcomes,zeros = raw.runtime(runtime,full['runs'],81,96,full['plan']['required_peer_wins'])
        focused_calls = 0
        for key in ('full_bucket_discovery','full_hybrid_discovery'):
            value = summary[key]
            assert value['complete']
            calls,_,zero = raw.runtime(runtime,value['runs'],21,120)
            focused_calls += calls;zeros += zero
        for run in summary['full_bucket_discovery']['runs']:
            lost = [r['case'] for r in run['results'] if r['paired_x_libdeflate']['mech-control']<1<=r['paired_x_libdeflate']['mech-experiment']]
            assert lost == ['320-overlap']
        for run in summary['full_hybrid_discovery']['runs']:
            for peer in ('roc','libdeflate'):
                assert not [r['case'] for r in run['results'] if r['paired_x_'+peer]['mech-control']<1<=r['paired_x_'+peer]['mech-experiment']]
        build_orders = complete_builds(root,summary)
        receipt = {'passed':True,'captured_frequency_vectors':212,'standalone_sorted_calls_checked':micro_calls,
            'public_full_codec_replays':8,'full81_orders':outcomes,'full81_saved_calls':full_calls,
            'focused_saved_calls':focused_calls,'zero_duration_individual_samples_retained':zeros,
            'balanced_complete_build_orders':build_orders,'runnable_executable_checks':567,
            'selected_default_changed':False,'next_C_target':64,'ultimate_C_target':81}
        (HERE/'verification.json').write_text(json.dumps(receipt,indent=2)+'\n')
        print(json.dumps(receipt,indent=2))


if __name__ == '__main__':
    main()
