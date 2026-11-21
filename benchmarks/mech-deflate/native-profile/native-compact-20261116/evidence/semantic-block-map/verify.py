"""Check the saved source maps, actual native files, gates and complete builds."""
from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
import re
import statistics
import struct
import tarfile
import tempfile
import zlib
from verify_runtime import close

HERE = Path(__file__).resolve().parent
read = lambda p: json.loads(Path(p).read_text())
sha = lambda p: sha256(Path(p).read_bytes()).hexdigest()


def text(path):
    image = path.read_bytes()
    assert struct.unpack_from('<I', image)[0] == 0xFEEDFACF
    at = 32
    for _ in range(struct.unpack_from('<I', image, 16)[0]):
        kind, size = struct.unpack_from('<II', image, at)
        if kind == 0x19:
            for index in range(struct.unpack_from('<I', image, at + 64)[0]):
                section = at + 72 + 80 * index
                if image[section:section + 16].rstrip(b'\0') == b'__text':
                    address, extent, offset = struct.unpack_from('<QQI', image, section + 32)
                    return address, image[offset:offset + extent]
        at += size
    raise AssertionError('missing native text')


def kernel(path, start, extent):
    address, payload = text(path)
    return payload[start - address:start - address + extent]


def native(root):
    recipes = read(HERE / 'recipes.json')['roles']
    selected = read(HERE.parent / 'encoder-size-gap/packet-family-root-study/recipes.json')['roles']['combined']
    for name, recipe in recipes.items():
        assert sha(HERE / recipe['source_file']) == recipe['source_sha256']
        receipt = read(root / 'probe' / name / 'receipt.json')
        assert receipt['artifacts'] == recipe['artifacts']
        assert len(text(root / 'probe' / name / 'frontend/program.o')[1]) == recipe['text_bytes']
        for file, pin in recipe['artifacts'].items():
            assert sha(root / 'probe' / name / 'frontend' / file) == pin
        assert sha(root / 'probe' / name / 'bound/mech-worker') == recipe['worker_sha256']
        if name.startswith('selected-'):
            assert recipe['artifacts'] == selected['artifacts']
            assert recipe['worker_sha256'] == selected['worker_sha256']
        for profile in recipe['profiles']:
            assert sha(HERE / profile['file']) == profile['sha256']
        for mode in ['normal', 'checked']:
            folder = root / 'public-replays' / (name + '-' + mode)
            replay = read(folder / 'reproduction.json')
            assert replay['all_three_exact'] and replay['artifacts'] == recipe['artifacts']
            assert sha(folder / 'program.mec') == recipe['source_sha256']
            for file, pin in recipe['artifacts'].items():
                assert sha(folder / 'frontend' / file) == pin
    a, b = recipes['raw-trace-off'], recipes['raw-trace-mapped']
    assert a['artifacts'] == b['artifacts'] and a['worker_sha256'] == b['worker_sha256']
    for name in recipes:
        folder = root / 'probe' / name / 'bound'
        smokes = read(folder / 'smokes.json')
        assert len(smokes) == 162
        for value in smokes:
            assert all(value[k] for k in ['exact_reference', 'independent_inflation',
                                         'input_alias_unchanged', 'borrowed_output_alias_unchanged'])
            assert value['nonreturned_private_bytes_at_return'] == 0
    return 8


