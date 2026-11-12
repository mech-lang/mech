from pathlib import Path
import hashlib, json, statistics

R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D = R / 'results'
read = lambda p: json.loads(p.read_text())
write = lambda p, d: p.write_text(json.dumps(d, indent=2) + '\n')
prefix = 'canonical-schema-sinks'
U = Path('/private/tmp/mech-related-' + prefix + '-runners-20261107-v1')
T = Path('/private/tmp/mech-related-' + prefix + '-tests-20261107-v1')
C = Path('/private/tmp/mech-related-' + prefix + '-regressions-20261107-v1')

def cohorts(stem, nlat, nres):
    lat = read(D / ('comparison-native-' + stem + '-randomized-20261107-5000-m3.json'))
    res = read(D / ('comparison-native-' + stem + '-resources-20261107-5000-m3.json'))
    for report, count in [(lat, nlat), (res, nres)]:
        assert len(report['samples']) == count
        assert all(x['exit'] == 0 and x['exact_rust_output'] for x in report['samples'])
        assert report['execution_order']['mode'] == 'randomized'
    return lat, res

def pairs(lat, selected):
    by = {}
    for r in lat['samples']:
        by.setdefault(r['variant'], {})[r['sample']] = r
    result = {}
    for candidate, reference in selected:
        row = {}
        for metric in ['post_json_seconds', 'wall_seconds']:
            values = [by[candidate][i][metric] - by[reference][i][metric] for i in by[candidate]]
            row[metric] = dict(rounds=len(values), wins=sum(v < 0 for v in values),
                               median_paired_delta_seconds=statistics.median(values))
        result[candidate + ' vs ' + reference] = row
    return result

lat, res = cohorts('canonical-schema-sinks', 434, 105)
blat, bres = cohorts('boolean-insertion-combination', 341, 84)
tests = read(T / 'test-run.json')
assert tests['expected_failures_exact'] and not tests['new_failures']
assert '853 passed; 2 failed' in tests['summary'][0]
ci = read(T / 'ci-selected-inventory-audit.json')
assert ci['current_selected_count'] == 358 and ci['all_selected_names_executed_and_passed_in_complete_gpu_run']
qual = read(U / 'qualification-summary.json')
assert qual['loader_fixtures_exact'] == 558 and qual['decoded_bundle_files_exact'] == 65
assert qual['finite_denials_exact'] == 48 and qual['finite_small_full_successes_exact'] == 12
assert qual['forged_frontier_rejections'] == 16
scalar = read(C / 'scalar-comparison.json')
compression = read(C / 'one-kib-comparison.json')
assert scalar['rows'] == 128 and scalar['all_semantic_nontiming_fields_equal_IR236']
assert scalar['allocation_requests_never_regress'] and scalar['requested_bytes_never_regress']
assert compression['rows'] == 5 and compression['all_nontiming_fields_equal_IR236']
runtime = read(Path('/private/tmp/mech-related-' + prefix + '-runtime-check-20261107-v1/summary.json'))
assert runtime['status'] == 'passed'
assert read(Path('/private/tmp/mech-related-' + prefix + '-feature-check-20261107-v1/scalar-compiler.json'))['exit'] == 0
old = read(Path('/private/tmp/mech-related-' + prefix + '-old-objects-20261107-v1/summary.json'))
assert old['fresh_old_objects'] == 2

ap = {}
for label, directory, expected in [
    ('IR236', 'local-facts', {'post-json': (3858, 8013126), 'restore': (36437, 13094307)}),
    ('IR238', 'canonical-schema-sinks', {'post-json': (3758, 8012356), 'restore': (26837, 12830999)}),
]:
    profile = read(Path('/private/tmp/mech-related-' + directory + '-allocation-profiles-20261107-v1/summary.json'))
    assert profile['exact_outputs'] and profile['counts_and_bytes_match_current_resource_cohort']
    ap[label] = {k: {a: v[a] for a in ['requests', 'requested_bytes', 'roles', 'unattributed_requests',
                                      'executable_sha256', 'raw_trace_sha256']} for k, v in profile['variants'].items()}
    for phase, counts in expected.items():
        assert tuple(ap[label][phase][k] for k in ['requests', 'requested_bytes']) == counts
        assert ap[label][phase]['unattributed_requests'] == 0

