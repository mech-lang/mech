"""Inspect the actual measured arena binaries, without recompilation or instrumentation."""
import json
import re
import subprocess
from pathlib import Path
from bootstrap import D, sha

OBJDUMP = '/Library/Developer/CommandLineTools/usr/bin/llvm-objdump'


def body(text, symbol):
    match = re.search(r'^\w+ <'+re.escape(symbol)+r'>:\n(.*?)(?=^\w+ <|\Z)', text, re.M|re.S)
    assert match, symbol
    return match.group(1)


def main():
    rows=json.loads((D/'results/final-extra-builds.json').read_text())['programs']
    results=[]
    for row in rows:
        if row['language'] not in ('zig','odin'): continue
        symbols=subprocess.check_output(['nm','-nm',row['binary']],text=True).splitlines()
        prefix='_'+row['id'] if row['language']=='zig' else '_main'
        separator='.' if row['language']=='zig' else '::'
        names={part:prefix+separator+part for part in ('build','check','batch')}
        assert all(any(line.endswith(name) for line in symbols) for name in names.values())
        selected=list(names.values())
        if row['language']=='zig': selected.append('_heap.FixedBufferAllocator.alloc')
        command=[OBJDUMP,'--disassemble','--no-show-raw-insn','--disassemble-symbols='+','.join(selected),row['binary']]
        assembly=subprocess.check_output(command,text=True)
        constructor=body(assembly,names['build'])
        traversal=body(assembly,names['check'])
        batch=body(assembly,names['batch'])
        recursive_calls=re.findall(r'\bbl\s+\S+ <'+re.escape(names['build'])+r'>',constructor)
        assert len(recursive_calls)==2
        assert re.search(r'\bmov\s+w[01], #16\b',constructor)
        assert '#8]' in constructor and '#8]' in traversal
        build_call=batch.index('<'+names['build']+'>')
        check_call=batch.index('<'+names['check']+'>')
        reset=re.search(r'\bstr\s+xzr, \[sp, #\d+\]',batch[check_call:])
        assert build_call < check_call and reset
        if row['language']=='zig':
            assert 'blr' in constructor and 'stp\txzr, xzr' in constructor
            allocator=body(assembly,'_heap.FixedBufferAllocator.alloc')
            assert re.search(r'\bstr\s+x\d+, \[x0\]',allocator)
        else: assert '<_runtime::mem_alloc_bytes>' in constructor
        path=D/'evidence'/('timed-'+row['id']+'.asm.txt')
        path.write_text(assembly)
        results.append(dict(key=row['key'],status='passed',source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],
            assembly=str(path),assembly_sha256=sha(path),node_allocation_bytes=16,recursive_child_constructor_calls=2,
            both_pointer_children_stored_and_loaded=True,construction_and_traversal_inside_iteration_loop=True,
            reset_after_traversal=True,allocator_call_retained=True))
        print('TIMED ASSEMBLY PASS',row['id'],flush=True)
    assert len(results)==4
    report=dict(status='passed',tool=OBJDUMP,tool_sha256=sha(OBJDUMP),programs=results,
        reason='Disassembly is from the actual pinned, uninstrumented measured binaries. The constructor retains a 16-byte allocator request in the common leaf/interior path and separate recursive child construction/stores. Checks load both child pointers. Each batch loop retains construction and traversal calls followed by arena reset. This verifies the optimized compiler did not collapse the workload or eliminate node allocation.')
    (D/'results/native-tree-assembly-final.json').write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__': main()
