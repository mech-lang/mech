from pathlib import Path
import datetime, gzip, hashlib, importlib.util, json, shutil, statistics, subprocess

S=Path('/private/tmp/mech-related-deflate-exhaustion-proof-integration-compiler-20261106')
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
U=Path(__file__).parent
PREFIX='/private/tmp/mech-related-deflate-exhaustion-proof-integration-'
O=Path(PREFIX+'opt-20261106-v1'); T=Path(PREFIX+'tests-20261106-v1')
C=Path(PREFIX+'regressions-20261106-v1'); RT=Path(PREFIX+'runtime-check-20261106-v1')
Q=Path(PREFIX+'feature-check-20261106-v1'); OLD=Path(PREFIX+'old-objects-20261106-v1')
I=S/'benchmarks/related-post-gen/integration'; E=I/'evidence'; X=E/'ordered-source-experiments'
X.mkdir(exist_ok=True)
read=lambda p:json.loads(p.read_text())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
write=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')
compiler='c4649860a5e445c1ce1bdec8503870e8a455da17'

build=read(O/'build.json')
assert len(build['stages'])==12 and all(s['exit']==0 for s in build['stages'])
spec=importlib.util.spec_from_file_location('builder',R/'build-compiler.py')
helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
assert helper.source_inventory(S)==read(O/'source-inventory.json')['files']
tests=read(T/'test-run.json')
assert tests['expected_failures_exact'] and not tests['new_failures']
assert '844 passed; 2 failed' in tests['summary'][0]
assert read(T/'focus-ordered_reduction.json')['exit']==0
for kind,count in [('scalar',128),('one-kib',5)]:
    rows=read(C/(kind+'-comparison.json'))
    assert rows['rows']==count and rows['all_nontiming_fields_equal_IR231'] and not rows['differences']
assert read(U/'qualification-summary.json')['all_completed_checks_asserted']
assert read(RT/'summary.json')['status']=='passed'
assert read(Q/'scalar-compiler.json')['exit']==0
assert read(OLD/'summary.json')['fresh_old_objects']==3
native=read(U/'native-text-equivalence.json')
assert len(native)==4 and all(x['complete_native_text_byte_identical'] for x in native.values())

# Preserve the previous compiler's exact pins before replacing current ones.
for name in ['compiler-build.json','source-inventory.json']:
    previous=subprocess.check_output(['git','show','6c073d2958ae98a8012d6027a5e353a5412114b6:benchmarks/related-post-gen/integration/'+name],cwd=S)
    (E/('ir234-retained-ir231-'+name)).write_bytes(previous)
    shutil.copyfile(O/('build.json' if name=='compiler-build.json' else name),I/name)

def copy(source,name):shutil.copyfile(source,E/('ir234-'+name))
for root,label in [(T,'gpu'),(C,'control'),(Q,'feature'),(OLD,'old-object')]:
    for p in sorted(root.iterdir()):
        if p.is_file() and p.suffix in ['.json','.log','.py','.rs']:
            copy(p,label+'-'+p.name+('.txt' if p.suffix=='.rs' else ''))
for name in ['summary.json','runtime-first10.json','runtime-full5000.json','managed-main-build.json','generate-standard-app.json','scalar-runtime.json','program-runtime.json','build-and-check.py']:
    copy(RT/name,'runtime-'+name)
for p in sorted(U.glob('*')):
    if p.is_file() and p.suffix in ['.json','.py'] and p.name!='publish-evidence.py':copy(p,p.name)
copy(Path(__file__),'publish-evidence.py')
for family in ['budget','compact-budget']:
    copy(U/family/'summary.json',family+'-summary.json')
    for loader in ['bound','independent','full-catalog']:
        for size in ['first-10','full-5000']:
            copy(U/family/loader/('success-rust-'+size+'-run.json'),family+'-'+loader+'-'+size+'-success.json')
for p in sorted(U.glob('edges-*/edge-fixtures.json'))+sorted(U.glob('*/edges/edge-fixtures.json')):
    copy(p,'edges-'+str(p.relative_to(U)).replace('/','-'))
