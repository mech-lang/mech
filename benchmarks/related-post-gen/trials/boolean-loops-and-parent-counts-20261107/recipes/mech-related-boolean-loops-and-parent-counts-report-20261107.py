from pathlib import Path
import datetime,json,statistics

R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
P=Path('/private/tmp/mech-related-boolean-loops-and-parent-counts-cohort-20261107-v2')
read=lambda p:json.loads(p.read_text())
a=read(R/'comparison-native-boolean-loops-and-parent-counts-randomized-20261107-5000-m3.json')
b=read(R/'comparison-native-boolean-loops-and-parent-counts-resources-20261107-5000-m3.json')
rows={(x['sample'],x['variant']):x for x in a['samples']}
assert len(rows)==310 and len(b['samples'])==77
assert all(x['exit']==0 and x['exact_rust_output']for x in a['samples']+b['samples'])
trials={}
for epoch in [242,252,253,255,257]:
 name=f'mech-ir{epoch}-reuse-4';pairs={}
 for baseline in ['mech-ir242-reuse-4','mech-ir252-reuse-4']:
  pair={}
  for field in ['post_json_seconds','wall_seconds']:
   values=[x[field]-rows[(x['sample'],baseline)][field]for x in a['samples']if x['variant']==name]
   assert len(values)==31
   pair[field]=dict(wins=sum(x<0 for x in values),median_paired_delta_seconds=statistics.median(values))
  pairs[baseline]=pair
 trials[name]=dict(timing=a['summary'][name],resources=b['summary'][name+'-instrumented'],paired_vs_selected=pairs['mech-ir242-reuse-4'],paired_vs_IR252=pairs['mech-ir252-reuse-4'],passes_joint_gate=all(v['wins']>=21 for v in pairs['mech-ir242-reuse-4'].values()))
qualification={}
for epoch,label,version,passed,count in [(255,'parent-frame-runtime-counts','v1',887,392),(257,'boolean-loop-integration','v3',888,393)]:
 T=Path('/private/tmp/mech-related-'+label+'-tests-20261107-'+version);t=read(T/'test-run.json');ci=read(T/'ci-selected-inventory-audit.json')
 assert t['expected_failures_exact']and not t['new_failures']and str(passed)+' passed; 2 failed'in t['summary'][0]
 assert ci['expected']==ci['listed']==ci['executed_passed']==count and ci['all_names_match']
 C=Path('/private/tmp/mech-related-'+label+'-regressions-20261107-'+version)
 assert all(read(C/(kind+'-comparison.json'))['all_nontiming_fields_equal_IR240']for kind in ['scalar','one-kib'])
 qualification[str(epoch)]=dict(GPU_passed=passed,inherited_failures=2,new_failures=0,actual_CI_executed=count,actual_CI_names_exact=True,controls_all133_nontiming_fields_exact=True,Rust_edge_outputs_exact=62,full_adoption_qualification_run=False)
