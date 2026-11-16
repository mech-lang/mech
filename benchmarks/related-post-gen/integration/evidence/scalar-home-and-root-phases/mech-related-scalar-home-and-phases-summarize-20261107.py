from pathlib import Path
import json, statistics, subprocess, re

R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D = R/'results'
P = Path('/private/tmp/mech-related-scalar-home-phase-profile-20261107-v1')
S = Path('/private/tmp/mech-related-scalar-home-retirement-compiler-20261107')
U = Path('/private/tmp/mech-related-scalar-home-retirement-runners-20261107-v1')
T = Path('/private/tmp/mech-related-scalar-home-retirement-tests-20261107-v1')
C = Path('/private/tmp/mech-related-scalar-home-retirement-regressions-20261107-v1')
read = lambda p: json.loads(p.read_text())
write = lambda p, d: p.write_text(json.dumps(d, indent=2)+'\n')
lat = read(D/'comparison-native-scalar-home-retirement-randomized-20261107-5000-m3.json')
res = read(D/'comparison-native-scalar-home-retirement-resources-20261107-5000-m3.json')
for report, count in [(lat,310),(res,77)]:
    assert len(report['samples']) == count and report['execution_order']['mode']=='randomized'
    assert all(r['exit']==0 and r['exact_rust_output'] for r in report['samples'])
by = {}
for row in lat['samples']: by.setdefault(row['variant'],{})[row['sample']] = row
pairs = {}
for candidate, reference in [('mech-ir241-original-4','mech-ir240-original-4'),
                              ('mech-ir241-reuse-4','mech-ir240-reuse-4'),
                              ('mech-ir241-original-4','mech-ir238-original-4')]:
    pairs[candidate+' vs '+reference] = {}
    for metric in ['post_json_seconds','wall_seconds']:
        differences = [by[candidate][i][metric]-by[reference][i][metric] for i in range(31)]
        pairs[candidate+' vs '+reference][metric] = dict(rounds=31,wins=sum(v<0 for v in differences),
            median_paired_delta_seconds=statistics.median(differences))
test = read(T/'test-run.json')
assert test['expected_failures_exact'] and not test['new_failures'] and '861 passed; 2 failed' in test['summary'][0]
ci = read(T/'ci-selected-inventory-audit.json')
assert ci['current_selected_count']==366 and ci['all_selected_names_executed_and_passed_in_complete_gpu_run']
for kind, count in [('scalar',128),('one-kib',5)]:
    q = read(C/(kind+'-comparison.json'))
    assert q['rows']==count and q['all_nontiming_fields_equal_IR240']
assert read(Path('/private/tmp/mech-related-scalar-home-retirement-feature-check-20261107-v1/scalar-compiler.json'))['exit']==0
for label in ['after-4','reuse-4']:
    fixtures = read(U/label/'edges/edge-fixtures.json')
    assert len(fixtures)==31 and all(r['exit']==0 and r.get('exact_rust_output',r.get('exact_expected_output',False)) for r in fixtures)
profile = read(P/'captures.json')
helpers = read(P/'helper-captures.json')
assert profile['all14_outputs_exact'] and helpers['all7_outputs_exact']
assert profile['native_objects_unchanged_and_actual_linked_bytes_audited']
assert all(r['profile']['site_overflow_count']==0 for r in helpers['rows'])
phase = {}
for label in ['before','after']:
    rows = [r for r in profile['rows'] if r['label']==label]
    phase[label] = dict(invocations=7,
        post_json_wall_seconds=statistics.median(r['profile']['post_json_seconds'] for r in rows),
        parallel_run_wall_seconds=statistics.median(sum(w['seconds'] for w in r['profile']['parallel_run_windows']) for r in rows),
        outside_parallel_wall_seconds=statistics.median(r['profile']['post_json_seconds']-sum(w['seconds'] for w in r['profile']['parallel_run_windows']) for r in rows),
        parallel_instructions=statistics.median(r['parallel_hardware_sum']['instructions_retired'] for r in rows),
        outside_parallel_instructions=statistics.median(r['outside_parallel_hardware']['instructions_retired'] for r in rows),
        parallel_cpu_seconds=statistics.median(r['parallel_hardware_sum']['user_cpu_seconds']+r['parallel_hardware_sum']['system_cpu_seconds'] for r in rows),
        outside_parallel_cpu_seconds=statistics.median(r['outside_parallel_hardware']['user_cpu_seconds']+r['outside_parallel_hardware']['system_cpu_seconds'] for r in rows),
        root_canonical_calls=statistics.median(r['profile']['canonical_root_outside_parallel']['calls'] for r in rows),
        root_canonical_inclusive_wall_seconds=statistics.median(r['profile']['canonical_root_outside_parallel']['inclusive_seconds'] for r in rows))
protocol = (S/'hosts/gpu/src/native_artifact/program/protocol/mod.rs').read_text()
names = {0:'Library',1:'Move',2:'Collection',63:'Output_or_unclassified'}
for name, offset in re.findall(r'Self::(\w+) => u64::MAX - (\d+)',protocol): names[3+int(offset)] = name
rows = helpers['rows']
detail = {}
for field in ['root_helper_counts','worker_helper_counts','root_helper_ns_by_operation',
              'root_helper_ns_by_site','root_native_tail_ns_by_header','root_native_segment_ns_by_start',
              'root_other_ns_by_site','root_other_ns_by_operation']:
    slots = sorted({i for r in rows for i,v in r['profile'][field]})
    detail[field] = [dict(key=i,name=names.get(i) if 'operation' in field or 'counts' in field else None,
        median=statistics.median(dict(r['profile'][field]).get(i,0) for r in rows)) for i in slots]