def gates(root):
    role = root / 'probe/raw-trace-mapped'
    gate = root / 'probe/raw-trace-mapped-gates'
    result = gate / 'linked-proofs/result.json'
    proof = read(role / 'linked-proof.json')
    assert sha(result) == proof['sha256']
    details = read(result)
    assert details['matches_success_observations'] == 162
    assert len(details['checked_failure_cases']) == 6
    stack = read(result.with_name('stack.json'))
    peaks = {Path(actor).name: max(v for sample in cases.values() for v in sample.values())
             for actor, cases in stack.items()}
    assert peaks == details['peak_stack_bytes'] and max(peaks.values()) <= 5280
    assert peaks['candidate'] == proof['candidate_peak_stack_bytes'] == 4752
    workers = gate / 'worker-checks'
    assert read(workers / 'maintained/smokes.json') == read(workers / 'candidate/smokes.json')
    for actor in ['maintained', 'candidate']:
        assert 'CLEANUP private_payload_bytes=0 live_owner_records=0' in (workers / actor / 'smokes.stderr').read_text()
    mapping = read(gate / 'source-origin-map.json')['source_mapping']
    failures = read(result.with_name('checked-failures.json'))['cases']
    for value in failures.values():
        a, b = value['per_actor_outcomes']
        assert a['status'] == b['status'] == 103426
        assert mapping[str(a['origin'])]['site'] == b['origin']
        assert a['live_at_failure'] == b['live_at_failure']
        for index, samples in enumerate(value['raw']):
            assert all({k: v for k, v in sample.items() if k != 'seconds'} == (a, b)[index] for sample in samples)
    fault = role / 'promotion-recoveries'
    report = read(fault / 'result.json')
    assert report['cases'] == 6 and report['observations'] == report['exact_recoveries'] == 12
    assert report['benchmark_worker_sha256'] == sha(role / 'bound/mech-worker')
    policy = read(gate / 'recovery-packet/qualification-policy-v3.json')['checked_failures']
    failed, recovered = read(fault / 'checked-failures.json'), read(fault / 'checked-recoveries.json')
    assert len(failed) == len(recovered) == 12
    assert Counter(v['case'] for v in failed) == dict.fromkeys(policy, 2)
    for failure, success in zip(failed, recovered):
        assert (failure['status'], failure['origin']) == (policy[failure['case']]['status'], policy[failure['case']]['origin'])
        assert success['case'] == failure['case'] and success['exact_reference'] and success['independent_inflation']
        assert success['input_alias_unchanged'] and success['borrowed_output_alias_unchanged']
        assert success['nonreturned_private_bytes_at_return'] == 0