for name in ['host-build-run.json.failed-disk-full','runner-aot.build.json.failed-disk-full','host-build-run.json','host-build-retry-run.json','runner-aot.build.json','audit-run.json']:
    copy(U/'compact-4-full-catalog'/name,'repaired-last-loader-'+name)
copy(U/'frontier-image-probe/run.json','frontier-image-run.json')

cohorts={}; paired={}; resources={}; sources={}
for label,stem,expected in [('ordered','ordered-subsets-source',217),('fused','ordered-subsets-fused-source',248),('reused-masks','ordered-subsets-reused-masks-source',279)]:
    p=Path('/private/tmp/mech-related-'+stem+'-20261106-v1')
    for name in ['scope.json','native-link-audit.json','runner-aot.build.json','compile-run.json','edge-run.json','build-and-check.py']:
        q=p/name
        if q.exists():shutil.copyfile(q,X/(label+'-'+name))
    shutil.copyfile(p/'source.mec',X/(label+'.mec'))
    shutil.copyfile(p/'source.mec',S/'benchmarks/related-post-gen/experiments'/('related-tiered-ordered-subsets-'+('four-workers' if label=='ordered' else label+'-four-workers')+'.mec'))
    edges=read(p/'edges/edge-fixtures.json');assert len(edges)==31
    assert all(x['exit']==0 and x.get('exact_rust_output',x.get('exact_expected_output',False)) for x in edges)
    shutil.copyfile(p/'edges/edge-fixtures.json',X/(label+'-edge-fixtures.json'))
    metadata=read(p/'build/metadata.json')
    assert metadata['typed_ir_version']==234
    write(X/(label+'-native-selection.json'),{key:metadata[key] for key in ['typed_ir_version','code_bytes','stack_bytes','compiled_numeric_functions','numeric_tree_candidates','captured_numeric_candidates','readonly_numeric_candidates']})
    sources[label]=dict(source_sha256=sha(p/'source.mec'),numeric_functions=len(metadata['compiled_numeric_functions']),code_bytes=metadata['code_bytes'],entry_stack_bytes=metadata['stack_bytes'],scope=read(p/'scope.json'))
    for phase,count in [('latency',expected),('resources',expected//31*7+7)]:
        suffix='randomized' if phase=='latency' else 'resources'
        path=R/'results'/('comparison-'+stem+'-'+suffix+'-20261106-5000-m3.json')
        raw=path.read_bytes();d=json.loads(raw)
        assert len(d['samples'])==count and all(x['exit']==0 and x['exact_rust_output'] for x in d['samples'])
        assert d['execution_order']['mode']=='randomized'
        (X/(label+'-'+phase+'.json.gz')).write_bytes(gzip.compress(raw,mtime=0))
        cohorts[label+'-'+phase]=dict(samples=count,execution_order=d['execution_order'],matched_reference=d['matched_native_reference'],whole_process_reference=d['whole_process_reference'],summary=d['summary'])
        if phase=='latency':
            by={}
            for row in d['samples']:by.setdefault(row['variant'],{})[row['sample']]=row
            pairs={}
            for candidate in ['mech-ordered-4','mech-fused-4','mech-reuse-4']:
                if candidate not in by:continue
                pairs[candidate]={}
                for metric in ['post_json_seconds','wall_seconds']:
                    differences=[by[candidate][i][metric]-by['mech-original-4'][i][metric] for i in by[candidate]]
                    pairs[candidate][metric]=dict(wins=sum(x<0 for x in differences),rounds=len(differences),median_paired_delta_seconds=statistics.median(differences))
            paired[label]=pairs
        else:
            resources[label]={}
            for name,item in d['summary'].items():
                if 'post_json_measurement_median' not in item:continue
                measurement=item['post_json_measurement_median']
                resources[label][name]=dict(post_json=measurement,whole_process=item['process_resources_median'],restore=item.get('stage_measurements_median',{}).get('restore'))
summary=dict(schema='mech.related-post-gen.ordered-source-experiments.v1',compiler=compiler,identity='IR234/work115/ABI41/image9',sources=sources,cohorts=cohorts,paired=paired,resources=resources,
    exact_cohort_outputs=sum(c['samples'] for c in cohorts.values()),fixture_outputs=93,selected=False,goal_complete=False,
    interpretation=['Separate ordering scan cancels simpler subset keys:13/31 processing wins in the first cohort.',
        'Fusing uniqueness/order gives21/31 post-JSON and23/31 full-process wins in its first cohort; processing benefit does not repeat in the mask-reuse cohort.',
        'Mask reuse exposes a native-tree currentness proof gap. Iteration3 declines: numeric tree child returns an older array version. Keep exact fallback and fix proof rather than removing semantics.',
        'All variants preserve original output tag order and exact fallback. Unordered unique rows lose the former subset fast path; all remain unselected.',
        'All randomized schedules and samples, including slow first samples, are retained. Separate instrumented counters are not latency acceptance results.'])
write(X/'summary.json',summary)
write(R/'results/compiler-ordered-source-experiments-20261106.json',summary)
archive=read(R/'results/native-deflate-exhaustion-proof-integration-20261106/archive-index.json')
assert archive['verified_files']==3177
shutil.copyfile(R/'results/native-deflate-exhaustion-proof-integration-20261106/archive-index.json',I/'ir234-local-archive-index.json')
status=dict(schema='mech.related-post-gen.current-compiler-review.v1',status='locally_qualified_draft_review_candidate',compiler=compiler,identity='IR234/work115/ABI41/image9',target_pr=973,target_branch='codex/level12-compression-consolidated',incoming='176d8134133befb8e35a6413fb8f98387bacebb3',
    correctness_fix='Accept current upstream exhausted-payload identity proof. Only identity terminal transitions may be bypassed; nonidentity/missing/conflicting evidence retains the checked funded native source loop. The prior IR231 regression now requires this safe native decline.',
    gpu_passed=844,inherited_gpu_failures=tests['failed_tests'],scalar_rows_exact_IR231=128,compression_rows_exact_IR231=5,all_comparison_fields_retained=True,
    loader_fixtures_exact=434,decoded_bundle_files_exact=52,finite_denials_exact=48,finite_small_full_successes_exact=12,forged_frontier_rejections=16,
    loader_disk_full_repair_completed=True,original_failed_driver_retained=True,fresh_old_objects_rejected=3,fresh_old_object_versions=[231,232,233],prior_old_object_checks_retained_separately=True,
    runtime_only_and_unmodified_generated_main_passed=True,scalar_only_compiler_passed=True,complete_native_text_all_four_variants_equal_IR231=True,native_functions_compared=212,
    current_performance_cohort_run=True,current_performance_role='Original-source control in three randomized source experiment cohorts; current compiler pin is shared by all source variants.',
    current_performance_report='evidence/ordered-source-experiments/summary.json',new_compiler_performance_gain_claim=False,selected=False,goal_complete=False,hosted_ci_green_claim=False,
    source_files=len(read(O/'source-inventory.json')['files']),local_archive_verified_files=archive['verified_files'],raw_archives_remotely_uploaded=False,updated_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
write(I/'ir234-review-status.json',status)
guide=I/'README.md';old=guide.read_text()
header='''# Current related-post integration targeting #973

Compiler **c4649860a5e445c1ce1bdec8503870e8a455da17**, **IR234 / work115 / ABI41 / image9**, integrates #973 through **176d8134133befb8e35a6413fb8f98387bacebb3**. [PR990](https://github.com/mech-lang/mech/pull/990) targets `codex/level12-compression-consolidated` (#973).

The latest upstream proof permits reduction bypass only when every exhausted source path preserves every payload. Nonidentity, missing or conflicting evidence retains the checked funded native source loop. The earlier changed-payload regression remains exact, with its selection assertion updated to require that safe native decline. The identity compressor path and all four related-post native text sections remain unchanged; no compiler speedup is claimed.

Local qualification: **844 GPU passes with exactly two inherited failures**, all **128 scalar and five compression rows exactly equal to IR231 in every non-timing field**, **434 loader fixtures**, **52 byte-identical original/decoded bundle files**, **48 finite denials**, **12 finite small/full successes**, **16 forged-frontier rejections**, and **three fresh same-ABI/work-policy old-Object checks (IR231/232/233)**. Runtime-only full5000, scalar-only compilation and unmodified generated-app success/error/denial checks pass. The last loader link initially exhausted disk; the original failed driver and receipts remain, with the separate successful retry, native audit and all31 fixtures. `ir234-qualification-summary.json` asserts every completed check.

The supplied performance review's Boolean experiments remain available as unselected IR232/233 patches. Three new source experiments use this exact IR234 compiler. Each cohort keeps standard Rust, all three fast numeric Rust controls and the preinitialized full-process control; schedules are recorded before timing. All **933 cohort outputs and93 edge fixtures** are exact.

| Source experiment cohort | Original post-JSON | Candidate post-JSON | Candidate × fastest matched Rust | Original full process | Candidate full process |
|---|---:|---:|---:|---:|---:|
| Separate order scan,31 rounds |2.875083 ms|2.891250 ms|1.813595×|37.376375 ms|37.463041 ms|
| Fused uniqueness/order,31 rounds |2.835500 ms|2.782167 ms|1.736865×|37.056416 ms|35.725541 ms|
| Mask reuse,31 rounds |3.147209 ms|17.569083 ms|10.464795×|38.100625 ms|53.781834 ms|

In the final cohort the simpler fused variant is also3.147209 ms, so its earlier processing win does not repeat. Matched Rust is1.678875 ms (U32) and full-process Rust24.584291 ms (U8/U32). Do not combine ratios across different cohorts. In its own first cohort, fused wins21/31 processing and23/31 full-process pairs; its instructions fall57,086,781→55,996,327. Allocation requests stay3858 versus Rust58. The final mask-reuse variant executes288,258,385 instructions and33,847 allocations versus the original57,544,416 and3858. It exposes a specific compiler gap: iteration3's numeric tree rejects with `numeric tree child returns an older array version`. The conjunction guarding append and the later payload-return branch need a sound correlated currentness proof; dropping the check would be incorrect.

All source variants keep original output tag order and exact fallback. The ordered variants send unordered unique rows to the general path, so they are experiments with a disclosed input-scope cost. **None is selected; IR208 stays selected and Rust parity remains unmet.** See `evidence/ordered-source-experiments/summary.json` for all paired results, all six complete cohort reports, native selections, exact source and scopes. Current qualification is in `ir234-review-status.json`, `evidence/ir234-*` and `ir234-local-archive-index.json` (3177 losslessly archived and SHA-verified files). Raw binaries are not remotely uploaded; hosted CI is not claimed green. Draft; no merge requested.

## Retained IR231 and earlier evidence

'''
guide.write_text(header+old.removeprefix('# Current related-post integration targeting #973\n\n'))
manifest_path=S/'benchmarks/related-post-gen/review/manifest.json';manifest=read(manifest_path)
manifest.setdefault('integration_history',[]).append(manifest['active_integration'])
manifest['active_integration']=status | dict(source_inventory='integration/source-inventory.json',status='integration/ir234-review-status.json')
for p in sorted(I.rglob('*')):
    if not p.is_file():continue
    name=str(p.relative_to(S/'benchmarks/related-post-gen'));item=dict(sha256=sha(p),bytes=p.stat().st_size)
    if p.suffix=='.gz':item.update(compression='gzip',uncompressed_sha256=hashlib.sha256(gzip.decompress(p.read_bytes())).hexdigest())
    manifest['files'][name]=item
for p in (S/'benchmarks/related-post-gen/experiments').glob('related-tiered-ordered-subsets*.mec'):
    manifest['files'][str(p.relative_to(S/'benchmarks/related-post-gen'))]=dict(sha256=sha(p),bytes=p.stat().st_size)
write(manifest_path,manifest)
print(json.dumps(dict(status='prepared',source_files=status['source_files'],evidence_files=len(manifest['files']),source_cohort_outputs=summary['exact_cohort_outputs'])),flush=True)
