from pathlib import Path
import datetime,gzip,hashlib,importlib.util,json,shutil,subprocess
S=Path('/private/tmp/mech-related-deflate-exhaustion-proof-integration-compiler-20261106')
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
U=Path(__file__).parent
PREFIX='/private/tmp/mech-related-native-boolean-implication-freshness-'
O=Path(PREFIX+'opt-20261106-v7');T=Path(PREFIX+'tests-20261106-v4')
C=Path(PREFIX+'regressions-20261106-v2');RT=Path(PREFIX+'runtime-check-20261106-v2')
Q=Path(PREFIX+'feature-check-20261106-v2');OLD=Path(PREFIX+'old-objects-20261106-v2')
P=Path('/private/tmp/mech-related-native-boolean-implication-mask-reuse-pc-profile-20261106-v1')
I=S/'benchmarks/related-post-gen/integration';E=I/'evidence';X=E/'boolean-implication-mask-reuse';X.mkdir(exist_ok=True)
read=lambda p:json.loads(p.read_text())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
write=lambda p,d:p.write_text(json.dumps(d,indent=2)+'\n')
summary=read(R/'results/compiler-native-boolean-implication-mask-reuse-20261106.json')
assert summary['qualification']['gpu_passed']==849 and summary['qualification']['loader_fixtures_exact']==558
assert summary['selected'] is False and summary['goal_complete'] is False
assert read(RT/'summary.json')['status']=='passed' and read(Q/'scalar-compiler.json')['exit']==0
assert read(OLD/'summary.json')['fresh_old_objects']==3
for kind,count in [('scalar',128),('one-kib',5)]:
    row=read(C/(kind+'-comparison.json'));assert row['rows']==count and row['all_nontiming_fields_equal_IR234'] and not row['differences']
