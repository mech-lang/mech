from pathlib import Path
from datetime import datetime, timezone
import json, subprocess

D=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
read=lambda p:json.loads(p.read_text())
write=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')
now=datetime.now(timezone.utc).isoformat()
report_path=D/'compiler-native-invariant-capture-selection-20261107.json';report=read(report_path)
pr=read(Path('/private/tmp/mech-related-invariant-capture-selection-pr990-publication-20261107.json'))
publication_path=Path('/private/tmp/mech-related-invariant-capture-selection-publication-20261107.json');publication=read(publication_path)
publication.update(pushed=True,remote_verified=True,pr=pr['url'],pr_updated_at=pr['updated_at']);write(publication_path,publication)
T=Path('/private/tmp/mech-related-immutable-capture-views-tests-20261107-v2')
groups={p.stem:read(p)for p in T.glob('focus-*.json')}
assert len(groups)==8 and all(x['exit']== 0 for x in groups.values())
controls=Path('/private/tmp/mech-related-immutable-capture-views-regressions-20261107-v1')
assert all(read(controls/(k+'-comparison.json'))['all_nontiming_fields_equal_IR240']for k in ['scalar','one-kib'])
equivalence=read(Path('/private/tmp/mech-related-immutable-capture-views-runners-20261107-v1/native-text-equivalence.json'))
assert all(v['full_native_text_byte_identical']for v in equivalence.values())
noop=dict(schema='mech.related-post-gen.immutable-capture-view-noop.v1',updated_utc=now,
 compiler='061822c2c4e8a4413d1d80afc16e47efc9c0d731',test_fixture_fix_commit='e6d89442f5f8b77b44f3ce4fe5dcdc2abe031c4e',identity='IR251/work115/ABI41/image9',
 branch='codex/related-immutable-capture-views-20261107',pushed=True,selected=False,
 all12_build_stages_passed=True,normal_native_text_equivalence=equivalence,
 focused_groups=groups,test_only_source_change=read(T/'test-only-source-change.json'),actual_CI_inventory=read(T/'ci-inventory-only.json'),
 scalar_control_rows_exact=128,DEFLATE_control_rows_exact=5,new_timing_cohort=False,full_GPU_CI_execution=False,full_adoption_qualification=False,
 initial_failure='New internally owned fixture called sequence/with-capacity with incorrect one-argument arity. Corrected to typed empty sequence; all three anchor fixtures then pass. Initial logs retained.',
 decision='Reject optimization hypothesis for this benchmark: both full native text sections equalIR250 exactly. Existing compiler already retains these view values. No native processing gain claimed.',
 report_location=str(D/'compiler-native-immutable-capture-views-noop-20261107.json'))
write(Path(noop['report_location']),noop)
audit_path=Path('/private/tmp/mech-related-invariant-read-backend-audit-runners-20261107-v1/summary.json');audit=read(audit_path)
assert len(audit['all13_normal_bundle_files_byte_identical'])==13 and all(audit['all13_normal_bundle_files_byte_identical'].values())
assert audit['optimized_clones']==51
audit['interpretation_limit']='The103 readonly loads/23 inside loops include backend duplication and nested activations. Map each load to its source activation and expected hoist point before claiming a LICM gap. No timing/dynamic-instruction inference from these static counts.'
audit['next_actions']=['Integrate newly fetched PR973 selected-range branch folding in a separate frozen compiler epoch.','Map original and optimized capture loads to nested loop headers and invariant definitions; distinguish valid outer-loop placement from missed inner-loop motion.','Inspect repeated source bounds/work guards and backend rematerialization using exact CLIF/assembly. Apply only proved native transformations; retain finite source errors and prefixes.','Continue restoration metadata allocation ancestry work; selected restore still26,837 allocations and about125.9M instructions.']
write(audit_path,audit)
incoming=dict(head='7f71c3796bac86eb963f6fe28eda7504022089e1',change='Fold native branches using selected SSA ranges',integrated=False,
 integrated_in_latest_frozen_trial_through='2f7f38a4332c93cbaa6679a7138f7e0717fc9043',selected_integrated_through='65034db9ca9da4133355108e85784703cde7592b',checked_utc=now)
candidate=dict(compiler=report['candidate_compiler'],identity=report['candidate_identity'],branch=report['candidate_branch'],pushed=True,selected=False,
 report=str(report_path),evidence_commit=publication['evidence_commit'],evidence_files_verified=190,
 paired_post_json_wins=13,paired_whole_process_wins=25,rounds=31,passes_joint_gate=False,
 actual_GPU_passed=880,inherited_GPU_failures=2,new_GPU_failures=0,actual_CI_executed=385,controls_nontiming_exact=133,edges_exact=124,
 full_adoption_qualification_run=False)
