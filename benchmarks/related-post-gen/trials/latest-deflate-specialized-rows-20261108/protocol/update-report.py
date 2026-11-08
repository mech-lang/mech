from pathlib import Path
import datetime, hashlib, json, shutil

P = Path(__file__).parent
C = Path('/private/tmp/mech-related-specialized-rows-confirmation-20261108')
R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
S = Path('/private/tmp/mech-related-latest-deflate-compiler-20261108')
D = R/'results'
def read(p): return json.loads(Path(p).read_text())
def write(p, d):
    p=Path(p); p.parent.mkdir(parents=True,exist_ok=True)
    p.write_text(json.dumps(d,indent=2)+'\n')
now=datetime.datetime.now(datetime.timezone.utc).isoformat()
a=read(P/'paired-analysis.json'); c=read(C/'paired-analysis.json')
candidate='mech-ir280-both-rows-4'; parent='mech-ir280-latest-deflate-4'
assert a['decisions'][candidate]['target_passed'] and c['decisions'][candidate]['target_passed']
assert all(x['paired'][parent]['post_json_seconds']['wins']==31 for x in [a['decisions'][candidate],c['decisions'][candidate]])
source=Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1/both-rows/build/source.mec')
source_hash=hashlib.sha256(source.read_bytes()).hexdigest()
assert source_hash=='97fc6c5a448aee096d19eb89839512974c724fee5efae5653ca88bed51b9c00a'
relative='experiments/subset3-specialized-rows4-5-native-4.mec'
for root in [R,S/'benchmarks/related-post-gen']:
    dest=root/relative;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,dest)
adoption=dict(adopted=True,scope='Working benchmark compiler and Mech source, independently confirmed; historical selectedIR242 and every raw trial decision remain unchanged.',
    reason='Two independent cohorts each31/31 faster processing pairs versus the identical current compiler/source control;17.47% fewer processing instructions in confirmation. Per user instruction, retain the measured whole-process/restoration/memory/code costs without a mandatory whole-process gate.',
    source_change=True,compiler_integration_alone_has_no_established_processing_gain=True,
    whole_process_paired_change_vs_IR278=0.04424350700404833,
    whole_process_paired_95_interval_vs_IR278=[0.005905048033880922,0.08289072923239726],
    original_frozen_raw_decisions_preserved=True,no_unchanged_failed_candidate_retry=True)
baseline=dict(compiler_source='ccfadbdf0af69f741237f5d1099a3c3eef79386a',identity='IR280/work116/ABI41/image11',
    source_variant='Checked4/5tag-row triple and quartet specialization, cached choose terms; original other-width loops',
    benchmark_source=relative,benchmark_source_sha256=source_hash,workers=4,
    runner='/private/tmp/mech-related-latest-deflate-runners-20261108-v1/both-rows/runner-aot',
    post_json_ms=1000*c['medians'][candidate]['post_json_seconds'],
    times_fastest_processing_Rust=c['medians'][candidate]['times_fastest_processing_Rust'],
    whole_ms=1000*c['medians'][candidate]['wall_seconds'],
    times_fastest_whole_Rust=c['medians'][candidate]['times_fastest_whole_Rust'],
    cohort='comparison-native-specialized-rows-confirmation-20261108-5000-m3.json',
    selection_reason=adoption)
