from collections import Counter
from hashlib import file_digest
from itertools import permutations
from pathlib import Path
from statistics import geometric_mean, median, quantiles
import gzip
import json
import sys

q = Path(__file__).resolve().parent
modes = ('fourteen', 'five', 'four', 'disabled')
labels = (
    *(f'controller-{variant}' for variant in modes),
    *(f'controller-packed-{variant}' for variant in modes),
    *(f'controller-passthrough-{variant}' for variant in modes),
    'controller-passthrough-combined-small', 'controller-passthrough-combined-mixed',
)
gpus = ('mech-control','mech-experiment')
actors = gpus + ('roc','libdeflate')
order_set = set(permutations(actors))
counters = ('payload_allocation_requests', 'requested_payload_bytes', 'initialized_bytes_copied',
            'zeroed_bytes', 'peak_private_payload_bytes', 'nonreturned_private_bytes_at_return')
oracle = {r['name']: r for r in json.loads((q/'fixtures.json').read_text())}
assert len(oracle) == 81


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def packed_variant(label):
    d = q / label
    report = json.loads((d/'receipt.json').read_text())
    assert report['object_sha256'] == sha(d/'program.o')
    assert report['metadata_sha256'] == sha(d/'metadata.json')
    if report['count']:
        verified = json.loads((q/(label+'-qualification.json')).read_text())
        assert verified['verified'] and verified['complete_smokes'] == 162
        assert report['worker_sha256'] == verified['worker_sha256'] == sha(d/'mech-worker')
    else:
        assert report['control_object_match'] and report['control_metadata_match']
    return report


def compare(label, *, allow_source_counter_changes=False):
    base = q/label
    complete = json.loads((base/'complete.json').read_text())
    rows = json.loads((base/'summary.json').read_text())
    pins = json.loads((base/'pins.json').read_text())
    assert complete['cases']==len(rows)
    if len(rows)==81:
        assert {r['case'] for r in rows}==set(oracle)
    assert sha(q/'fixtures.json')==pins['input_inventory_sha256']
    for actor in actors:
        command=pins['actors'][actor]['command']
        assert sha(command[0])==pins['actors'][actor]['binary_sha256'], actor
        if actor in gpus:
            assert sha(command[1])==pins['actors'][actor]['metadata_sha256']
    per_case = {}
    examples = 0
    for row in rows:
        case = row['case']
        fixtures = oracle[case]
        blocks = json.loads(gzip.decompress((base/case/'observations.json.gz').read_bytes()))
        assert len(blocks)==complete['balanced_blocks_per_case']
        orders=Counter(tuple(block['order']) for block in blocks)
        assert orders.keys()==order_set and len(set(orders.values()))==1, case
        counters_by_actor={actor:Counter() for actor in gpus}
        samples = {actor:[] for actor in actors}
        block_pairs = {actor:[] for actor in actors}
        for block in blocks:
            assert set(block['observations'])==set(actors)
            assert len({len(t) for t in block['observations'].values()})==1
            for actor in actors:
                calls=block['observations'][actor]
                for v in calls:
                    assert not v['warmup'] and not v['profile'] and v['case']==case
                    for key in ('input_bytes','prefix_bytes'):
                        assert v[key]==fixtures[key]
                    for key in ('independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged'):
                        assert v[key], (case,actor,key)
                    if actor=='libdeflate':
                        assert v['capacity']==row['libdeflate_capacity']
                        assert v['raw_bytes']==row['libdeflate_raw_bytes']
                        assert v['output_sha256']==row['raw_stream_sha256']
                        assert v['level']==12 and v['version']=='1.26'
                    else:
                        assert v['capacity']==fixtures['capacity']
                        assert v['raw_bytes']==fixtures['raw_bytes'] and v['exact_reference']
                    if actor in gpus:
                        counters_by_actor[actor][tuple(v[k] for k in counters)]+=1
                samples[actor].extend(v['seconds'] for v in calls)
                block_pairs[actor].append(block['means'][actor])
                examples+=len(calls)
        left,right=(counters_by_actor[a] for a in gpus)
        assert left and right and all(t[-1]==0 for actor in (left, right) for t in actor)
        if not allow_source_counter_changes:
            assert left==right
        relative = lambda actor, ref: geometric_mean(x/y for x,y in zip(block_pairs[actor],block_pairs[ref],strict=True))
        roc = [relative(actor,'roc') for actor in gpus]
        lib = [relative(actor,'libdeflate') for actor in gpus]
        groups = [blocks[i:i+24] for i in range(0,len(blocks),24)]
        assert all({tuple(block['order']) for block in group}==order_set for group in groups)
        per_case[case]={
            'candidate_over_control':relative(gpus[1],gpus[0]),
            'candidate_over_control_per_24_orders':[
                geometric_mean(b['means'][gpus[1]]/b['means'][gpus[0]] for b in group)
                for group in groups
            ],
            'mech_over_roc':roc,'mech_over_libdeflate':lib,
            'pooled_105':median(samples[gpus[1]])<=1.05*median(samples[gpus[0]]),
            'block_105':median(block_pairs[gpus[1]])<=1.05*median(block_pairs[gpus[0]]),
        }
        if allow_source_counter_changes:
            per_case[case]['allocation_profiles'] = {
                actor: [dict(zip(counters, profile), calls=count)
                        for profile, count in counter.items()]
                for actor, counter in zip(gpus, (left, right))
            }
    assert examples==complete['timed_calls']
    summary = {
        'roc_wins':[sum(case['mech_over_roc'][i]<1 for case in per_case.values()) for i in (0,1)],
        'libdeflate_wins':[sum(case['mech_over_libdeflate'][i]<1 for case in per_case.values()) for i in (0,1)],
        'candidate_over_control_geometric_mean':geometric_mean(v['candidate_over_control'] for v in per_case.values()),
        'all_105_gates':all(v['pooled_105'] and v['block_105'] for v in per_case.values()),
        'verified_full_calls':examples,
    }
    for field in ('mech_over_roc','mech_over_libdeflate'):
        summary['lost_cases_'+field]=[name for name,v in per_case.items() if v[field][0]<1<=v[field][1]]
        summary['gained_cases_'+field]=[name for name,v in per_case.items() if v[field][1]<1<=v[field][0]]
    print(label,summary,flush=True)
    return dict(summary,pins=pins,per_case=per_case)


