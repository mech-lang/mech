from pathlib import Path
from datetime import datetime,timezone
from statistics import median
import json

D=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
read=lambda p:json.loads(p.read_text())
latency=D/'comparison-native-selected-ranges-randomized-20261107-5000-m3.json'
resources=D/'comparison-native-selected-ranges-resources-20261107-5000-m3.json'
a,b=read(latency),read(resources)
assert len(a['samples'])==434 and len(b['samples'])==105
assert all(x['exit']==0 and x['exact_rust_output']for x in a['samples']+b['samples'])
rows={(x['sample'],x['variant']):x for x in a['samples']}
def pairs(variant,baseline):
 result={}
 for field in ['post_json_seconds','wall_seconds']:
  deltas=[x[field]-rows[(x['sample'],baseline)][field]for x in a['samples']if x['variant']==variant]
  assert len(deltas)==31
  result[field]=dict(wins=sum(x<0 for x in deltas),median_paired_delta_seconds=median(deltas))
 return result
trials={}
for name,timing in a['summary'].items():
 if not name.startswith('mech'):continue
 paired=pairs(name,'mech-ir242-reuse-4')
 trials[name]=dict(timing=timing,resources=b['summary'][name+'-instrumented'],paired_vs_selected=paired,
                   passes_joint_gate=all(x['wins']>=21 for x in paired.values()))
assert not any(x['passes_joint_gate']for x in trials.values())
T=Path('/private/tmp/mech-related-selected-range-integration-tests-20261107-v1')
gpu=read(T/'test-run.json');ci=read(T/'ci-selected-inventory-audit.json')
assert gpu['expected_failures_exact'] and not gpu['new_failures'] and '884 passed; 2 failed' in gpu['summary'][0]
assert ci['expected']==ci['listed']==ci['executed_passed']==389 and ci['all_names_match']
C=Path('/private/tmp/mech-related-selected-range-integration-regressions-20261107-v1')
assert all(read(C/(kind+'-comparison.json'))['all_nontiming_fields_equal_IR240']for kind in ['scalar','one-kib'])
U=Path('/private/tmp/mech-related-selected-range-integration-runners-20261107-v1')
metadata=read(U/'reuse-4/build/metadata.json')
profile=read(Path('/private/tmp/mech-related-selected-range-restore-allocation-profile-20261107-v1/summary.json'))
restore=profile['variants']['reuse-4'];h=restore['header']
assert h['buffer_dropped']==h['reentrant_dropped']==0 and restore['unattributed_requests']==0
counter=trials['mech-ir252-reuse-4']['resources']['stage_measurements_median']['restore']
assert restore['requests']==counter['allocation_requests']==26837
assert restore['requested_bytes']==counter['requested_allocation_bytes']==12830999
escapes=read(Path('/private/tmp/mech-related-selected-ranges-borrowed-json-runners-20261107-v1/escaped-field-checks/summary.json'))
assert escapes['all_outputs_exact'] and escapes['fresh_processes']==32
report=dict(schema='mech.related-post-gen.selected-ranges-and-json-ownership.v1',updated_utc=datetime.now(timezone.utc).isoformat(),
 selected_compiler='4cc266ae0d05c69432a4c15ffe99d5e586685647',selected_identity='IR242/work115/ABI41/image9',selected_workers=4,selected_source='ordered-combinadic-mask-reuse',selected_changed=False,
 candidate_compiler='c879e9541277635d9faca26121b00c43d349193b',candidate_identity='IR252/work115/ABI41/image9',candidate_branch='codex/related-selected-range-integration-20261107',candidate_selected=False,
 integrated_PR973='7f71c3796bac86eb963f6fe28eda7504022089e1',
 latency_report=str(latency),resource_report=str(resources),latency_seed=990252,resource_seed=9902527,exact_latency_outputs=434,exact_resource_outputs=105,
 all_first_and_adverse_samples_retained=True,resource_and_profile_timings_excluded=True,matched_reference=a['matched_native_reference'],whole_reference=a['whole_process_reference'],trials=trials,
 qualification=dict(gpu=gpu,actual_CI=dict(expected=ci['expected'],listed=ci['listed'],executed=ci['executed_passed'],names_exact=True),controls_all_nontiming_fields_exact=133,Rust_edge_outputs_exact=248,extended_host_differential_processes=32,full_adoption_qualification_run=False),
 compiled_root_selected_range_branches_removed=metadata['compiled_cfg_cleanup']['selected_range_branches_removed'],
 numeric_selected_range_branches_removed=sum(x['cleanup']['selected_range_branches_removed']for x in metadata['compiled_numeric_functions']),
 source_combinations=['hoisted invariant tag reads','partial combinadic sums','byte IDs/U32 ordered keys with four native workers','same compact source with eight native workers'],
 borrowed_json=dict(scope='Host IO source optimization only; tags retain owned transfer to canonical inputs, exact native bundles and continuous timing boundaries unchanged. Cow borrows ordinary ID/title text and owns escaped fallback.',
                    extended_tests=escapes,JSON_requests_before=40089,JSON_requests_after=30089,selected=False,
                    paired_IR242_borrowed_vs_owned=pairs('mech-ir242-borrowed-json-4','mech-ir242-reuse-4'),
                    paired_IR252_borrowed_vs_owned=pairs('mech-ir252-borrowed-json-4','mech-ir252-reuse-4'),
                    decision='Fewer JSON allocations did not translate to better measured latency; processing and memory worsen. Keep unselected.'),
 restore_allocation_profile=dict(path='/private/tmp/mech-related-selected-range-restore-allocation-profile-20261107-v1',requests=26837,requested_bytes=12830999,counts_and_bytes_equal_resource_cohort=True,zero_loss_and_unattributed=True,
   scope='Allocation ancestry only; inclusive stack counts overlap and are not CPU-time percentages. Genuine compiled-module restoration before JSON. Native Object audited unchanged; timings excluded.',
   top_allocation_symbols=restore['top_allocation_symbols'],top_call_stacks=restore['top_call_stacks']),
 size_report=str(D/'size-native-selected-ranges-20261107.json'),
 next_compiler_gap='Root numeric frames use CompiledFrame but static parent-piece composition frames still select owned FrameWords. Carry the existing reader through private composition wire types, preserving eager structural validation and independent derivation.',
 followup=dict(compiler='b36d51f6d879fe9636ada1964b326ce50392b330',identity='IR253/work115/ABI41/image9',source='/private/tmp/mech-related-borrowed-parent-frame-metadata-compiler-20261107',build='/private/tmp/mech-related-borrowed-parent-frame-metadata-opt-20261107-v1',selected=False,status='12-stage build passed; new borrowed-parent tests and full controls in progress; no speed claim'),
 decision='Keep selectedIR242. IR252 wins18 processing/20 whole pairs; all source/host combinations fail the joint timing gate. Eight compact workers regress again. Original promotion record stays unchanged.',goal_complete=False)
out=D/'compiler-native-selected-ranges-and-json-ownership-20261107.json';out.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(report=str(out),selected_changed=False,all_candidates_rejected=True,restore_profile_exact=True,next_trial='IR253')))
