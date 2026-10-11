"""Check the configured interpreter and canonical outputs before its timed cohort."""
import json
import subprocess
from dotnet_fetch import D,sha
from probe import execute,save,compact,REQUIRED

audit_path=D/'micropython-source-audit.json';audit=json.loads(audit_path.read_text())
runtime=audit['runtimes']['micropython']
assert sha(runtime['path'])==runtime['sha256']
compact.wait_for_environment(REQUIRED)
text=subprocess.check_output([runtime['path'],*runtime['runtime_options'],'-c',
    'import gc; print(gc.mem_alloc()+gc.mem_free()); print((1.0000000000000002-1.0)>0); print(2**100)'],text=True)
lines=text.strip().splitlines()
assert 12288*1024*1024*.95<int(lines[0])<=12288*1024*1024
assert lines[1]=='True' and int(lines[2])==2**100
results=[]
for row in audit['programs']:
    assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
    result=execute(row,False,900)
    assert result['validation']['reference_match']
    assert not result['power_changes'] and not result['competing_compilers']
    results.append(dict(key=row['key'],validation=result['validation']))
save(D/'evidence/micropython-heap-preflight.json',dict(status='passed',
    source_audit_sha256=sha(audit_path),runtime_sha256=runtime['sha256'],heap_bytes=int(lines[0]),
    binary64=True,unbounded_integers=True,canonical_outputs=results))
print('MicroPython 12288 MB heap, binary64, big integers and twenty canonical outputs: passed.',flush=True)