profile=read(Path('/private/tmp/mech-related-parent-frame-counts-post-json-allocation-profile-20261107-v1/summary.json'))['variants']['255']
assert profile['requests']==3758 and profile['requested_bytes']==8012356 and profile['unattributed_requests']==0
assert not any('CompilerFrameData'in v['symbol']for v in profile['all_inclusive_symbols'])
metrics=trials['mech-ir255-reuse-4']['resources']['post_json_measurement_median']
assert (metrics['allocation_requests'],metrics['requested_allocation_bytes'])==(profile['requests'],profile['requested_bytes'])
meta=read(Path('/private/tmp/mech-related-boolean-loop-integration-runners-20261107-v3/reuse-4/build/metadata.json'))
query=next(x for x in meta['compiled_numeric_functions']if x['header']==1282)
report=dict(schema='mech.related-post-gen.boolean-loops-and-parent-counts.v1',updated_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),selected_compiler='4cc266ae0d05c69432a4c15ffe99d5e586685647',selected_identity='IR242/work115/ABI41/image9',selected_workers=4,selected_changed=False,candidate_compiler='70f5744439f1d7e027b5e9ab9a0e3fcf1c08bcba',candidate_identity='IR257/work115/ABI41/image9',candidate_selected=False,candidate_branch='codex/related-boolean-loop-integration-20261107',metadata_compiler='93b5569cc478740fab9b823f5093286e5564b694',metadata_identity='IR255/work115/ABI41/image9',metadata_branch='codex/related-parent-frame-runtime-counts-20261107',integrated_PR973='b6a42fde80dbc011dfefa9de5d9659be334411b0',latency_report=str(R/'comparison-native-boolean-loops-and-parent-counts-randomized-20261107-5000-m3.json'),resource_report=str(R/'comparison-native-boolean-loops-and-parent-counts-resources-20261107-5000-m3.json'),size_report=str(R/'size-native-boolean-loops-and-parent-counts-20261107.json'),latency_seed=990257,resource_seed=9902577,exact_latency_outputs=310,exact_resource_outputs=77,all_first_and_adverse_samples_retained=True,resource_and_profile_timings_excluded=True,matched_reference=a['matched_native_reference'],whole_reference=a['whole_process_reference'],trials=trials,qualification=qualification,native_text_proofs=read(P/'native-text-proofs.json'),fixed_parent_metadata_gap=dict(mechanism='Keep the exact validated field count in runtime metadata, so the existing element-limit comparison does not materialize compiler-only vectors.',post_JSON_requests=3758,post_JSON_requested_bytes=8012356,extra_IR253_requests_removed=66,extra_IR253_bytes_removed=2112,late_compiler_vector_allocations=0,counts_equal_resource_cohort=True,profile=str(Path('/private/tmp/mech-related-parent-frame-counts-post-json-allocation-profile-20261107-v1')),zero_loss_truncation_invalid_or_unattributed=True,scope='Allocation ancestry only; not CPU-time percentages. Native Object audited unchanged; all profile timings/memory excluded.'),fixed_boolean_gaps=dict(mechanism='Compose direct low-bit Boolean lowering with selected-edge facts only when the original SSA word is independently proved zero-or-one. Preserve exact singleton bit-and results in the shared range transfer.',initial_IR254_failure_retained='/private/tmp/mech-related-boolean-loop-integration-tests-20261107-v1/focus-cfg_cleanup.json',initial_IR256_failure_retained='/private/tmp/mech-related-boolean-loop-integration-tests-20261107-v2/focus-cfg_cleanup.json',original_upstream_proof_assertion_unchanged=True,new_four_width_unknown_word_and_publication_checks=True,root_boolean_phi_values=meta['compiled_cfg_cleanup']['boolean_phi_values'],root_selected_range_folds=meta['compiled_cfg_cleanup']['selected_range_branches_removed'],hot_query=query,hot_query_additional_selected_range_folds=0),interpretation=['IR255 removes the late compiler-vector reconstruction but is not faster reliably; it wins only8/31 processing and16/31 complete pairs versus selectedIR242.','IR257 wins13 processing/23 complete pairs; its whole-process improvement does not pass the unchanged joint promotion gate.','The hot query native body stays exactly27,552B/544B stack; new branch folds affect the root and fallback functions, explaining why static proof gains do not establish a hot-query gain.','SelectedIR242 processing is2.849875ms versus2.848000ms in the preceding cohort. Faster relative multiples partly reflect a slower1.644500ms Rust reference; they are not a new selected-compiler speedup.','All first/adverse cohorts remain. Original selected promotion timing is unchanged.'],next_compiler_gap=dict(source='/private/tmp/mech-related-forward-read-windows-compiler-20261107',identity='IR258/work115/ABI41/image9',change='Generalize complete immutable child-window admission beyond paired forward/reverse reads, preserving paired priority, exact current bounds and ordinary refusal. Require additional source checks be proved before duplicating a forward loop.',selected=False,performance_measured=False),decision='Keep selectedIR242/four native workers. Neither IR255 nor IR257 passes21/31 paired wins at both scopes. Compiler gaps are fixed in separately published, unselected branches; no full adoption qualification or Rust parity is claimed.',goal_complete=False)
out=R/'compiler-native-boolean-loops-and-parent-counts-20261107.json';out.write_text(json.dumps(report,indent=2)+'\n')
print(out)
print(json.dumps(dict(IR255_pairs=trials['mech-ir255-reuse-4']['paired_vs_selected'],IR257_pairs=trials['mech-ir257-reuse-4']['paired_vs_selected'],selected_changed=False)))
