from pathlib import Path
from datetime import datetime, timezone
from statistics import median
import json, hashlib

D = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
P = Path('/private/tmp/mech-related-invariant-capture-hoisting-cohort-20261107-v2')
read = lambda p: json.loads(p.read_text())
latency = D/'comparison-native-invariant-captures-randomized-20261107-5000-m3.json'
resources = D/'comparison-native-invariant-captures-resources-20261107-5000-m3.json'
a, b = read(latency), read(resources)
assert len(a['samples'])==372 and len(b['samples'])==91
assert all(x['exit']==0 and x['exact_rust_output'] for x in a['samples']+b['samples'])
rows={(x['sample'],x['variant']):x for x in a['samples']}
def pairs(name, baseline):
    result={}
    for field in ['post_json_seconds','wall_seconds']:
        deltas=[x[field]-rows[(x['sample'],baseline)][field] for x in a['samples'] if x['variant']==name]
        assert len(deltas)==31
        result[field]=dict(wins=sum(x<0 for x in deltas),median_paired_delta_seconds=median(deltas))
    return result
trials={}
for name,v in a['summary'].items():
    if not name.startswith('mech'):continue
    paired=pairs(name,'mech-ir242-reuse-4')
    trials[name]=dict(timing=v,resources=b['summary'][name+'-instrumented'],paired_vs_selected=paired,
        passes_joint_timing_gate=all(x['wins']>=21 for x in paired.values()))
assert not trials['mech-ir250-reuse-4']['passes_joint_timing_gate']
assert not trials['mech-ir250-reads-4']['passes_joint_timing_gate']
assert not trials['mech-ir250-partial-4']['passes_joint_timing_gate']
qualification={}
for label,version in [('integration','v1'),('hoisting','v2')]:
    T=Path('/private/tmp/mech-related-invariant-capture-'+label+'-tests-20261107-'+version)
    C=Path('/private/tmp/mech-related-invariant-capture-'+label+'-regressions-20261107-'+version)
    gpu=read(T/'test-run.json');ci=read(T/'ci-selected-inventory-audit.json')
    assert gpu['expected_failures_exact'] and not gpu['new_failures']
    controls={kind:read(C/(kind+'-comparison.json')) for kind in ['scalar','one-kib']}
    assert all(x['all_nontiming_fields_equal_IR240'] for x in controls.values())
    qualification[label]=dict(gpu=gpu,CI=dict(expected=ci['expected'],listed=ci['listed'],executed_passed=ci['executed_passed'],names_exact=ci['all_names_match']),
        controls=controls,upstream_lexical_test=read(T/'upstream-interval-integration-v2/run.json'),full_adoption_qualification_run=False)
profile=Path('/private/tmp/mech-related-invariant-capture-all-thread-profile-20261107-v1')
profiles={name:dict(run=read(profile/name/'run.json'),summary=read(profile/name/'summary.json'),
                   static_query=read(profile/name/'query-instructions.json')) for name in ['selected-ir242','candidate-ir250']}
assert all(x['run']['final_json_exact_Rust'] and x['run']['iterations']==1500 for x in profiles.values())
static=read(Path('/private/tmp/mech-related-invariant-capture-hoisting-runners-20261107-v2/static-analysis/comparison.json'))
history=read(D/'compiler-native-invariant-keys-and-movable-captures-20261107.json')
source_trials=[]
for cohort in ['source_cohort','compiler_cohort']:
    t=history[cohort]['trials']['mech-ir242-invariant-reads-4']
    source_trials.append(dict(report=history[cohort]['latency_report'],paired=t['paired_vs_selected'],passes=t['passes_joint_timing_gate']))
t=trials['mech-ir242-invariant-reads-4']
source_trials.append(dict(report=str(latency),paired=t['paired_vs_selected'],passes=t['passes_joint_timing_gate']))
assert [x['passes'] for x in source_trials]==[True,False,True]
source_totals={field:sum(x['paired'][field]['wins'] for x in source_trials) for field in ['post_json_seconds','wall_seconds']}
assert source_totals==dict(post_json_seconds=59,wall_seconds=76)
instructions=lambda variant:trials[variant]['resources']['post_json_measurement_median']['hardware']['instructions_retired']
report=dict(schema='mech.related-post-gen.invariant-capture-selection.v1',updated_utc=datetime.now(timezone.utc).isoformat(),
    selected_compiler='4cc266ae0d05c69432a4c15ffe99d5e586685647',selected_identity='IR242/work115/ABI41/image9',
    selected_source='ordered-combinadic-mask-reuse',selected_workers=4,selected_changed=False,
    integrated_PR973='2f7f38a4332c93cbaa6679a7138f7e0717fc9043',
    integration_compiler='a5a610cb1e8e6f6174d67f704d5255c411d38503',integration_identity='IR249/work115/ABI41/image9',
    candidate_compiler='f7237af541137f25daf19d41087e4905bd3737b8',candidate_identity='IR250/work115/ABI41/image9',
    candidate_branch='codex/related-invariant-capture-hoisting-20261107',candidate_selected=False,
    latency_report=str(latency),resource_report=str(resources),latency_seed=990250,resource_seed=9902507,
    exact_latency_outputs=372,exact_resource_outputs=91,all_first_and_adverse_samples_retained=True,
    resource_timings_excluded=True,matched_reference=a['matched_native_reference'],whole_reference=a['whole_process_reference'],
    trials=trials,paired_IR250_vs_IR249=pairs('mech-ir250-reuse-4','mech-ir249-reuse-4'),
    instruction_reduction_vs_IR249_percent=100*(1-instructions('mech-ir250-reuse-4')/instructions('mech-ir249-reuse-4')),
    instruction_increase_vs_selected_percent=100*(instructions('mech-ir250-reuse-4')/instructions('mech-ir242-reuse-4')-1),
    actual_selected_capture_reads=23,actual_ordinary_capture_reads=91,query_selected_capture_reads=0,query_ordinary_capture_reads=14,
    qualified_source_repeat=dict(cohorts=source_trials,total_rounds=93,paired_wins=source_totals,selected=False,
        explanation='Latest21/27 passes alone; earlier22/23 and16/26 are retained. Aggregate59/93 processing wins remains below the originally required21/31 win fraction. Do not promote by selecting only favorable cohorts.'),
    qualification=qualification,profiles=profiles,static_native_analysis=static,
    size_report=str(D/'size-native-invariant-captures-20261107.json'),
    decision='Keep selectedIR242. IR250 removes most blanket-clamp overhead but fails the joint speed gate. Newer unsigned-interval combination selects no predicates in this benchmark and all IR249 numeric function bytes equalIR248.',
    followup=dict(identity='IR251/work115/ABI41/image9',compiler='061822c2c4e8a4413d1d80afc16e47efc9c0d731',
        source='/private/tmp/mech-related-immutable-capture-views-compiler-20261107',
        build='/private/tmp/mech-related-immutable-capture-views-opt-20261107-v1',selected=False,
        hypothesis='Retain proved immutable capture view metadata as entry SSA values through callback refreshes so full addresses can become invariant.',
        status='build/qualification pending; no speed claim'),goal_complete=False)
out=D/'compiler-native-invariant-capture-selection-20261107.json'
out.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(report=str(out),candidate_selected=False,instruction_reduction_vs_IR249_percent=report['instruction_reduction_vs_IR249_percent'],instruction_increase_vs_selected_percent=report['instruction_increase_vs_selected_percent'],source_paired_wins_all93=source_totals)))
