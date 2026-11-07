from pathlib import Path
import json,os,subprocess,time,shutil,hashlib

R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O=Path('/private/tmp/mech-related-invariant-read-backend-audit-opt-20261107-v1')
P=Path('/private/tmp/mech-related-invariant-read-backend-audit-runners-20261107-v1');P.mkdir()
B=Path('/private/tmp/mech-related-invariant-capture-hoisting-runners-20261107-v2/reuse-4')
def run(c,p,env=None):
 c=list(map(str,c));start=time.monotonic();x=subprocess.run(c,capture_output=True,text=True,env={**os.environ,**(env or {})})
 p.write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-start,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n')
 assert x.returncode==0,x.stderr
shutil.copyfile('/private/tmp/mech-related-invariant-capture-hoisting-opt-20261107-v2/compiler-facts.rs',O/'compiler-facts.rs')
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',O/'compiler-facts.rs','--out',O/'compiler'],O/'compiler-build-run.json')
run([O/'compiler',B/'build/source.mec',P/'build'],P/'compile-run.json',dict(MECH_NATIVE_IR_DUMP=str(P/'ir')))
files={p.name:p.read_bytes()==(B/'build'/p.name).read_bytes()for p in(P/'build').iterdir()if p.is_file()}
assert len(files)==13 and all(files.values()),files
loads=[json.loads(p.read_text())for p in(P/'ir').glob('*-loads.json')]
assert len(loads)==51
report=dict(compiler='c14760210be8e161ed6a523c651203f25b9cded5',identity='IR250/work115/ABI41/image9',diagnostic_only=True,
 all13_normal_bundle_files_byte_identical=files,original_source=str(B/'build/source.mec'),
 optimized_clones=len(loads),readonly_loads=sum(len(x['readonly_movable_loads'])for x in loads),
 readonly_loads_inside_loops=sum(x['innermost_loop_header']is not None for row in loads for x in row['readonly_movable_loads']),
 dumps=[dict(header=x['header'],selected_source_reads=x['selected_source_reads'],loads=x['readonly_movable_loads'])for x in loads if x['selected_source_reads']],
 performance_claim=False)
(P/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
