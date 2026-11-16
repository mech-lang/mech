from pathlib import Path
import json, statistics, hashlib
U=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D=R/'results'
T=Path('/private/tmp/mech-related-native-boolean-implication-freshness-tests-20261106-v4')
P=Path('/private/tmp/mech-related-native-boolean-implication-mask-reuse-pc-profile-20261106-v1')
read=lambda p:json.loads(p.read_text())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
write=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')

fixtures={}
for path in sorted(U.glob('edges-*/edge-fixtures.json'))+sorted(U.glob('*/edges/edge-fixtures.json')):
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

tests=read(T/'test-run.json');assert tests['expected_failures_exact'] and not tests['new_failures'] and '849 passed; 2 failed' in tests['summary'][0]
accounting=read(T/'boolean-accounting-audit.json');assert accounting['rows']==112 and accounting['first_case_difference_entirely_within_callbacks']
ci=read(T/'ci-selected-inventory-audit.json');assert ci['current_selected_count']==354 and ci['all_selected_names_executed_and_passed_in_complete_gpu_run']
latency=read(D/'comparison-native-boolean-implication-mask-reuse-randomized-20261106-5000-m3.json')
resources=read(D/'comparison-native-boolean-implication-mask-reuse-resources-20261106-5000-m3.json')
for report,count in [(latency,279),(resources,70)]:
    assert len(report['samples'])==count and all(x['exit']==0 and x['exact_rust_output']for x in report['samples'])
    assert report['execution_order']['mode']=='randomized'
by={}
for row in latency['samples']:by.setdefault(row['variant'],{})[row['sample']]=row
paired={}
for candidate in ['mech-current-original-4','mech-current-reuse-4']:
    paired[candidate]={}
    for reference in ['mech-parent-original-4','mech-parent-reuse-4','mech-current-original-4']:
        if candidate==reference:continue
        paired[candidate][reference]={}
        for metric in ['post_json_seconds','wall_seconds']:
            deltas=[by[candidate][i][metric]-by[reference][i][metric]for i in by[candidate]]
            paired[candidate][reference][metric]=dict(rounds=31,wins=sum(x<0 for x in deltas),median_paired_delta_seconds=statistics.median(deltas))

native=read(U/'native-text-equivalence.json');assert len(native)==4 and all(x['complete_native_text_byte_identical']for x in native.values())
new=read(U/'reuse-4/build/metadata.json');assert new['typed_ir_version']==235 and next(x for x in new['numeric_tree_candidates']if x['iteration']==3)['selected']
before=Path('/private/tmp/mech-related-ordered-subsets-reused-masks-source-20261106-v1')
assert sha(before/'source.mec')==sha(U/'reuse-4/source.mec')=='3c9d6fce65742e15dec0718571ac1f76b4f41f384ea1964bcafe92844bb00d99'
profiles=read(P/'summary.json');assert set(profiles['rows'])=={'before','after'}
assert all(x['all1000_index_arrays_and_final_rust_output_exact'] and x['profiled_timings_excluded']for x in profiles['rows'].values())
summary=dict(schema='mech.related-post-gen.boolean-implication-native-mask-reuse.v1',compiler='cbe5b47839b4fe2300fe16ebc1fcacfcfa03f98b',identity='IR235/work115/ABI41/image9',incoming_973='176d8134133befb8e35a6413fb8f98387bacebb3',
    problem='A conjunction guarding append and a later read of the same Bool tuple field lost correlation; the numeric tree rejected an allegedly older returned array.',
    fix='Prove every Boolean producer canonical, normalize exact current private scalar projections, require collectively dominating producers with at most one producer execution per traversal, and propagate And-true/Or-false/Not implications. Reset facts at lexical header/exit; unknown representations retain the checked route.',
    source_sha256=sha(U/'reuse-4/source.mec'),source_unchanged_across_compilers=True,
    qualification=read(U/'qualification-summary.json')|dict(gpu_passed=849,inherited_gpu_failures=tests['failed_tests'],ci_selected_names_passed=354,new_regressions_passed=5,accounting_observations=112),
    latency_summary=latency['summary'],matched_reference=latency['matched_native_reference'],whole_process_reference=latency['whole_process_reference'],paired=paired,
    resources_summary=resources['summary'],size_metrics=read(D/'size-native-boolean-implication-mask-reuse-20261106.json'),
    all_four_original_native_text_sections_equal_IR234=True,new_source_numeric_functions=len(new['compiled_numeric_functions']),new_source_native_code_bytes=new['code_bytes'],new_source_main_stack_bytes=new['stack_bytes'],
    profile_invocations_exact=2000,profile=read(P/'all-symbol-observations.json'),query_instruction_profile=read(P/'worker-instruction-observations.json')['variants']['after']['rows']['_mech_canonical_program_entry_v1_numeric_1282'],
    work_accounting=dict(all_totals_retained=True,production_prices_meter_or_funding_changed=False,source_price_tables_equal=True,first_fixture_reference_total=202217009,first_fixture_native_total=84816871,first_fixture_outside_callbacks_both=907408,first_fixture_difference_within_callbacks=117400138,total_work_includes_actual_runtime_owner_metadata_and_transport_charges=True),
    selected=False,selected_speed_checkpoint='IR208',goal_complete=False,
    limitations=['The sixfold gain repairs the same source regression; the original benchmark improves only modestly.',
        'Current source wins27/31 processing pairs but only14/31 full-process pairs against same-compiler original source. Whole-process improvement is not established.',
        'Ordered source sends unordered unique rows to the exact general fallback. This scope tradeoff remains explicit; source stays unselected.',
        'Instruction/resource samples are separate from latency. PC observations and waiting positions are not CPU-time percentages.',
        'No hosted-CI-green, merge, or remotely uploaded raw-archive claim.'],
    next_targets=['Cheaper immutable first-invocation and restoration metadata; avoid discarded descriptor clones and redundant owned-shape parameter copies.',
        'Producer-derived immutable posting-ID/index ranges, bounded native-tree funding, and cold scalar-home materialization/retirement.',
        'Combine compact source representations and ordered key/mask reuse only with separate exactness, counter and latency evidence.'])
write(D/'compiler-native-boolean-implication-mask-reuse-20261106.json',summary)
print(json.dumps(dict(status='verified',fixtures=558,gpu_passed=849,cohort_outputs=349,profile_invocations=2000,selected=False)),flush=True)
