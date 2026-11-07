from pathlib import Path
import json,subprocess,shutil,time,os,hashlib
Q=Path('/private/tmp/mech-related-invariant-key-source-qualification-20261107-v1');Q.mkdir(exist_ok=True)
U=Path('/private/tmp/mech-related-invariant-key-source-runners-20261107-v1/242-reads-4');B=Path('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1');O=Path('/private/tmp/mech-related-conjoined-window-integration-opt-20261107-v1');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');RT=Path('/private/tmp/mech-related-conjoined-window-integration-runtime-check-20261107-v1')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(c,p,env=None):
 c=list(map(str,c));start=time.monotonic();x=subprocess.run(c,capture_output=True,text=True,env={**os.environ,**(env or {})});p.write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-start,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');assert x.returncode==0,x.stdout+x.stderr;print(p.name,'passed',flush=True);return x
(Q/'reuse-4').symlink_to(U,target_is_directory=True);(Q/'after-4').symlink_to(U,target_is_directory=True)
text=(B/'qualify-new-source-loaders.py').read_text().replace("P/'source.mec'","P/'build/source.mec'").replace("old=(U/('after-4-'+loader)","old=(B/('after-4-'+loader)")
text=text.replace("P=U/'reuse-4'","B=Path('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1')\nP=U/'reuse-4'")
(Q/'qualify-loaders.py').write_text(text)
(Q/'qualify-budgets.py').write_text((B/'qualify-mask-budgets.py').read_text())
run(['python3',Q/'qualify-loaders.py'],Q/'loaders-run.json')
run(['python3',Q/'qualify-budgets.py'],Q/'budgets-run.json')
p=Q/'frontier-image-probe';p.mkdir();(p/'build').symlink_to(U/'build',target_is_directory=True);shutil.copyfile(B/'frontier-image-probe/probe.rs',p/'probe.rs')
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'probe.rs','--out',p/'probe','--bundle',p/'build','--linked'],p/'build-run.json')
run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'probe','--output',p/'native-link-audit.json'],p/'audit-run.json')
row=json.loads(run([p/'probe'],p/'run.json').stdout);assert row['genuine_legacy_independent_bound_and_compiled_module_accepted'] and len(row['forged_cases'])==8 and row['native_entry_never_invoked']
print('16 forged frontier refusals exact',flush=True)
p=Q/'runtime-only';p.mkdir();(p/'build').symlink_to(U/'build',target_is_directory=True);shutil.copyfile(U/'runner.rs',p/'runner.rs')
run(['python3',R/'build-host.py','--compiler-build',RT/'program-runtime-build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'build-run.json')
run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
for case,posts,oracle in [('first10',R/'fixtures/posts-first-10.json',Path('/private/tmp/mech-related-object-bound-image-budget-checks-20261104-v4/oracle/related_posts_rust.json')),('full5000',Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json'),Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json'))]:
 out=p/(case+'-output.json');run([p/'runner-aot',p/'build/source.mec',posts,out,'1'],p/(case+'-run.json'));assert json.loads(out.read_text())==json.loads(oracle.read_text())
old=json.loads((R/'results/selected-speed-checkpoint-20261107.json').read_text());assert old['compiler']=='4cc266ae0d05c69432a4c15ffe99d5e586685647'
summary=dict(status='passed',source=str(U/'build/source.mec'),source_sha256=sha(U/'build/source.mec'),compiler=old['compiler'],compiler_identity='IR242/work115/ABI41/image9',compiler_qualification_reused_only_identical_code_and_flags=old['qualification'],new_source_original_decoded_files_exact=13,loader_edges_exact=124,finite_denials_exact=24,finite_small_full_successes_exact=6,forged_frontier_rejections=16,runtime_only_first10_full5000_exact=True,source_selection_pending_confirmation_with_new_compiler_trial=True,selected=False)
(Q/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print('New source qualification passed; inherited compiler qualification and all new source loader/finite checks retained.',flush=True)