def mapping(root):
    probe = root / 'probe'
    folder = probe / 'raw-trace-mapped'
    proof = read(probe / 'mapping-proof.json')
    sidecar = folder / '475-vector.json'
    assert sha(sidecar) == proof['source_map_sha256'] == sha(probe / 'selected-mapped/475-vector.json')
    vcode = folder / 'mech_compact_numeric_475_vector.vcode'
    assert sha(vcode) == sha(probe / 'raw-trace-off' / vcode.name)
    labels = [{'vcode_block': int(index), 'offset': int(offset), 'ir_block': int(block) if block else None}
              for index, offset, block in re.findall(r'^;; REGALLOC_BLOCK (\d+) offset=(\d+) ir=(?:Some\(block(\d+)\)|None) successors=', vcode.read_text(), re.M)]
    assert labels == proof['labels']
    by_ir = {}
    for value in labels:
        if value['ir_block'] is not None:
            by_ir.setdefault(value['ir_block'], []).append(value)
    records = [row | {'machine_labels': by_ir.get(row['ir_block'], [])} for row in read(sidecar)['emitted_sites']]
    assert records == proof['records']
    counts = read(folder / 'counts-475/counts.json')
    canonical_path = folder / 'counts-475/canonical-counts.json'
    canonical = read(canonical_path)
    assert sha(canonical_path) == counts['canonical_counts_sha256']
    assert sha(folder / 'frontend/program.o') == counts['object_sha256'] == proof['raw_object_sha256']
    canonical_object = folder / 'canonical-counter-input/frontend/program.o'
    assert sha(canonical_object) == canonical['object_sha256']
    atom = read(folder / 'receipt.json')['layout']['symbols'][counts['symbol']]
    body = kernel(folder / 'frontend/program.o', atom['start'], atom['extent'])
    assert sha256(body).hexdigest() == atom['sha256']
    assert body == kernel(canonical_object, atom['start'] - counts['address_translation'], atom['extent'])
    plan = read(folder / 'counts-475/plan.json')
    assert plan['source_worker_sha256'] == plan['canonical_worker_sha256'] == sha(folder / 'bound/mech-worker') == sha(folder / 'canonical-counter-input/bound/mech-worker')
    translated = json.loads(json.dumps(canonical))
    for case in translated['cases'].values():
        for block in case['blocks']:
            block['address'] = hex(int(block['address'], 16) + counts['address_translation'])
    assert translated['cases'] == counts['cases']
    # Only exact machine-head anchors are used for the logical tree comparison.
    # Merged labels and missing sites are not counted as independent events.
    anchors = {914: 1568, 1426: 648, 510: 4612}
    for site, offset in anchors.items():
        rows = [r for r in records if r['source_site'] == site and r['version_initialize'] is None]
        assert len(rows) == 1 and len(rows[0]['machine_labels']) == 1
        assert rows[0]['machine_labels'][0]['offset'] == offset
        assert offset == 0 or is_branch(struct.unpack_from('<I', body, offset - 4)[0]) or branch_targeted(body, offset)
    rows = []
    saved = read(probe / 'tree-work-comparison.json')
    for row in saved['cases']:
        case = row['case']
        visits = {int(b['address'], 16) - atom['start']: b['visits'] for b in counts['cases'][case]['blocks']}
        peer = read(root / 'C-traces' / (case + '.json'))['counters']
        actual = {'case': case, 'Mech_private_search_tree_anchor': visits.get(1568, 0),
                  'C_search_tree_visits': peer['search_tree_visits'],
                  'Mech_private_skip_tree_anchor': visits.get(648, 0),
                  'C_skip_tree_visits': peer['skip_tree_visits'], 'collection10_handoffs': visits.get(4612, 0)}
        actual['search_scope_difference'] = actual['C_search_tree_visits'] - actual['Mech_private_search_tree_anchor']
        actual['exact_search_and_skip_anchor_counts'] = (actual['search_scope_difference'] == 0 and actual['Mech_private_skip_tree_anchor'] == actual['C_skip_tree_visits'])
        assert actual == row
        rows.append(actual)
    assert sum(r['exact_search_and_skip_anchor_counts'] for r in rows) == saved['matched_cases'] == 7
    with tarfile.open(HERE.parent / 'current-phase-work-study/c-semantic-traces.tar.gz') as archive:
        for row in rows:
            name = row['case'] + '.json'
            assert archive.extractfile('libdeflate/' + name).read() == (root / 'C-traces' / name).read_bytes()
    return len(labels), len(records), len(rows)


def is_branch(word):
    return (word & 0x7c000000 == 0x14000000 or word & 0xff000010 == 0x54000000
            or word & 0x7e000000 in (0x34000000, 0x36000000)
            or word & 0xfffffc1f in (0xd61f0000, 0xd65f0000) or word == 0xd65f0fff)


def branch_targeted(body, offset):
    for at in range(0, len(body), 4):
        word = struct.unpack_from('<I', body, at)[0]
        bits = 26 if word & 0x7c000000 == 0x14000000 else 19 if (word & 0xff000010 == 0x54000000 or word & 0x7e000000 == 0x34000000) else 14 if word & 0x7e000000 == 0x36000000 else 0
        if bits:
            immediate = (word if bits == 26 else word >> 5) & ((1 << bits) - 1)
            if immediate & (1 << (bits - 1)):
                immediate -= 1 << bits
            if at + immediate * 4 == offset:
                return True
    return False


