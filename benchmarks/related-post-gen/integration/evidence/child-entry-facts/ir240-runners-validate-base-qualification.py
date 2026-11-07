from pathlib import Path
import json, statistics, hashlib
U=Path('/private/tmp/mech-related-child-entry-facts-integration-runners-20261107-v1')
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D=R/'results'
T=Path('/private/tmp/mech-related-native-boolean-implication-freshness-tests-20261106-v4')
P=Path('/private/tmp/mech-related-native-boolean-implication-mask-reuse-pc-profile-20261106-v1')
read=lambda p:json.loads(p.read_text())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
write=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')

fixtures={}
for path in sorted(U.glob('edges-*/edge-fixtures.json'))+sorted(U.glob('*/edges/edge-fixtures.json')):
    
    if 'rust-insert' in str(path):continue
    rows=read(path);assert len(rows)==31
    assert all(x['exit']==0 and x.get('exact_expected_output',x.get('exact_rust_output',False))for x in rows)
    fixtures[str(path.relative_to(U))]=dict(cases=31,sha256=sha(path))
assert len(fixtures)==18 and sum(x['cases']for x in fixtures.values())==558
decoded=read(U/'original-decoded-correspondence.json')
assert len(decoded)==4 and all(x['all13_bundle_files_byte_identical'] and len(x['files'])==13 for x in decoded.values())
assert read(U/'reuse-original-decoded-correspondence.json')['all13_bundle_files_byte_identical']
denials=successes=0
for family in ['budget','compact-budget']:
    budget=read(U/family/'summary.json');assert budget['all_eight_denials_exact']
    rows=[x for x in budget['cases']if x['case']!='success'];assert len(rows)==8
    for row in rows:
        assert row['denial_exact'] and len(row['results'])==3
        assert all(x['output_unpublished'] and x['retained_bytes']==0 and x['retained_nodes']==0 for x in row['results'].values())
        denials+=3
    for loader in ['bound','independent','full-catalog']:
        for size in ['first-10','full-5000']:
            row=read(U/family/loader/('success-rust-'+size+'-run.json'))
            assert row['exit']==0 and row['exact_standard_rust_output'];successes+=1
frontier=read(U/'frontier-image-probe/run.json');assert frontier['exit']==0
frontier=json.loads(frontier['stdout']);assert frontier['genuine_legacy_independent_bound_and_compiled_module_accepted'] and frontier['native_entry_never_invoked']
assert len(frontier['forged_cases'])==8 and all(x['independent_rejected'] and x['object_bound_rejected']for x in frontier['forged_cases'])
assert denials==48 and successes==12
write(U/'qualification-summary.json',dict(status='passed',loader_fixtures_exact=558,fixtures=fixtures,decoded_bundle_files_exact=65,finite_denials_exact=48,finite_small_full_successes_exact=12,forged_frontier_rejections=16))

print('All558 loader fixtures,65 decoded files,48 finite denials,12 successes and16 forged frontier rejections pass')
