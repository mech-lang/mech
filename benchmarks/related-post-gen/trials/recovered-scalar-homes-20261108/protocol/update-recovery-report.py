from pathlib import Path
import datetime, hashlib, json

P = Path(__file__).parent
R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D = R / 'results'
S = Path('/private/tmp/mech-related-recovered-scalar-homes-compiler-20261108')
H = Path('/private/tmp/mech-related-invariant-native-lowering-current-handoff-20261107.json')
now = datetime.datetime.now(datetime.timezone.utc).isoformat()
def read(p): return json.loads(Path(p).read_text())
def write(p, d):
    p = Path(p); p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(d, indent=2) + '\n')

a = read(P / 'paired-analysis.json')
rule = read(P / 'prospective-rule.json')
n = 'mech-ir279-scalar-home-masks-4'
parent = rule['matched_parent']
d = a['decisions'][n]
q = read(D / 'compiler-native-recovered-scalar-homes-20261108.json')
q.update(updated_utc=now, source_commit='3ad8bb9210d322124eb5df913f5652c10336cb4c',
         full_CI_and_remaining_qualification='complete; all required qualification passed',
         performance_measured=True, analysis=a, prospective_rule=rule,
         retained_for_composition=True, selected=False, goal_complete=False,
         status='Verified processing instruction saving; elapsed-time superiority remains inconclusive. Retained for compiler composition; IR278 remains the latency control. No whole-process gate or unchanged favorable retry.',
         production_backend='Cranelift', LLVM_mandatory=False, default_dependencies_changed=False,
         qualification=dict(native_CI_names_executed=408, old_405_names_retained=True,
                            recovered_mask_and_finite_ablation_checks_passed=3,
                            packed_row_and_route_checks_passed=8,
                            GPU_image_and_caller_checks_passed=27,
                            optional_LLVM_checks_explicitly_executed=4,
                            scalar_DEFLATE_rows_all_nontiming_and_allocation_fields_exact=133,
                            related_outputs_exact=35, cohort_outputs_exact=1109,
                            no_build_test_profile_overlap=True),
         native_code=dict(code_bytes=889768, parent_code_bytes=886344,
                          stack_bytes=1552, parent_stack_bytes=1536,
                          native_functions=51,
                          normalized_text_sha256='fd33588597e17028d3c45709626a771545e246cc48e1a05453645a8eeb5f3960'),
         metadata_bytes=4335172, parent_metadata_bytes=4335172,
         executable_bytes=Path('/private/tmp/mech-related-recovered-scalar-homes-runners-20261108-v1/plain/runner-aot').stat().st_size,
         interpretation=[
             'Processing instructions 51,147,313 -> 49,520,546 (-1,626,767; -3.18055%) against unchanged IR278 in the same resource cohort.',
             'Processing medians 2.912833 -> 2.881333 ms; 20/31 paired wins, paired median -0.84666%, two-sided 95% interval [-1.38028%, +0.24786%]. Timing superiority and regression are both unestablished.',
             'Fastest eligible processing Rust in this latency cohort is 1.631083 ms; IR279 is 1.766515x and unchanged IR278 is 1.785828x. Cross-cohort clock variation is not a compiler gain.',
             'Whole-process medians 37.220791 -> 37.352500 ms; paired median -0.92980%, 95% [-3.08132%, +2.00840%]. Both observations retained; whole process is not a required gate.',
             'Processing and restore allocation requests, requested bytes and peak additional requested heap remain equal. Native code grows 3424 bytes; cumulative native stack grows 16 bytes.',
             'This recovers a genuine missing native optimization. Retain its instruction saving and composition opportunity without calling an inconclusive elapsed-time result a proven speedup.'
         ])
write(D / 'compiler-native-recovered-scalar-homes-20261108.json', q)

audit = read(D / 'optimization-recovery-audit-20261108.json')
audit['updated_utc'] = now
entry = next(x for x in audit['prioritized_entries'] if x['id'] == 'IR241')
entry['historical_gain'] = entry['gain']
entry.update(current='Recovered and fully qualified as IR279; retained for composition, not a proven timing promotion',
             gain='Fresh same-cohort processing instructions 51147313->49520546 (-3.18055%). Processing 2.912833->2.881333ms,20/31 paired wins;95% paired change[-1.38028%,+0.24786%],so elapsed superiority is inconclusive.',
             action='Keep recovered native masks available for composition;408 actual native CI checks,35 related outputs and133 exact scalar/DEFLATE allocation rows pass. No mandatory whole-process gate.',
             current_report='compiler-native-recovered-scalar-homes-20261108.json')