def builds(root):
    folder = root / 'matched-builds'
    plan, comparison = read(folder / 'plan.json'), read(folder / 'comparison.json')
    assert comparison['plan_sha256'] == sha(folder / 'plan.json')
    assert len(plan['actors']) == 4 and len(plan['orders']) == len(comparison['observations']) == 8
    for position in range(4):
        assert Counter(order[position] for order in plan['orders']) == dict.fromkeys(plan['actors'], 2)
    for index, order in enumerate(plan['orders'], 1):
        record = {actor: read(folder / f'order-{index}' / actor / 'result.json') for actor in order}
        assert record == comparison['observations'][index - 1]
        for actor, value in record.items():
            assert value['generated_artifacts'] == plan['actors'][actor]['artifact_sha256']
            assert value['stripped_executable'] == comparison['warmup'][actor]['stripped_executable']
            assert sha(folder / f'order-{index}' / actor / 'compressor') == value['stripped_executable']['sha256']
            if actor.startswith('mech-'):
                for tag, name in [('encoder', 'program.o'), ('metadata', 'metadata.json'), ('bytecode', 'program.mecb')]:
                    assert sha(folder / f'order-{index}' / actor / 'frontend' / name) == value['generated_artifacts'][tag]
    observations = comparison['observations']
    for actor, cuts in comparison['medians'].items():
        for cut, value in cuts.items():
            close(value, statistics.median(r[actor]['seconds'][cut] for r in observations))
    for actor, peers in comparison['paired_ratios'].items():
        for peer, value in peers.items():
            close(value, statistics.median(r[actor]['seconds']['source_through_link'] / r[peer]['seconds']['source_through_link'] for r in observations))
    bindings = read(folder / 'compiler-bindings.json')
    assert bindings['raw_plan_sha256'] == sha(folder / 'plan.json')
    for value in bindings['records']:
        build = read(root / value['build_manifest'])
        assert sha(root / value['build_manifest']) == value['build_manifest_sha256']
        assert build['compiler_sha256'] == value['compiler_sha256'] == plan['actors'][value['actor']]['compiler_sha256']
        assert build['source_base'] == value['source_base']
        assert plan['actors'][value['actor']]['compile'][0] == build['compiler']
        for name, pin in build['source_sha256'].items():
            assert sha(root / value['source_snapshot'] / name) == pin
    smoke = read(folder / 'deployment-smokes.json')
    assert smoke['checked_executions'] == len(smoke['records']) == 324
    fixtures = read(root / 'fixtures-portable.json')
    for actor, pin in smoke['actors_sha256'].items():
        assert sha(folder / 'warmup' / actor / 'compressor') == pin
    for value in smoke['records']:
        data = {tag: (root / path).read_bytes() for tag, path in fixtures[value['case']].items()}
        payload = (folder / value['output_file']).read_bytes()
        assert sha256(payload).hexdigest() == value['output_sha256'] and len(payload) == value['output_bytes']
        assert sha256(data['input']).hexdigest() == value['input_sha256']
        assert zlib.decompress(payload, -15) == data['input'] and value['independent_inflation']
        if value['actor'] != 'libdeflate':
            assert value['matches_selected_bytes'] and payload == data['expected'][len(data['prefix']):]


def main():
    manifest = read(HERE / 'manifest.json')
    assert sha(HERE / 'observations.tar.gz') == manifest['archive_sha256']
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        with tarfile.open(HERE / 'observations.tar.gz') as archive:
            assert all(not Path(m.name).is_absolute() and '..' not in Path(m.name).parts and m.isfile() for m in archive.getmembers())
            archive.extractall(root, filter='data')
        assert {str(p.relative_to(root)): sha(p) for p in root.rglob('*') if p.is_file()} == manifest['members']
        replays = native(root)
        gates(root)
        labels, records, cases = mapping(root)
        builds(root)
        tests = read(root / 'repository-tests/result.json')
        assert tests['exit_code'] == 0
        assert 'test result: ok. 79 passed; 0 failed;' in (root / 'repository-tests/stdout.log').read_text()
        result = {'passed': True, 'source_normal_checked_replays': replays, 'vcode_labels': labels,
                  'emitted_source_sites': records, 'tree_cases': cases, 'exact_private_tree_cases': 7,
                  'native_successes_per_probe': 162, 'checked_error_cases': 6, 'recoveries': 12,
                  'scalar_tests': 79, 'complete_build_orders': 8, 'deployment_executions': 324,
                  'selected_bytes_unchanged': True, 'selected_score': {'Roc': 81, 'libdeflate': 63},
                  'new_C_win': False}
        (HERE / 'verification.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