detail['native_tail_seconds'] = statistics.median(sum(v for i,v in r['profile']['root_native_tail_ns_by_header'])*1e-9 for r in rows)
detail['helper_outside_parallel_seconds'] = statistics.median(sum(v for i,v in r['profile']['root_helper_ns_by_operation'] if names.get(i)!='ParallelRun')*1e-9 for r in rows)
detail['observer_gaps_seconds'] = statistics.median(r['profile']['post_json_seconds']-sum(v for key in ['root_helper_ns_by_operation','root_native_tail_ns_by_header','root_native_segment_ns_by_start','root_other_ns_by_site'] for i,v in r['profile'][key])*1e-9 for r in rows)
rust_phases=[]
for row in lat['samples']:
    if row['variant']=='rust-mech-control-u32-casts':
        for line in row['stdout'].splitlines():
            if line.startswith('{') and 'diagnostic_phases' in json.loads(line): rust_phases.append(json.loads(line)['diagnostic_phases'])
assert len(rust_phases)==31
rust = {k:statistics.median(r[k] for r in rust_phases) for k in rust_phases[0] if k.endswith('_seconds')}
summary = dict(schema='mech.related-post-gen.scalar-home-and-root-phases.v1',
    compiler='8f6516cd93410c75df18648ed10b9d335ca5311b',identity='IR241/work115/ABI41/image9',
    parent='deebf08df690e37ddf763635d70fd25aaf0bedb4',source_files=len(read(Path('/private/tmp/mech-related-scalar-home-retirement-opt-20261107-v2/source-inventory.json'))['files']),
    change='Compiler-local dirty masks elide repeated physical scalar-home zero stores. Entry and every resumable first-change helper conservatively mark all homes dirty. No runtime state, source operation, price, ABI or admission rule changes.',
    latency_summary=lat['summary'],resource_summary=res['summary'],paired=pairs,
    matched_reference=lat['matched_native_reference'],whole_process_reference=lat['whole_process_reference'],
    sizes=read(D/'size-native-scalar-home-retirement-20261107.json'),native_selection=read(U/'selection-summary.json'),
    qualification=dict(gpu_passed=861,inherited_failures=test['failed_tests'],ci_selected_passed=366,
        scalar_all_nontiming_exact_IR240=128,compression_all_nontiming_exact_IR240=5,
        scalar_only_passed=True,source_fixtures_exact=62,cohort_outputs_exact=387,
        full_AOT_loader_finite_runtime_adoption_qualification_not_run=True,
        unchanged_Core_Engine_sources_reuse_IR238_393_Core_and_289_Engine_checks=True),
    selected=False,applied_to_public_compiler=False,selected_speed_checkpoint='IR238 original/four',goal_complete=False,
    reason_unselected='Original-source wins22/31 processing but14/31 whole-process pairs against selectedIR238; does not pass the21/31 both-boundaries promotion gate.',
    diagnostic_phase_profile=phase,detailed_root_profile=detail,rust_existing_phase_timer_medians=rust,
    diagnostic_outputs_exact=21,
    new_priority='Serial preprocessing/index construction plus immutable runtime preparation. The parallel stage is already about0.45ms; outside it remains about2.44ms and39million instructions. More query workers cannot close that serial gap.',
    source_pins=dict(before_profile=subprocess.check_output(['git','rev-parse','HEAD'],cwd='/private/tmp/mech-related-phase-attribution-before-compiler-20261107',text=True).strip(),
        after_phase_profile=subprocess.check_output(['git','rev-parse','HEAD^'],cwd='/private/tmp/mech-related-phase-attribution-after-compiler-20261107',text=True).strip(),
        after_helper_profile=subprocess.check_output(['git','rev-parse','HEAD'],cwd='/private/tmp/mech-related-phase-attribution-after-compiler-20261107',text=True).strip()),
    limitations=['All diagnostic observations are excluded from benchmark comparisons; first and adverse primary samples remain.',
        'Parallel hardware windows are disjoint root-thread intervals and include all process threads. Outside counters and wall intervals are derived per invocation before taking medians.',
        'Detailed native spans are root wall intervals between helpers, associated with the current admitted loan. They are not sampled CPU-time shares.',
        'Canonical timers are inclusive and may overlap. The observer adds work; no zero-overhead or subtraction claim.',
        'Rust phase timers come from the ordinary31-round latency cohort; their phase boundaries differ from Mech helper scopes and are not substituted for headline timing.',
        'Xcode Time Profiler is unavailable; SDK-verified process counters and audited identical native Objects provide the diagnostic attribution.',
        'Runtime vector addressing already exists in numeric_words::linear_address; do not reimplement it as a new optimization.'],
    next_steps=['Inspect the actual emitted guards/bookkeeping in root native headers256,801,572,1166 and459; connect them to preprocessing operations.',
        'Extend bounded funding only where current-loop completion and actual prefix/error debits can be preserved. Existing complete-child prepayment is phase/enum-specific; do not duplicate it or deny early on insufficient funding.',
        'Inspect immutable call/descriptor planning and restoration, preserving current-value admission and same-Object/catalog validation.',
        'Investigate missing readonly/alias facts for retained numeric input views only after checking the real owner lifetime and mutation contracts; no unsafe disjointness assumption.'])
write(D/'compiler-native-scalar-home-and-root-phases-20261107.json',summary)
print(json.dumps(dict(status='verified',source_files=summary['source_files'],cohort_outputs=387,profile_outputs=21,selected=False,phase=phase,detail_native=detail['native_tail_seconds'],detail_helpers=detail['helper_outside_parallel_seconds'])))
