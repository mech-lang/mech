from pathlib import Path
import json,subprocess,time,os,importlib.util
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-late-key-additions-compiler-20261108');T=P/'key-tests-v2-optimized';B=Path('/private/tmp/mech-related-completed-key-additions-20261108/final-qualification')
Q=P/'final-qualification';Q.mkdir(exist_ok=True)
read=lambda p:json.loads(Path(p).read_text())
def run(c,n,env=None):
 c=list(map(str,c)); prior=Q/(n+'.json')
 if prior.exists():
  old=read(prior)
  if old['command']==c and old['environment']==(env or{})and old['exit']==0:
   print(n,'reused exact successful qualification',flush=True);return subprocess.CompletedProcess(c,0,old['stdout'],old['stderr'])
 t=time.monotonic();r=subprocess.run(c,cwd=S,capture_output=True,text=True,env={**os.environ,**(env or{})});(Q/(n+'.json')).write_text(json.dumps(dict(command=c,environment=env or{},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,scope='IR310 compiler-only completed-key arithmetic; qualification only, timers ineligible'),indent=2)+'\n');print(n,r.returncode,flush=True)
 if r.returncode:print(r.stdout[-9000:]+r.stderr[-8000:],flush=True)
 assert r.returncode==0
 return r
spec=importlib.util.spec_from_file_location('h',P/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);files=h.source_inventory(S);assert files==read(T/'source-inventory.json')['files'];assert read(T/'contracts.json')['exit']==0
run([T/'gpu-tests','matrix_map_keys_full_input_selects_28','--ignored','--test-threads=1','--nocapture'],'actual-source-and-full-input-first',{'MECH_MATRIX_KEY_BENCH_SOURCE':'/private/tmp/mech-related-native-string-classification-reuse-runners-20261108-v2/histogram-reuse/build/source.mec','MECH_MATRIX_KEY_BENCH_INPUT':'/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','MECH_MATRIX_KEY_BENCH_ORACLE':'/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json'})
wf=(S/'.github/workflows/ci-full.yml').read_text().split('- name: Inventory and execute native composition contracts',1)[1].split('- name:',1)[0];selection=wf.split('--lib --',1)[1].split('--list',1)[0].replace('\\',' ').split();listed=run([T/'gpu-tests',*selection,'--list'],'integrated-ci-list');names={l.removesuffix(': test')for l in listed.stdout.splitlines()if l.endswith(': test')};old={l.removesuffix(': test')for l in read(B/'integrated-ci-list.json')['stdout'].splitlines()if l.endswith(': test')};assert len(old)==489 and old<=names;assert len(names)==490,len(names)
expected_count=int(wf.split('native-composition-tests.txt)" -eq ',1)[1].splitlines()[0]);assert expected_count==len(names)==490
r=run([T/'gpu-tests',*selection,'--test-threads=1'],'integrated-ci-execute');passed={l.removeprefix('test ').removesuffix(' ... ok')for l in r.stdout.splitlines()if l.startswith('test ')and l.endswith(' ... ok')};assert passed==names
(Q/'integrated-ci-inventory-audit.json').write_text(json.dumps(dict(expected=490,listed=len(names),executed_passed=len(passed),all_names_match=True,all489_parent_names_retained=True,new_names=sorted(names-old)),indent=2)+'\n')
run([T/'gpu-tests','program::runtime::maps::','--test-threads=1'],'all-retained-map-contracts')
run([T/'gpu-tests','program::image::','caller_geometry::tests','--test-threads=1'],'image-and-caller-geometry-tests')
run([T/'gpu-tests','native_string_classes','unsigned_guard_facts::','modular_ranges','codegen::utf8_lookup::','codegen::utf8_histogram::','compiled_histogram_tests','program::protocol::','--test-threads=1'],'classification-UTF8-range-and-protocol-required')

r=run([T/'gpu-tests','codegen::packed_map::frame_state::tests','--test-threads=1'],'packed-frame-effects');assert '2 passed; 0 failed' in r.stdout

assert files==h.source_inventory(S);(Q/'qualified-source-inventory.json').write_text(json.dumps(dict(root=str(S),files=files),indent=2)+'\n');(Q/'GPU-qualification-summary.json').write_text(json.dumps(dict(actual_CI_passed=490,all489_parent_names_retained=True,source_unchanged=True,actual_original_and_restored_outputs_exact=True,borrowed_value_banks=1,owned_seed_imports=0,borrowed_bank_entry_helpers=1,native_seed_frames=1,canonical_seed_tuples=0,native_seed_imports=1,late_seed_materializations=0,owned_key_views=1,borrowed_String_cells=28,canonical_key_matrices=0,canonical_String_values=0,native_dictionary_publications=1,canonical_dictionary_publications=0,frozen_dictionary_loans=1,actual_source_receipt=str(Q/'actual-source-and-full-input-first.json')),indent=2)+'\n');print('IR310 required GPU qualification complete',flush=True)