paired = pairs(lat, [
    ('mech-ir238-original-4', 'mech-ir236-original-4'),
    ('mech-ir238-reuse-4', 'mech-ir236-reuse-4'),
    ('mech-ir238-original-4', 'mech-selected-ir208-original-4'),
    ('mech-ir238-reuse-4', 'mech-ir238-original-4'),
    ('mech-ir238-reuse-rust-insert-4', 'mech-ir238-reuse-4'),
])
gate = paired['mech-ir238-original-4 vs mech-selected-ir208-original-4']
assert all(gate[m]['wins'] >= 21 for m in ['post_json_seconds', 'wall_seconds'])
BT = Path('/private/tmp/mech-related-branch-insertion-boolean-combination-tests-20261107-v1')
bt = read(BT / 'test-run.json')
assert bt['expected_failures_exact'] and not bt['new_failures'] and '855 passed; 2 failed' in bt['summary'][0]
assert read(BT / 'ci-selected-inventory-audit.json')['current_selected_count'] == 360
BC = Path('/private/tmp/mech-related-branch-insertion-boolean-combination-regressions-20261107-v1')
assert all(read(BC / (name + '-comparison.json'))['all_nontiming_fields_equal_IR238'] for name in ['scalar', 'one-kib'])
summary = dict(
    schema='mech.related-post-gen.canonical-schema-and-boolean-ablations.v1',
    active_compiler='30a5501b0401cd96e73507139c889c720f0968c2', active_identity='IR238/work115/ABI41/image9',
    source_commit_with_helper_integrity_fix='6e599d0ac7ed64896998c017dadb6e0acfcdb6ef',
    incoming_973='40bba44acd7a60e28e3378dab012ee9d50007c66',
    unselected_compiler='cddea543f849bc329a3988c7528406bae61ee199', unselected_identity='IR239/work115/ABI41/image9',
    latency_summary=lat['summary'], matched_reference=lat['matched_native_reference'],
    whole_process_reference=lat['whole_process_reference'], paired=paired, resource_summary=res['summary'],
    size_metrics=read(D / 'size-native-canonical-schema-sinks-20261107.json'), allocation_profiles=ap,
    qualification=qual | dict(gpu_passed=853, inherited_failures=tests['failed_tests'], ci_selected_names_passed=358,
        core_passed=393, relevant_engine_passed=289, scalar_semantic_control_rows_exact=128,
        scalar_physical_allocations_never_increase=True, compression_all_nontiming_rows_exact=5,
        runtime_only_and_unmodified_generated_app_passed=True, scalar_only_passed=True,
        genuine_old_objects_rejected=2, additional_rust_insertion_loader_fixtures=124,
        additional_rust_insertion_decoded_files=13),
    boolean_combination=dict(latency_summary=blat['summary'], resource_summary=bres['summary'],
        matched_reference=blat['matched_native_reference'], whole_process_reference=blat['whole_process_reference'],
        paired=pairs(blat, [('mech-ir239-' + n, 'mech-ir238-' + n) for n in ['original-4', 'reuse-4', 'reuse-rust-insert-4']]),
        size_metrics=read(D / 'size-native-boolean-insertion-combination-20261107.json'),
        gpu_passed=855, inherited_failures=bt['failed_tests'], ci_selected_passed=360,
        control_rows_all_nontiming_exact_IR238=133, exact_source_fixtures=93,
        full_loader_finite_runtime_qualification_not_run=True, selected=False, applied_to_active_compiler=False),
    decision=dict(selected_speed_checkpoint='IR238 original source, four workers',
        previous_selected_speed_checkpoint='IR208 original source, four workers',
        selection_gate='At least21/31 paired wins in both continuous post-JSON and whole-process time, plus complete qualification.',
        original_source_selected=True, ordered_mask_reuse_source_selected=False,
        rust_insertion_source_selected=False, boolean_lowering_applied=False, goal_complete=False),
    cohort_outputs_exact=964,
    findings=[
        'Canonical schema/dimension children encode into one sink; table keys reuse the exact bytes already built for ordering/dedup. Canonical bytes and SHA keys match the independent previous encoder.',
        'The schema change preserves all emitted native function text. Startup requests fall36437 to26837 for mask reuse; processing requests fall3858 to3758, while processing instructions remain effectively flat.',
        'IR238 original/four passes the retained promotion gate against selectedIR208. Processing wins31/31 and whole-process wins21/31. The processing gain includes earlier compiler work; do not attribute it all to schema encoding.',
        'Mask reuse is faster on ordered input but still directs unordered unique rows to the exact slow fallback. It remains unselected pending restoration of that input fast-path scope.',
        'Rust-style insertion adds query code and instructions; the Boolean combination does not demonstrate a reliable same-source overall win. Both remain unselected.',
        'A real disk-full rust-objcopy failure produced exit0 and a corrupt file. The host builder now validates failed stripping and thinAArch64 Mach-O headers; the failed artifact and successful requalification are retained.',
    ], limitations=[
        'All first/adverse samples retained; latency and resources are separate randomized cohorts with complete fresh processes.',
        'All four allocation profiles reproduce exact counts/bytes and exact outputs. Inclusive stacks overlap and are not CPU-time percentages.',
        'The selected candidate remains slower than the fastest same-cohort external Rust on both boundaries. Parity is unmet.',
        'Two inherited GPU failures remain. HostedCI is not claimed green; no merge requested; raw executable archives remain local.',
    ],
    next_targets=['Producer-derived immutable posting/index ranges and bounded native-tree funding with the original checked fallback.',
                  'Share or borrow immutable BoundCall, CallMemoryPlan and resolved descriptor metadata on first use.',
                  'Prune scalar-home retirement stores only with a complete path/materialization proof.'])
write(D / 'compiler-native-canonical-schema-and-boolean-ablations-20261107.json', summary)
print(json.dumps(dict(status='verified', cohort_outputs=964, selected='IR238 original/four', goal_complete=False)))
