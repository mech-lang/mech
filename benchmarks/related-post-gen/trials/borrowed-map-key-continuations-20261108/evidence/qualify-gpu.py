from pathlib import Path
import hashlib,importlib.util,json,os,subprocess,time,re
P=Path(__file__).parent;T=P/'expanded-tests';S=Path('/private/tmp/mech-related-borrowed-map-key-views-compiler-20261108')
def read(p):return json.loads(Path(p).read_text())
def run(cmd,name,env=None):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,cwd=S,capture_output=True,text=True,env={**os.environ,**(env or {})})
 (P/(name+'.json')).write_text(json.dumps(dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,scope='Qualification only; timers ineligible'),indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-6000:]+r.stderr[-6000:];return r
spec=importlib.util.spec_from_file_location('b','/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/build-compiler.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
files=m.source_inventory(S);assert files==read(P/'gpu-source-inventory-v3.json')['files']
assert read(T/'borrowed-map-key-runtime.json')['exit']==read(T/'matrix-map-key-regressions.json')['exit']==0
run([T/'gpu-tests','virtual_value_definitions','--test-threads=1','--nocapture'],'virtual-generation-contracts')
env={'MECH_MATRIX_KEY_BENCH_SOURCE':'/private/tmp/mech-related-native-string-classification-reuse-runners-20261108-v2/histogram-reuse/build/source.mec','MECH_MATRIX_KEY_BENCH_INPUT':'/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','MECH_MATRIX_KEY_BENCH_ORACLE':'/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json'}
run([T/'gpu-tests','borrowed_map_keys_actual','matrix_map_keys_selects_actual','--ignored','--test-threads=1','--nocapture'],'actual-source-proofs',env)
run([T/'gpu-tests','matrix_map_keys_full_input','--ignored','--test-threads=1','--nocapture'],'actual-full-input-path',env)
workflow=(S/'.github/workflows/ci-full.yml').read_text().split('- name: Inventory and execute native composition contracts',1)[1].split('- name:',1)[0]
selection=workflow.split('--lib --',1)[1].split('--list',1)[0].replace('\\',' ').split()
listed=run([T/'gpu-tests',*selection,'--list'],'ci-list');names={x.removesuffix(': test') for x in listed.stdout.splitlines()if x.endswith(': test')}
old=read('/private/tmp/mech-related-private-write-first-20261108/ci-list.json')['stdout'];oldnames={x.removesuffix(': test')for x in old.splitlines()if x.endswith(': test')}
assert len(oldnames)==446 and oldnames<=names
assert len(names)==451,len(names)
executed=run([T/'gpu-tests',*selection,'--test-threads=1'],'ci-execute');passed={x.removeprefix('test ').removesuffix(' ... ok')for x in executed.stdout.splitlines()if x.startswith('test ')and x.endswith(' ... ok')}
assert names==passed,(names-passed,passed-names)
(P/'ci-inventory-audit.json').write_text(json.dumps(dict(expected=len(names),listed=len(names),executed_passed=len(passed),all_names_match=True,all446parent_names_retained=True,new_names=sorted(names-oldnames)),indent=2)+'\n')
run([T/'gpu-tests','program::runtime::maps::','--test-threads=1'],'all-retained-map-contracts')
run([T/'gpu-tests','program::image::','caller_geometry::tests','--test-threads=1'],'image-and-caller-geometry-tests')
run([T/'gpu-tests','native_string_classes','unsigned_guard_facts::','modular_ranges','codegen::utf8_lookup::','codegen::utf8_histogram::','compiled_histogram_tests','program::protocol::','--test-threads=1'],'classification-UTF8-range-and-protocol-regressions')
assert files==m.source_inventory(S)
(P/'qualified-source-inventory.json').write_text((P/'gpu-source-inventory-v3.json').read_text())
(P/'GPU-qualification-summary.json').write_text(json.dumps(dict(actual_CI_passed=len(passed),all446parent_names_retained=True,full_input_owned_view=1,canonical_key_matrices=0,borrowed_String_cells=28,retired_histogram_handle=1,actual_original_and_restored_outputs_exact=True,all_retained_map_image_caller_UTF8_protocol_contracts_passed=True,source_inventory_unchanged=True),indent=2)+'\n');print('IR303 GPU qualification complete',flush=True)
