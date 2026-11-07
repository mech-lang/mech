from pathlib import Path
import gzip, hashlib, json, statistics
P = Path(__file__).resolve().parent
manifest = json.loads((P/'manifest.json').read_text())
for name, info in manifest['files'].items():
    blob=(P/name).read_bytes()
    assert len(blob)==info['bytes'] and hashlib.sha256(blob).hexdigest()==info['sha256'], name
    raw=gzip.decompress(blob) if name.endswith('.gz') else blob
    assert len(raw)==info['decoded_bytes'] and hashlib.sha256(raw).hexdigest()==info['decoded_sha256'], name
def read(name):
    blob=(P/name).read_bytes()
    return json.loads(gzip.decompress(blob) if name.endswith('.gz') else blob)
trial=read('trial.json')
assert not trial['selected_changed'] and not trial['candidate_selected']
for scope in ['source', 'compiler']:
    latency=read(scope+'-latency.json.gz')
    resource=read(scope+'-resources.json.gz')
    assert len(latency['samples'])==341 and len(resource['samples'])==84
    assert all(s['exit']==0 and s['exact_rust_output'] for s in latency['samples']+resource['samples'])
    rows={(s['sample'],s['variant']):s for s in latency['samples']}
    assert len(rows)==341
    baseline='mech-ir242-reuse-4'
    for variant, info in trial[scope+'_cohort']['trials'].items():
        passed=True
        for field in ['post_json_seconds','wall_seconds']:
            deltas=[s[field]-rows[(s['sample'],baseline)][field] for s in latency['samples'] if s['variant']==variant]
            assert len(deltas)==31
            wins=sum(delta<0 for delta in deltas)
            assert wins==info['paired_vs_selected'][field]['wins'], (scope,variant,field)
            assert abs(statistics.median(deltas)-info['paired_vs_selected'][field]['median_paired_delta_seconds'])<1e-12
            passed=passed and wins>=21
        assert passed==info['passes_joint_timing_gate']
assert trial['source_cohort']['trials']['mech-ir242-invariant-reads-4']['passes_joint_timing_gate']
assert not trial['compiler_cohort']['trials']['mech-ir242-invariant-reads-4']['passes_joint_timing_gate']
assert not any(x['passes_joint_timing_gate'] for x in trial['compiler_cohort']['trials'].values())
assert read('tests/gpu-result.json')['expected_failures_exact']
assert read('source-qualification/summary.json')['status']=='passed'
print(json.dumps(dict(files=len(manifest['files']),latency_outputs=682,resource_outputs=168,paired_gates_verified=True,selected_changed=False)))
