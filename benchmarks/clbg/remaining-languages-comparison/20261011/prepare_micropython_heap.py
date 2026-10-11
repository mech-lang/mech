"""Freeze a larger uniform heap before any MicroPython full-size measurements."""
import copy
import json
from dotnet_fetch import D,sha
from probe import save

original=D/'source-audit-v2.json';target=D/'micropython-source-audit.json'
assert not target.exists()
audit=copy.deepcopy(json.loads(original.read_text()))
audit['programs']=[r for r in audit['programs'] if r['language']=='micropython']
assert len(audit['programs'])==20
for row in audit['programs']:
    assert row['command'][1:3]==['-X','heapsize=1024M']
    row['command'][2]='heapsize=12288M'
runtime=audit['runtimes']['micropython']
runtime['runtime_options']=['-X','heapsize=12288M']
audit['runtimes']={'micropython':runtime}
audit['coverage']={'micropython':20};audit['unmeasured_alternates']=[]
audit['heap_policy']=dict(megabytes=12288,uniform=True,
    reason='The full 125-million-base nucleotide workload constructs complete growable maps. The initial 1024 MB test heap is not a full-workload memory budget.',
    application_source_changed=False,prior_full_size_micropython_samples=0,
    original_audit=str(original),original_audit_sha256=sha(original))
audit['timing_boundary']+=' MicroPython uses a uniform 12288 MB heap for every candidate and sample; allocation, GC, startup and output remain timed.'
for path in (original,D/'prepare_micropython_heap.py',D/'micropython_heap_preflight.py'):
    audit['pin_files'][str(path)]=sha(path)
save(target,audit)
print(json.dumps(dict(programs=len(audit['programs']),heap_megabytes=12288,source_changed=False)))
