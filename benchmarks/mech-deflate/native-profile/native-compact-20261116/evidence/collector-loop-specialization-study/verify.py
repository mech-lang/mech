"""Portable checks of source, native files, every saved call and complete builds."""
from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
import statistics
import struct
import tarfile
import tempfile
import zlib
from generate import generate
import verify_runtime as raw

HERE = Path(__file__).resolve().parent
read = lambda p:json.loads(Path(p).read_text())
sha = lambda p:sha256(Path(p).read_bytes()).hexdigest()


def text_bytes(path):
    data = path.read_bytes()
    assert struct.unpack_from('<I',data)[0] == 0xFEEDFACF
    at = 32
    for _ in range(struct.unpack_from('<I',data,16)[0]):
        kind, size = struct.unpack_from('<II',data,at)
        if kind == 0x19:
            for index in range(struct.unpack_from('<I',data,at+64)[0]):
                section = at+72+80*index
                if data[section:section+16].rstrip(b'\0') == b'__text':
                    return struct.unpack_from('<Q',data,section+40)[0]
        at += size
    raise AssertionError('missing native text')


def native(root):
    roles = read(HERE/'recipes.json')['roles']
    for name, recipe in roles.items():
        source = generate((HERE/recipe['base_source']).read_text(), recipe['generator_mode'])
        assert sha256(source.encode()).hexdigest() == recipe['source_sha256']
        assert source == (HERE/recipe['source_file']).read_text()
        role = root/'study'/recipe['local_role']
        receipt = read(role/'receipt.json')
        assert receipt['artifacts'] == recipe['artifacts']
        assert receipt['source_sha256'] == recipe['source_sha256']
        assert text_bytes(role/'frontend/program.o') == recipe['text_bytes']
        for file, pin in recipe['artifacts'].items():
            assert sha(role/'frontend'/file) == sha(role/'checked-emission/frontend'/file) == pin
        assert sha(role/'bound/mech-worker') == recipe['worker_sha256']
        for profile in recipe['profiles']:
            assert sha(HERE/profile['file']) == profile['sha256']
        for mode in ['normal','checked']:
            folder = root/'study/public-replays'/(name+'-'+mode)
            replay = read(folder/'reproduction.json')
            assert replay['all_three_exact'] and replay['artifacts'] == recipe['artifacts']
            assert (folder/'program.mec').read_text() == source
            for file, pin in recipe['artifacts'].items():
                assert sha(folder/'frontend'/file) == pin
        gate = root/'study'/(recipe['local_role']+'-gates')
        proof = read(role/'linked-proof.json')
        result = gate/'linked-proofs/result.json'
        assert sha(result) == proof['sha256']
        details = read(result)
        assert details['matches_success_observations'] == 162 and len(details['checked_failure_cases']) == 6
        stack = read(result.with_name('stack.json'))
        peaks = {Path(actor).name:max(v for sample in cases.values() for v in sample.values())
                 for actor, cases in stack.items()}
        assert peaks == details['peak_stack_bytes']
        assert peaks['candidate'] == recipe['measured_peak_stack_bytes'] == proof['candidate_peak_stack_bytes']
        assert max(peaks.values()) <= 5280
        workers = gate/'worker-checks'
        before, after = [read(workers/actor/'smokes.json') for actor in ['maintained','candidate']]
        assert len(before) == len(after) == 162
        for actor in ['maintained','candidate']:
            assert 'CLEANUP private_payload_bytes=0 live_owner_records=0' in (workers/actor/'smokes.stderr').read_text()
        for a, b in zip(before, after):
            fields = {'payload_allocation_requests','requested_payload_bytes','zeroed_bytes'} if name.startswith('hybrid-') else set()
            assert {k:v for k,v in a.items() if k not in fields} == {k:v for k,v in b.items() if k not in fields}
            if fields:
                requests = b['payload_allocation_requests']-a['payload_allocation_requests']
                assert requests >= 0
                assert b['requested_payload_bytes']-a['requested_payload_bytes'] == b['zeroed_bytes']-a['zeroed_bytes'] == requests*288*4
            assert b['exact_reference'] and b['independent_inflation'] and b['input_alias_unchanged'] and b['borrowed_output_alias_unchanged']
            assert b['nonreturned_private_bytes_at_return'] == 0
        mapping = read(gate/'source-origin-map.json')
        origins = [read(workers/actor/'metadata.json')['origins'] for actor in ['maintained','candidate']]
        for old, value in mapping['source_mapping'].items():
            a, b = origins[0][int(old)], origins[1][value['site']]
            assert (a['node'],a['local']) == (b['node'],b['local'])
        failures = read(result.with_name('checked-failures.json'))['cases']
        for value in failures.values():
            a, b = value['per_actor_outcomes']
            assert a['status'] == b['status'] == 103426
            assert mapping['source_mapping'][str(a['origin'])]['site'] == b['origin']
            assert a['live_at_failure'] == b['live_at_failure']
            for actor, samples in enumerate(value['raw']):
                assert all({k:v for k,v in sample.items() if k!='seconds'} == (a,b)[actor] for sample in samples)
        fault = role/'promotion-recoveries'
        report = read(fault/'result.json')
        assert report['cases'] == 6 and report['observations'] == report['exact_recoveries'] == 12
        assert report['benchmark_worker_sha256'] == recipe['worker_sha256']
        policy = read(gate/'recovery-packet/qualification-policy-v3.json')['checked_failures']
        failed, recovered = read(fault/'checked-failures.json'),read(fault/'checked-recoveries.json')
        assert len(failed) == len(recovered) == 12
        assert Counter(v['case'] for v in failed) == dict.fromkeys(policy,2)
        for failure, success in zip(failed,recovered):
            assert (failure['status'],failure['origin']) == (policy[failure['case']]['status'],policy[failure['case']]['origin'])
            assert success['case'] == failure['case'] and success['exact_reference'] and success['independent_inflation']
            assert success['input_alias_unchanged'] and success['borrowed_output_alias_unchanged']
            assert success['nonreturned_private_bytes_at_return'] == 0
    return len(roles)*2