extra = [
    ('IR211', 'Omit proved zero append-frontier guards', 'compiler-native-zero-append-frontiers-candidate-20261106.json',
     'Historical processing instructions -1.45%; confirmation10/15 processing wins but6/15 whole wins versus immediate parent.',
     'Already inherited: ancestor7661fb381e28171222858f685d1a0efcf17f29ec; ZeroAppendFrontiers remains enabled in readonly_numeric.rs.'),
    ('IR221', 'Borrow compiler-only frame metadata from immutable native records', 'compiler-native-borrowed-frame-metadata-candidate-20261106.json',
     'Historical restore allocations58019->50828(-12.394%), instructions-2.131%; mixed whole timing versus then-selectedIR208.',
     'Already inherited: ancestorc34338953b316c48d78c2f7f0c2ef0ae19bd6f29; numeric_frame.rs retains genuine borrowed storage and private OnceLock decoding.'),
    ('IR227', 'Earlier latest-DEFLATE compiler integration', 'compiler-native-latest-deflate-integration-screen-20261106.json',
     'Extended original-source processing31/31 wins versus old selected compiler; whole19/31,below old21/31 rule.',
     'Already inherited: ancestordae0b9b6c33b39ce55104880660501929ee07b6b; distinguish this from newest PR973 changes not yet in the frozen compiler.'),
]
for ident, feature, report, gain, current in extra:
    if not any(x['id'] == ident for x in audit['prioritized_entries']):
        audit['prioritized_entries'].append(dict(id=ident,feature=feature,report=report,
            rule_casualty=True,current=current,gain=gain,action='Retained; do not count this historical gain a second time.',
            double_count=True,report_sha256=hashlib.sha256((D/report).read_bytes()).hexdigest()))
audit['active_test'] = dict(identity='IR279/work116/ABI41/image11',completed=True,
    report='compiler-native-recovered-scalar-homes-20261108.json',processing_instruction_gain_percent=3.18055222177556,
    elapsed_time_superiority_established=False,retained_for_composition=True,next='Assess paired wide clears with masks, compact-visit/grouped-reader ablation and newest PR973 native geometry/address staging; keep adverse standalone results.')
audit['prioritized_count'] = len(audit['prioritized_entries'])
write(D / 'optimization-recovery-audit-20261108.json', audit)
for filename in ['compiler-native-recovered-scalar-homes-20261108.json','optimization-recovery-audit-20261108.json']:
    write(S/'benchmarks/related-post-gen/results'/filename, read(D/filename))

h = read(H)
h['updated_utc'] = now
h['live_own_jobs'] = []
h['latest_own_jobs'] = []
h['active_recovered_scalar_homes'].update(performance_measured=True,cohort_session_id=None,
    first_cohort_finished=True,analysis=str(P/'paired-analysis.json'),retained_for_composition=True,
    selected=False,processing_instruction_delta=-1626767,processing_instruction_gain_percent=3.18055222177556,
    post_json_ms=2.881333,times_fastest_processing_Rust=a['medians'][n]['times_fastest_processing_Rust'],
    elapsed_time_superiority_established=False)
h['optimization_recovery_audit'].update(prioritized=len(audit['prioritized_entries']),
    key_unrecovered='IR241 recovered as IR279 with verified3.18055% processing-instruction saving,elapsed inconclusive. IR270 wide clears and IR275 table slots remain composition hypotheses; their unfavorable old processing results are retained.')
h['current_handoff'] = 'IR278 remains the working timing control. IR279 recovered scalar-home masks fully qualified and measured:1109 exact cohort outputs,408 actual native CI,35 qualification outputs and133 exact scalar/DEFLATE allocation rows. Processing instructions-3.18055%,2.881333ms/1.766515x fastest eligible same-cohort Rust; paired timing-0.84666%,95%[-1.38028%,+0.24786%] inconclusive. Retained for composition without timing promotion; whole gate optional. No live jobs. Publish evidence; then combine compatible native ideas/integrate latest PR973. Rust goal not achieved.'
h['previous_goal_turn_classification'] = 'progress: audited120 historical reports/18 prioritized ideas, recovered and fully qualified missing native scalar-home masks, completed first899 latency+210 resource samples with all1109 exact; verified1.627M fewer processing instructions, elapsed result inconclusive and retained.'
write(H,h)
print(json.dumps(dict(report_updated=True,audited_reports=audit['reports_inventoried'],prioritized=len(audit['prioritized_entries']),retained_for_composition=True,timing_promotion=False,live_jobs=[])))
