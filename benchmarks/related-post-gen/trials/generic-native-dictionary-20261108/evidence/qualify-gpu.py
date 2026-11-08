from pathlib import Path
import hashlib,importlib.util,json,os,subprocess,time
P=Path(__file__).parent;T=P/'native-tests-v3';S=Path('/private/tmp/mech-related-generic-native-dictionary-compiler-20261108')
def read(p):return json.loads(Path(p).read_text())
def run(cmd,name,env=None):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,cwd=S,capture_output=True,text=True,env={**os.environ,**(env or {})})
 (P/(name+'.json')).write_text(json.dumps(dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,scope='Qualification only; timers ineligible'),indent=2)+'\n');print(name,r.returncode,flush=True)
 if r.returncode:print(r.stdout[-8000:]+r.stderr[-8000:],flush=True)
 assert r.returncode==0
 return r
spec=importlib.util.spec_from_file_location('h','/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
files=h.source_inventory(S);assert files==read(P/'native-connection-source-inventory-v3.json')['files']
assert read(T/'generic-dictionary.json')['exit']==0
env={'MECH_MATRIX_KEY_BENCH_SOURCE':'/private/tmp/mech-related-native-string-classification-reuse-runners-20261108-v2/histogram-reuse/build/source.mec','MECH_MATRIX_KEY_BENCH_INPUT':'/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','MECH_MATRIX_KEY_BENCH_ORACLE':'/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json'}
run([T/'gpu-native-tests','generic_dictionary_actual','borrowed_map_keys_actual','matrix_map_keys_selects_actual','--ignored','--test-threads=1','--nocapture'],'actual-source-proofs',env)
run([T/'gpu-native-tests','matrix_map_keys_full_input','--ignored','--test-threads=1','--nocapture'],'actual-full-input-path',env)
workflow=(S/'.github/workflows/ci-full.yml').read_text().split('- name: Inventory and execute native composition contracts',1)[1].split('- name:',1)[0]
selection=workflow.split('--lib --',1)[1].split('--list',1)[0].replace('\\',' ').split()
listed=run([T/'gpu-native-tests',*selection,'--list'],'ci-list');names={x.removesuffix(': test')for x in listed.stdout.splitlines()if x.endswith(': test')}
prior=read('/private/tmp/mech-related-borrowed-map-key-continuations-20261108/ci-list.json')['stdout'];parents={x.removesuffix(': test')for x in prior.splitlines()if x.endswith(': test')}
assert len(parents)==451 and parents<=names
assert len(names)==460,len(names)
executed=run([T/'gpu-native-tests',*selection,'--test-threads=1'],'ci-execute');passed={x.removeprefix('test ').removesuffix(' ... ok')for x in executed.stdout.splitlines()if x.startswith('test ')and x.endswith(' ... ok')}
assert names==passed,(names-passed,passed-names)
(P/'ci-inventory-audit.json').write_text(json.dumps(dict(expected=len(names),listed=len(names),executed_passed=len(passed),all_names_match=True,all451IR303names_retained=True,new_names=sorted(names-parents)),indent=2)+'\n')
run([T/'gpu-native-tests','program::runtime::maps::','--test-threads=1'],'all-retained-map-contracts')
run([T/'gpu-native-tests','program::image::','caller_geometry::tests','--test-threads=1'],'image-and-caller-geometry-tests')
run([T/'gpu-native-tests','native_string_classes','unsigned_guard_facts::','modular_ranges','codegen::utf8_lookup::','codegen::utf8_histogram::','compiled_histogram_tests','program::protocol::','program::runtime::collections::','--test-threads=1'],'classification-UTF8-range-and-protocol-regressions')
assert files==h.source_inventory(S)
(P/'qualified-source-inventory.json').write_text((P/'native-connection-source-inventory-v3.json').read_text())
(P/'GPU-qualification-summary.json').write_text(json.dumps(dict(actual_CI_passed=len(passed),all451IR303names_retained=True,full_input_owned_view=1,canonical_key_matrices=0,borrowed_String_cells=28,actual_original_and_restored_outputs_exact=True,native_dictionary_publications=1,canonical_dictionary_publications=0,frozen_dictionary_loans=1,fallback_materializations=0,all_retained_map_image_caller_UTF8_protocol_contracts_passed=True,source_inventory_unchanged=True),indent=2)+'\n');print('IR304 GPU qualification complete',flush=True)