def main():
    record = {
        'native_outline_revision':'779f46e335',
        'machine':'Apple M1',
        'roc_object_text_bytes':57696,
        'control_object_text_bytes':658180,
        'variants':{label: packed_variant(label) for label in labels},
        'runtime':{label: compare(label) for label in sys.argv[1:]},
    }
    stack=q/'controller-stack-touched.json'
    if stack.exists():
        record['measured_stack_bytes_touched']=json.loads(stack.read_text())
    compiled=q/'controller-compile-work'/'results.json'
    if compiled.exists():
        record['matched_decode_to_object']=json.loads(compiled.read_text())
    scope={'controller-small-affected','controller-small-repeatability'}
    if scope<=record['runtime'].keys():
        comparison=record['runtime']['controller-small-affected']['per_case']
        same=record['runtime']['controller-small-repeatability']
        assert same['pins']['actors'][gpus[0]]['binary_sha256']==same['pins']['actors'][gpus[1]]['binary_sha256']
        assert same['pins']['actors'][gpus[0]]['metadata_sha256']==same['pins']['actors'][gpus[1]]['metadata_sha256']
        same=same['per_case']
        assert comparison.keys()==same.keys()
        noise=quantiles([abs(v['candidate_over_control']-1) for v in same.values()],n=20,method='inclusive')[-1]
        details={}
        for case,x in comparison.items():
            aa=abs(same[case]['candidate_over_control']-1)
            details[case]={
                'candidate_over_control':x['candidate_over_control'],
                'same_build_over_control':same[case]['candidate_over_control'],
                'descriptive_tolerance':max(aa,noise),
                'adverse_groups':sum(value>1 for value in x['candidate_over_control_per_24_orders']),
            }
        record['targeted_repeatability']={'same_build_p95_absolute_difference':noise,'cases':details}
        print('targeted repeatability',record['targeted_repeatability'])
    (q/'checked-controller-results.json').write_text(json.dumps(record,indent=2)+'\n')


if __name__ == '__main__':
    main()