spec=importlib.util.spec_from_file_location('builder',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
assert h.source_inventory(S)==read(O/'source-inventory.json')['files']
for name in ['compiler-build.json','source-inventory.json']:
    old=subprocess.check_output(['git','show','acdd87fbe4b22325b121bb3a6609eb04a0a02666:benchmarks/related-post-gen/integration/'+name],cwd=S)
    (E/('ir235-retained-ir234-'+name)).write_bytes(old)
    shutil.copyfile(O/('build.json' if name=='compiler-build.json' else name),I/name)
def copy(source,name):shutil.copyfile(source,E/('ir235-'+name))
for root,label in [(T,'gpu'),(C,'control'),(Q,'feature'),(OLD,'old-object')]:
    for p in sorted(root.iterdir()):
        if p.is_file() and p.suffix in ['.json','.log','.py','.rs']:
            copy(p,label+'-'+p.name+('.txt'if p.suffix=='.rs'else''))
for v in [1,2,3]:
    root=Path(PREFIX+'tests-20261106-v'+str(v))
    for p in sorted(root.iterdir()):
        if p.is_file() and p.suffix in ['.json','.log','.py']:
            copy(p,'initial-tests-v'+str(v)+'-'+p.name)
for name in ['summary.json','runtime-first10.json','runtime-full5000.json','managed-main-build.json','generate-standard-app.json','scalar-runtime.json','program-runtime.json','build-and-check.py']:
    copy(RT/name,'runtime-'+name)
for p in sorted(U.iterdir()):
    if p.is_file() and p.suffix in ['.json','.py']:
        copy(p,p.name)
for family in ['budget','compact-budget']:
    copy(U/family/'summary.json',family+'-summary.json')
    for loader in ['bound','independent','full-catalog']:
        for case in ['first-10','full-5000']:
            copy(U/family/loader/('success-rust-'+case+'-run.json'),family+'-'+loader+'-'+case+'-success.json')
for path in sorted(U.glob('edges-*/edge-fixtures.json'))+sorted(U.glob('*/edges/edge-fixtures.json')):
    copy(path,'edges-'+str(path.relative_to(U)).replace('/','-'))
copy(U/'frontier-image-probe/run.json','frontier-image-run.json')
copy(S/'.github/workflows/ci-full.yml','ci-full.yml')
copy(S/'hosts/gpu/tests/fixtures/native/numeric-boolean-mask-reuse.mec','native-mask-reuse.mec')
for name in ['summary.json','enriched-summary.json','all-symbol-observations.json','worker-instruction-observations.json','build-and-profile.py','analyze-profile.py','analyze-all-symbols.py']:
    shutil.copyfile(P/name,X/('profile-'+name))
for label in ['before','after']:
    shutil.copyfile(P/label/'run.json',X/('profile-'+label+'-run.json'))
    shutil.copyfile(P/label/'native-link-audit.json',X/('profile-'+label+'-native-link-audit.json'))
for phase,name in [('latency','comparison-native-boolean-implication-mask-reuse-randomized-20261106-5000-m3.json'),('resources','comparison-native-boolean-implication-mask-reuse-resources-20261106-5000-m3.json')]:
    (X/(phase+'.json.gz')).write_bytes(gzip.compress((R/'results'/name).read_bytes(),mtime=0))
write(X/'summary.json',summary)
for name in ['source.mec','native-link-audit.json','runner-aot.build.json','compile-run.json','edge-run.json']:
    shutil.copyfile(U/'reuse-4'/name,X/name)
metadata=read(U/'reuse-4/build/metadata.json')
write(X/'native-selection.json',{k:metadata[k]for k in ['typed_ir_version','code_bytes','stack_bytes','compiled_numeric_functions','numeric_tree_candidates','captured_numeric_candidates','readonly_numeric_candidates']})
archive=read(R/'results/native-boolean-implication-freshness-20261106/archive-index.json');assert archive['verified_files']==2647
shutil.copyfile(R/'results/native-boolean-implication-freshness-20261106/archive-index.json',I/'ir235-local-archive-index.json')
status=dict(schema='mech.related-post-gen.current-compiler-review.v1',status='locally_qualified_draft_review_candidate',compiler=summary['compiler'],identity=summary['identity'],incoming=summary['incoming_973'],target_pr=973,target_branch='codex/level12-compression-consolidated',gpu_passed=849,inherited_gpu_failures=summary['qualification']['inherited_gpu_failures'],source_files=2464,scalar_rows_exact_IR234=128,compression_rows_exact_IR234=5,all_comparison_fields_retained=True,loader_fixtures_exact=558,decoded_bundle_files_exact=65,finite_denials_exact=48,finite_small_full_successes_exact=12,forged_frontier_rejections=16,fresh_old_objects_rejected=3,fresh_old_object_versions=[234,232,233],ci_selected_names_passed=354,all_five_new_regressions_passed=True,source_prices_unchanged=True,all_work_total_differences_recorded=True,accounting_observations_exact=112,runtime_only_and_unmodified_generated_main_passed=True,scalar_only_compiler_passed=True,complete_native_text_all_four_original_variants_equal_IR234=True,native_functions_compared=212,source_unchanged_across_compilers=True,current_performance_cohort_run=True,current_performance_report='evidence/boolean-implication-mask-reuse/summary.json',cohort_outputs_exact=349,profile_invocations_exact=2000,local_archive_verified_files=2647,selected=False,selected_speed_checkpoint='IR208',goal_complete=False,hosted_ci_green_claim=False,raw_archives_remotely_uploaded=False,updated_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
write(I/'ir235-review-status.json',status)
header='''# Current related-post integration targeting #973

Compiler **cbe5b47839b4fe2300fe16ebc1fcacfcfa03f98b**, **IR235 / work115 / ABI41 / image9**, integrates #973 through **176d8134133befb8e35a6413fb8f98387bacebb3**. [PR990](https://github.com/mech-lang/mech/pull/990) targets `codex/level12-compression-consolidated` (#973).

This fills the compiler gap exposed by reusing eligibility masks. The compiler now proves that every Boolean producer yields zero or one, recognizes repeated reads of the same private tuple field, and carries true-conjunction/false-disjunction/negation implications through its current-array proof. Producers must dominate the current read and execute at most once per loop traversal. Facts reset at the relevant header/exit. Raw inputs, unknown joins, tags, cycles and unavailable values grant no implication. A genuinely stale returned array still declines. No production meter, source-price or funding code changes.

The exact same mask-reuse source falls from **16.914833 ms to2.891875 ms** in a31-round randomized matched cohort. This repairs a source regression; the gain against the original source is modest. Every process computes all5000 posts. Continuous post-JSON includes admission, preparation, allocation, worker creation/run/join and materialization. Full process covers fresh launch through exit. All279 latency outputs and70 separate resource outputs are exact, with every schedule and initial/adverse sample retained.

| Four-worker Mech variant | Post-JSON | × fastest matched Rust | Full process | × fastest full-process Rust |
|---|---:|---:|---:|---:|
| IR234 original control |2.947458 ms|1.844227×|37.544959 ms|1.530292×|
| IR234 mask reuse, same source |16.914833 ms|10.583624×|53.275958 ms|2.171471×|
| IR235 original source |2.944250 ms|1.842220×|37.371041 ms|1.523204×|
| IR235 mask reuse |2.891875 ms|1.809448×|37.271916 ms|1.519163×|

Fastest matched Rust is U8/U32, **1.598208 ms**. Fastest full-process Rust is U32, **24.534500 ms**. Standard Rust, streaming, U32, U8/U32 and the preinitialized full-process control remain in the matrix. The latter's short internal timer remains ineligible for matched processing. The new source wins31/31 pairs against its slow parent, but27/31 processing and only14/31 full-process pairs against same-compiler original source. **Full-process improvement is not established.** Ordered rows retain original output tag order; unordered unique rows take the exact general fallback and lose their old subset fast path. The source stays unselected; IR208 stays the selected speed checkpoint and Rust parity remains unmet.

Processing instructions fall287,497,378→55,052,841 for the unchanged mask-reuse source; original-source IR235 is57,099,888 and matched Rust19,213,377. Requests fall33,847→3858, which equals original Mech and still exceeds Rust58. New-source requested bytes8,013,126 and peak live requested heap13,344,076 compare with Rust6,825,713 and7,663,209. Full-process instructions are347,008,059 versus full-process Rust167,992,662. Restoration alone remains132,499,754 instructions. New source emits876,720 native bytes versus original908,464; its hot query remains28,448 bytes with a528-byte frame.

Fresh before/after profiles validate2000 exact invocations using each executable's actual linked native bytes. The slow parent's runtime segment, work-budget and admission positions largely disappear; the current native query is the leading application-code position, followed by UTF-8 preparation and other generated kernels. Current query observations still include stack traffic, comparisons, rank arithmetic and moves. Wait positions are separate. **PC observations are not CPU-time shares**, and profiled timings are excluded from comparisons. No earlier PC map is reused.

Qualification: **849 GPU passes and exactly two inherited failures**; all128 scalar and five compression control rows equal IR234 in every non-timing field, allocations included. There are558 exact loader/source fixtures,65 byte-identical original/decoded bundle files,48 finite denials,12 finite first10/full5000 successes,16 forged-frontier rejections and three fresh genuine same-ABI/work-policy old Objects (IR234/232/233) rejected before entry. Scalar-only, runtime-only full5000 and unmodified generated-app success/error/denial checks pass. All354 selected CI names execute and pass in the full local run. The prior integrated guard218 was stale against IR234's349 names; the guard now includes the five new regressions without dropping a filter. Hosted CI is not claimed green.

The first new test incorrectly assumed that total charged work must equal emission with every scalar loop disabled. Total work also charges actual owner/metadata/transport callbacks. The completed112-observation audit keeps every total, proves source-price tables unchanged and records monotonic callback debits. In its first case, the entire117,400,138 difference is inside callbacks; outside-callback work is907,408 for both. The original failed assertion, test-only compilation mistake and qualification-helper setup failures remain retained. No old control counter is exempted.

See `ir235-review-status.json`, `evidence/boolean-implication-mask-reuse/summary.json`, `evidence/ir235-*` and `ir235-local-archive-index.json` (2647 SHA-verified local files, plus retained superseded executable references). Raw binaries are not remotely uploaded. Next targets are immutable first-call/restoration metadata, producer-derived posting/index ranges, bounded tree funding and cold scalar-home handling. **Draft; no merge requested.**

## Retained IR234 and earlier evidence

'''
p=I/'README.md';p.write_text(header+p.read_text().removeprefix('# Current related-post integration targeting #973\n\n'))
p=S/'benchmarks/related-post-gen/README.md';text=p.read_text();a=text.index('Current integration:');b=text.index('\n\n',a)
text=text[:a]+'''Current integration: [related-post compiler plus #973](integration/README.md), **IR235 / work115 / ABI41 / image9**, through176d8134133 and targeting [PR990](https://github.com/mech-lang/mech/pull/990). The compiler now lowers the unchanged mask-reuse source as native code: **2.891875 ms /1.809448× fastest matched Rust**, **37.271916 ms /1.519163× fastest full-process Rust**, against same-cohort references1.598208 ms and24.534500 ms. This repairs the slow source variant; its gain over original-source IR235 (2.944250 ms) is small and its full-process paired result is inconclusive. Current qualification is849 GPU passes with two inherited failures,133 exact control rows,558 loader/source fixtures,65 decoded bundle files and2000 exact profile invocations. All349 latest cohort outputs are exact. All four original-source native text sections are byte-identical to IR234. The source stays unselected and IR208 remains the selected speed checkpoint. Older results and failed receipts remain separately pinned; the table below is historical IR223 evidence.''' +text[b:]
text=text.replace('## Current measurements','## Retained IR223 measurements',1);p.write_text(text)
manifest_path=S/'benchmarks/related-post-gen/review/manifest.json';manifest=read(manifest_path)
manifest.setdefault('integration_history',[]).append(manifest['active_integration']);manifest['active_integration']=status|dict(source_inventory='integration/source-inventory.json',status='integration/ir235-review-status.json')
for p in [S/'benchmarks/related-post-gen/README.md',*sorted(I.rglob('*'))]:
    if not p.is_file():continue
    name=str(p.relative_to(S/'benchmarks/related-post-gen'));item=dict(sha256=sha(p),bytes=p.stat().st_size)
    if p.suffix=='.gz':item.update(compression='gzip',uncompressed_sha256=hashlib.sha256(gzip.decompress(p.read_bytes())).hexdigest())
    manifest['files'][name]=item
write(manifest_path,manifest)
print(json.dumps(dict(status='prepared',source_files=2464,evidence_files=len(manifest['files']))),flush=True)