selected=report['trials']['mech-ir242-reuse-4']
remeasured=dict(compiler=report['selected_compiler'],identity=report['selected_identity'],selected=True,
 comparison=report['latency_report'],resources=report['resource_report'],timing=selected['timing'],resource_summary=selected['resources'],
 matched_reference=report['matched_reference'],whole_process_reference=report['whole_reference'],note='Fresh measurement of unchanged selected source/compiler; original promotion timing is preserved. Separate resource timings excluded.')
handoff=Path('/private/tmp/mech-related-invariant-native-lowering-current-handoff-20261107.json')
checkpoint=D/'selected-speed-checkpoint-20261107.json';promotion=read(checkpoint)['timing']
for name in ['selected-speed-checkpoint-20261107.json','published-integrated-review-branch-20261106.json','next-related-compiler-gaps-20261106.json','compiler-branches-20261105.json']:
 p=D/name;snapshot=D/'ledger-history-20261107'/(p.stem+'-before-IR250-publication.json')
 if not snapshot.exists():snapshot.write_bytes(p.read_bytes())
 d=read(p);d.update(updated_utc=now,latest_selected_remeasurement=remeasured,unselected_IR250=candidate,
    unselected_IR251=dict(compiler=noop['compiler'],test_fixture_fix_commit=noop['test_fixture_fix_commit'],report=noop['report_location'],pushed=True,selected=False,full_native_text_unchanged=True,full_GPU_CI_execution=False),
    invariant_source_all93=report['qualified_source_repeat'],current_handoff=str(handoff),goal_complete=False)
 if name!=checkpoint.name:d['incoming_pr973']=incoming
 if name=='published-integrated-review-branch-20261106.json':
  assert d['commit']==pr['selected_head'];d['latest_trial_evidence']=publication
 elif name=='next-related-compiler-gaps-20261106.json':
  d.update(latest_report=str(report_path),status='active: IR250 rejected, IR251 emits unchanged native text, backend audit available',next_compiler_hypotheses=audit['next_actions'])
 elif name=='compiler-branches-20261105.json':
  d.update(latest_compiler_candidate=candidate,latest_comparison=report['latency_report'],latest_performance_comparison=report['latency_report'],latest_resource_metrics=report['resource_report'],latest_sizes=report['size_report'],
           latest_execution_profile='/private/tmp/mech-related-invariant-capture-all-thread-profile-20261107-v1',latest_backend_audit=str(audit_path),next_action=audit['next_actions'],
           current_goal_turn_classification='Progress: integrated/qualified IR249, implemented/qualified/measured rejected IR250, ruled out entry-view gap with exact native bytes, collected current all-thread profiles and51 optimized IR clones; selected unchanged.')
  for k in ['current','current_same_cohort']:
   if isinstance(d.get(k),dict):d[k].update(comparison=report['latency_report'],resources=report['resource_report'])
 write(p,d)
assert read(checkpoint)['timing']==promotion
state=dict(updated_utc=now,goal_active=True,goal_complete=False,previous_goal_turn_classification='progress',
 selected=dict(compiler=report['selected_compiler'],identity=report['selected_identity'],workers=4,source='ordered-combinadic-mask-reuse',checkpoint=str(checkpoint)),
 latest_report=str(report_path),latest_performance_cohort=report['latency_report'],latest_resource_cohort=report['resource_report'],
 publication=publication,pr=pr,candidate=candidate,noop_IR251_report=noop['report_location'],
 backend_audit=dict(compiler=audit['compiler'],source='/private/tmp/mech-related-invariant-read-backend-audit-compiler-20261107',build='/private/tmp/mech-related-invariant-read-backend-audit-opt-20261107-v1',runners=str(audit_path.parent),report=str(audit_path),branch='codex/related-invariant-read-backend-audit-20261107',pushed_pending_verification=True,all13_normal_bundle_files_exact=True,optimized_clones=51),
 current_profiles='/private/tmp/mech-related-invariant-capture-all-thread-profile-20261107-v1',incoming_PR973=incoming,
 next_actions=audit['next_actions'],live_own_jobs=[],pending_publication=False)
write(handoff,state)
for old in ['/private/tmp/mech-related-invariant-capture-hoisting-current-handoff-20261107.json','/private/tmp/mech-related-invariant-and-capture-current-handoff-20261107.json']:
 p=Path(old);d=read(p);d['superseded_by']=str(handoff);d['live_sessions']={};write(p,d)
print(json.dumps(dict(handoff=str(handoff),selected_unchanged=True,IR250_selected=False,IR251_native_text_unchanged=True,backend_audit_ready=True,new_incoming=incoming['head'],goal_active=True)))