def complete_builds(root, summary):
    folder = root/'builds'
    plan, comparison = read(folder/'plan.json'),read(folder/'comparison.json')
    assert comparison['plan_sha256'] == sha(folder/'plan.json')
    actors, orders = len(plan['actors']),len(plan['orders'])
    assert actors == 6 and orders == len(comparison['observations']) == 12
    for position in range(actors):
        assert Counter(order[position] for order in plan['orders']) == dict.fromkeys(plan['actors'],2)
    observations = []
    for index, order in enumerate(plan['orders'],1):
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
    assert smoke['checked_executions'] == len(smoke['records']) == actors*81
    for actor,pin in smoke['actors_sha256'].items():
        assert sha(folder/'warmup'/actor/'compressor') == pin
    assert all(r['independent_inflation'] and (r['actor']=='libdeflate' or r['matches_selected_bytes']) for r in smoke['records'])
    return orders, smoke['checked_executions']


def main():
    summary, manifests = read(HERE/'summary.json'),read(HERE/'archives.json')
    assert summary['archives'] == manifests and not summary['selected_default_changed']
    assert not summary['candidate_promoted'] and not summary['candidate_full81_qualified']
    with tempfile.TemporaryDirectory(prefix='mech-collector-verify-') as temp:
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
        replays = native(root)
        runtime = root/'runtime'
        inventory = {c['name']:c for c in read(runtime/'fixtures-original.json')}
        oracle = {c['case']:c for c in read(runtime/'C-oracle.json')}
        for fixture in read(runtime/'fixtures-portable.json').values():
            values = {tag:(root/v['member']).read_bytes() for tag,v in fixture.items()}
            assert all(sha256(values[k]).hexdigest()==v['sha256'] for k,v in fixture.items())
            assert values['expected'].startswith(values['prefix'])
            assert zlib.decompress(values['expected'][len(values['prefix']):],-15)==values['input']
        def validate(actor,value,row):
            fixture = inventory[row['case']]
            assert value['input_bytes']==fixture['input_bytes'] and value['prefix_bytes']==fixture['prefix_bytes']
            assert not value['warmup'] and not value['profile']
            if actor=='libdeflate':
                expected = oracle[row['case']]
                assert value['level']==12 and value['version']=='1.26'
                assert all(value[k]==expected[k] for k in ['output_sha256','capacity','raw_bytes','matches_reference_bytes'])
            else:
                assert value['raw_bytes']==fixture['raw_bytes'] and value['capacity']==fixture['capacity']
        raw.VALUE_VALIDATOR = validate
        calls = zeros = 0
        for key in ['initial_split','projection_repair']:
            campaign = summary[key]
            assert campaign['complete']
            count,_,zero = raw.runtime(runtime,campaign['runs'],21,24)
            calls += count; zeros += zero
        for run in summary['projection_repair']['runs']:
            lost = [r['case'] for r in run['results'] if r['paired_x_libdeflate']['mech-control']<1<=r['paired_x_libdeflate']['mech-experiment']]
            assert lost == ['320-overlap']
        orders, executions = complete_builds(root,summary)
        result = {'passed':True,'public_native_replays':replays,'all_saved_calls_checked':calls,
                  'zero_duration_individual_samples_retained':zeros,'balanced_complete_build_orders':orders,
                  'deployment_executions':executions,'selected_default_changed':False,
                  'new_qualified_C_win':False,'next_C_target':64,'ultimate_C_target':81}
        (HERE/'verification.json').write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps(result,indent=2))


if __name__ == '__main__':
    main()