r=read(D/'compiler-native-latest-deflate-integration-20261108.json')
r.update(updated_utc=now,remaining_qualification='complete',performance_measured=True,
    first_analysis=a,confirmation_analysis=c,prospective_rule=read(P/'prospective-rule.json'),
    confirmation_rule=read(C/'prospective-rule.json'),selected=True,selection_scope=adoption['scope'],
    working_baseline_adoption=adoption,current_working_baseline=baseline,
    related_outputs_exact=140,cohort_outputs_exact=2332,
    scalar_DEFLATE_rows_all_nontiming_and_allocation_fields_exact=133,
    raw_target_results_rewritten=False,source_hash=source_hash,maintained_source=relative,
    arithmetic_native_Object_text_exact_IR279=True,
    native_code={name:dict(code_bytes=read(Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1')/name/'summary.json')['code_bytes'],
        stack_bytes=read(Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1')/name/'summary.json')['stack_bytes'],
        native_functions=51,normalized_text_sha256=read(Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1')/name/'native-link-proof.json')['normalized_text_sha256'],
        metadata_bytes=(Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1')/name/'build/native-metadata.bin').stat().st_size,
        executable_bytes=(Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1')/name/'runner-aot').stat().st_size)
        for name in ['plain','triple-rows','quartet-rows','both-rows']},
    interpretation=[
        'Compiler integration alone leaves the arithmetic Object byte-exact to IR279 and does not establish a processing gain. Its first whole-process pairs regress2.4226% versus279,95%[+0.7162%,+4.0013%]. Preserve this adverse result.',
        'Both source changes combined win31/31 processing pairs initially and independently. Confirmation2.358833ms/1.554335x fastest eligible measured Rust1.517583ms; paired-9.527966%,95%[-10.115226%,-8.948174%] versus identical IR280 compiler control.',
        'Versus currentIR278 control, confirmation processing31/31 wins,paired-10.772636%,95%[-11.390024%,-9.872982%]. Whole paired+4.424351%,95%[+0.590505%,+8.289073%]; advance per explicit user phase-first policy.',
        'Confirmation processing49,401,422->40,770,303 instructions(-17.471398%); allocation count2945 stays equal,requested bytes+86,880 and peak additional requested heap+86,880. Restoration+465 allocations,+362,329 requested bytes and+349,609 peak additional heap.',
        'Native code889,768->1,077,480B; cumulative stack1552->2144B; metadata4,335,172->4,532,218B. PeakRSS+848KiB and footprint+432KiB versus matched compiler in confirmation; these observations are not complete causal attribution.',
        'The unchanged original-source LLVM comparison is2.379541ms/1.567981x in confirmation; new Cranelift/source wins23/31 pairs,paired-0.954243%,95%[-1.812165%,-0.274446%]. This is not a same-source backend superiority claim.',
        'Rust parity remains unmet. A fresh native/callback profile on the final original and specialized Objects is being built as a separate diagnostic; it cannot affect the completed promotion cohorts.'
    ])
r['first_failures'].append('First scalar regression link failed with errno28 disk exhaustion. All receipts retained;24 old qualified task-only executables archived losslessly and SHA-verified, then same-source link and all133 exact controls pass. No timing sample discarded.')
write(D/'compiler-native-latest-deflate-integration-20261108.json',r)
write(S/'benchmarks/related-post-gen/results/compiler-native-latest-deflate-integration-20261108.json',r)

policy=read(D/'compiler-working-baseline-policy-20261108.json')
policy.setdefault('working_baseline_history',[]).append(policy['working_baseline'])
policy['previous_working_baseline']=policy['working_baseline'];policy['working_baseline']=baseline;policy['updated_utc']=now
write(D/'compiler-working-baseline-policy-20261108.json',policy)
for filename in ['selected-speed-checkpoint-20261107.json','next-related-compiler-gaps-20261106.json',
                 'published-integrated-review-branch-20261106.json','compiler-branches-20261105.json']:
    d=read(D/filename);d['current_working_baseline']=baseline
    d['latest_report']='compiler-native-latest-deflate-integration-20261108.json'
    d['latest_verified_source_confirmation']='comparison-native-specialized-rows-confirmation-20261108-5000-m3.json'
    d['latest_source_compiler_gain_scope']='Working source/compiler adoption; historical IR242 selection unchanged; whole-process regression retained'
    write(D/filename,d);write(S/'benchmarks/related-post-gen/results'/filename,d)
write(S/'benchmarks/related-post-gen/results/compiler-working-baseline-policy-20261108.json',policy)
hpath=Path('/private/tmp/mech-related-invariant-native-lowering-current-handoff-20261107.json');h=read(hpath)
h['current_working_baseline']=baseline
h['active_latest_deflate_20261108'].update(confirmation_session=None,confirmation_complete=True,
    performance_measured=True,selected_working_source=True,qualification_complete=True,report=str(D/'compiler-native-latest-deflate-integration-20261108.json'))
h['current_handoff']='IR280/sourceccf integrates latest973a7c plus missing nullary dependency;415actual nativeCI,7upstream/3mask/8row/27image/4LLVM checks,140relatedoutputs and133exactscalar/DEFLATEallocation rows pass. Three source row specializations tested. Combined4/5row source independently confirmed and adopted:2.358833ms/1.554335x fastestRust;31/31pairs,-9.528%vsIR280plain,-10.773%vsIR278;17.471%fewerinstructions. Whole regression+4.424%paired vsIR278 retained.2332exactcohortoutputs. Runtime/native fresh profile build42836 active; no timing cohort live. Publish evidence and target remaining runtime/native costs;Rust goal unmet.'
h['live_own_jobs']=[dict(handle=42836,script='/private/tmp/mech-related-specialized-rows-profile-20261108/build.py',status='Diagnostic-only current runtime counter/profile build; all performance cohorts terminal')]
h['active_specialized_rows_profile']=dict(source='/private/tmp/mech-related-specialized-rows-profile-compiler-20261108',source_commit='a632dafd52ce8d0d4ad76076ae551fc7f099c81d',branch='codex/related-specialized-rows-profile-20261108',directory='/private/tmp/mech-related-specialized-rows-profile-20261108',build_session=42836,production_timing_eligible=False)
h['previous_goal_turn_classification']='progress: latest compiler integration plus missing nullary native foundation;415actualCI and133exactcontrols;140sourcequalifications;2332exactfreshcohortoutputs;sourcecombination independently saves9.53%processing/17.47%instructions and isadopted while retainingwhole/memory/codecosts. Fresh diagnostic profilebuild active.'
write(hpath,h)
print(json.dumps(dict(adopted=baseline,cohort_outputs_exact=2332,goal_complete=False)))
