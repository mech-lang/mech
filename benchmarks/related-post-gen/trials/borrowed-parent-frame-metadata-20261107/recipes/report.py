from pathlib import Path
import datetime,json,statistics

R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
read=lambda p:json.loads(p.read_text())
a=read(R/'comparison-native-borrowed-parent-frame-metadata-randomized-20261107-5000-m3.json')
b=read(R/'comparison-native-borrowed-parent-frame-metadata-resources-20261107-5000-m3.json')
rows={(x['sample'],x['variant']):x for x in a['samples']}
assert len(rows)==248 and len(b['samples'])==63
assert all(x['exit']==0 and x['exact_rust_output']for x in a['samples']+b['samples'])
trials={}
for name in ['mech-ir242-reuse-4','mech-ir252-reuse-4','mech-ir253-reuse-4']:
 paired={}
 for field in ['post_json_seconds','wall_seconds']:
  values=[x[field]-rows[(x['sample'],'mech-ir242-reuse-4')][field]for x in a['samples']if x['variant']==name]
  assert len(values)==31
  paired[field]=dict(wins=sum(x<0 for x in values),median_paired_delta_seconds=statistics.median(values))
 trials[name]=dict(timing=a['summary'][name],resources=b['summary'][name+'-instrumented'],paired_vs_selected=paired,passes_joint_gate=all(v['wins']>=21 for v in paired.values()))
T=Path('/private/tmp/mech-related-borrowed-parent-frame-metadata-tests-20261107-v2')
t=read(T/'test-run.json');ci=read(T/'ci-selected-inventory-audit.json')
assert t['expected_failures_exact']and not t['new_failures']and '886 passed; 2 failed'in t['summary'][0]
assert ci['expected']==ci['listed']==ci['executed_passed']==391 and ci['all_names_match']
C=Path('/private/tmp/mech-related-borrowed-parent-frame-metadata-regressions-20261107-v2')
assert all(read(C/(k+'-comparison.json'))['all_nontiming_fields_equal_IR240']for k in ['scalar','one-kib'])
P=Path('/private/tmp/mech-related-parent-frame-post-json-allocation-profile-20261107-v1')
profile=read(P/'summary.json');old=profile['variants']['252'];new=profile['variants']['253']
assert (old['requests'],old['requested_bytes'],new['requests'],new['requested_bytes'])==(3758,8012356,3824,8014468)
assert old['unattributed_requests']==new['unattributed_requests']==0
def count(row,needle):
 return next((v for v in row['all_inclusive_symbols']if needle in v['symbol']),dict(requests=0,requested_bytes=0))
x=count(new,'Invocation>::begin_numeric_segment');y=count(old,'Invocation>::begin_numeric_segment')
assert (x['requests']-y['requests'],x['requested_bytes']-y['requested_bytes'])==(66,2112)
report=dict(schema='mech.related-post-gen.borrowed-parent-frame-metadata.v1',updated_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),selected_compiler='4cc266ae0d05c69432a4c15ffe99d5e586685647',selected_identity='IR242/work115/ABI41/image9',selected_workers=4,selected_changed=False,candidate_compiler='d79d97570749426d67c10c935c3109137492cc67',candidate_identity='IR253/work115/ABI41/image9',candidate_branch='codex/related-borrowed-parent-frame-metadata-20261107',candidate_selected=False,integrated_PR973='7f71c3796bac86eb963f6fe28eda7504022089e1',latency_report=str(R/'comparison-native-borrowed-parent-frame-metadata-randomized-20261107-5000-m3.json'),resource_report=str(R/'comparison-native-borrowed-parent-frame-metadata-resources-20261107-5000-m3.json'),size_report=str(R/'size-native-borrowed-parent-frame-metadata-20261107.json'),latency_seed=990253,resource_seed=9902537,exact_latency_outputs=248,exact_resource_outputs=63,all_first_and_adverse_samples_retained=True,resource_and_profile_timings_excluded=True,matched_reference=a['matched_native_reference'],whole_reference=a['whole_process_reference'],trials=trials,qualification=dict(gpu=t,actual_CI=dict(expected=391,listed=391,executed=391,names_exact=True),controls_all_nontiming_fields_exact=133,Rust_edge_outputs_exact=62,full_adoption_qualification_run=False),native_text_equality=read(Path('/private/tmp/mech-related-borrowed-parent-frame-metadata-cohort-20261107-v2/native-text-equality.json')),post_JSON_allocation_profile=dict(path=str(P),requests_before=3758,requests_after=3824,bytes_before=8012356,bytes_after=8014468,counts_match_resource_cohort=True,zero_loss_truncation_invalid_or_unattributed=True,extra_begin_numeric_segment_requests=66,extra_begin_numeric_segment_bytes=2112,mechanism='The element-limit predicate uses frame.fields.len(), materializing the compiler-only field/path vectors lazily. This is actual post-JSON work, not an allocation-counting artifact.',scope='Allocation ancestry only; inclusive counts overlap and are not CPU-time percentages. Both native Objects are audited unchanged and complete Rust output is exact.'),startup_allocation_requests_before=26837,startup_allocation_requests_after=25142,initial_development_failure_retained='/private/tmp/mech-related-borrowed-parent-frame-metadata-tests-20261107-v1/focus-borrowed_parent_frame_metadata.json',initial_test_correction='Legacy V2 serialization omits static layouts. The corrected positive test requires those frames remain borrowed through V2 and materialize only through exact V9 table serialization.',followup=dict(identity='IR255/work115/ABI41/image9',source='/private/tmp/mech-related-parent-frame-runtime-counts-compiler-20261107',change='Cache the exact structurally validated field count in runtime metadata and use it in the unchanged finite element-limit predicate.',selected=False,performance_measured=False),decision='Reject IR253 as a performance candidate. It saves1695 restore allocations but adds66 processing allocations and wins only10/31 processing and2/31 whole-process pairs against selectedIR242. Fix the runtime metadata consumer rather than shifting compiler reconstruction out of the startup timer.',goal_complete=False)
out=R/'compiler-native-borrowed-parent-frame-metadata-20261107.json';out.write_text(json.dumps(report,indent=2)+'\n')
print(out)
print(json.dumps(dict(selected=trials['mech-ir242-reuse-4']['timing'],candidate=trials['mech-ir253-reuse-4']['timing'],paired=trials['mech-ir253-reuse-4']['paired_vs_selected'],selected_changed=False)))
