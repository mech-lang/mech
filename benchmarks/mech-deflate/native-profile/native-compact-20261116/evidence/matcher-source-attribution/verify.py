"""Verify the saved diagnostic source map, exact controls and native checks."""
from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
import tarfile
import tempfile

HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(Path(p).read_text())
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()


def main():
    summary=read(HERE/'summary.json');pin=summary['archive']
    assert sha(HERE/pin['file'])==pin['sha256']
    assert sha(HERE/'diagnostic-source.patch')==summary['diagnostic_patch_sha256']
    with tempfile.TemporaryDirectory(prefix='mech-source-attribution-') as temp:
        root=Path(temp)
        with tarfile.open(HERE/pin['file']) as archive:
            members=archive.getmembers();assert {m.name for m in members}==set(pin['members'])
            for member in members:
                assert member.isfile() and not member.name.startswith('/') and '..' not in Path(member.name).parts
                assert sha256(archive.extractfile(member).read()).hexdigest()==pin['members'][member.name]
            archive.extractall(root,filter='data')
        study=root/'study'
        for role in ['source-sites-off','selected-default-off']:
            receipt=read(study/role/'receipt.json')
            assert receipt['all_three_exact'] and receipt['artifacts']==receipt['reference_artifacts']
            for name,value in receipt['artifacts'].items():assert sha(study/role/'frontend'/name)==value
        image=study/'source-sites-on-unprofiled-vcode';receipt=read(image/'receipt.json')
        assert receipt['source_bytecode_identical'] and not receipt['native_artifacts_identical_to_profiled_control']
        checked=read(study/'source-sites-checked/receipt.json')
        assert checked['normal_checked_all_three_exact'] and checked['artifacts']==receipt['artifacts']
        for name,value in receipt['artifacts'].items():
            assert sha(image/'frontend'/name)==sha(study/'source-sites-checked/frontend'/name)==value
        meta=read(image/'frontend/metadata.json');phase=next(p['vector_variant'] for p in meta['numeric_phases'] if p['header']==475)
        ranges=read(image/'mech_compact_numeric_475_vector.source-ranges.json')
        source=read(image/'canonical-sites/source-475-vector-true-no-growth-false.json');sites={v['site'] for v in source}
        assert len(ranges)==752 and phase['code_bytes']==6232
        assert all(0<=v['start']<v['end']<=6232 and v['start']%4==v['end']%4==0 for v in ranges)
        assert all(v['end']<=w['start'] for v,w in zip(ranges,ranges[1:]))
        assert all(v['site'] in sites or v['site']==4294967295 for v in ranges)
        assert sum(v['end']-v['start'] for v in ranges if v['site'] in sites)==4020
        gate=study/'native-gates';result=read(gate/'linked-proofs/result.json')
        assert result['matches_success_observations']==162 and len(result['checked_failure_cases'])==6
        workers=gate/'worker-checks';a,b=[read(workers/actor/'smokes.json') for actor in ['maintained','candidate']]
        assert a==b and len(b)==162
        assert all(v['exact_reference'] and v['independent_inflation'] and v['input_alias_unchanged'] and v['borrowed_output_alias_unchanged'] and v['nonreturned_private_bytes_at_return']==0 for v in b)
        for actor in ['maintained','candidate']:
            assert 'CLEANUP private_payload_bytes=0 live_owner_records=0' in (workers/actor/'smokes.stderr').read_text()
        policy=read(gate/'qualified-source-policy.json')['checked_failures']
        failures=read(gate/'linked-proofs/checked-failures.json')['cases']
        for name,value in failures.items():
            assert (value['outcome']['status'],value['outcome']['origin'])==(policy[name]['status'],policy[name]['origin'])
            assert all({k:v for k,v in row.items() if k!='seconds'}==value['outcome'] for samples in value['raw'] for row in samples)
        stack=read(gate/'linked-proofs/stack.json')
        peaks={Path(actor).name:max(v for samples in cases.values() for v in samples.values()) for actor,cases in stack.items()}
        assert peaks==result['peak_stack_bytes'] and peaks['candidate']==4704 and max(peaks.values())<=5280
        recovery=image/'promotion-recoveries';report=read(recovery/'result.json')
        assert report['cases']==6 and report['observations']==report['exact_recoveries']==12
        failed,recovered=read(recovery/'checked-failures.json'),read(recovery/'checked-recoveries.json')
        assert len(failed)==len(recovered)==12 and Counter(v['case'] for v in failed)==dict.fromkeys(policy,2)
        for failure,success in zip(failed,recovered):
            assert (failure['status'],failure['origin'])==(policy[failure['case']]['status'],policy[failure['case']]['origin'])
            assert success['exact_reference'] and success['independent_inflation'] and success['input_alias_unchanged'] and success['borrowed_output_alias_unchanged'] and success['nonreturned_private_bytes_at_return']==0
        bulk=read(root/'bulk/counts.json')['cases']
        large=bulk['prefix-random-1048576'];assert large['calls']==31
        assert large['counts']['instructions']==7365104 and large['counts']['uqsub.8h']*8==31*262144
        assert bulk['prefix-random-65536']['calls']==1
        assert all(v['calls']==0 for k,v in bulk.items() if k not in ['prefix-random-65536','prefix-random-1048576'])
        result={'passed':True,'exact_control_recipes':2,'diagnostic_native_successes':162,'checked_errors':6,'injected_failures_and_recoveries':12,'normal_checked_all_three_exact':True,'canonical_source_ranges':752,'known_static_source_bytes':4020,'diagnostic_code_bytes':6232,'bulk_large_calls':31,'selected_default_changed':False,'new_qualified_C_win':False}
        (HERE/'verification.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))


if __name__=='__main__':main()
