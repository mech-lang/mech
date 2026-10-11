"""Check the stock CPU compiler's node storage and retained allocation capabilities."""
import hashlib,json,re,subprocess
from pathlib import Path
D=Path(__file__).resolve().parent
COMPILER=Path('/opt/homebrew/bin/futhark')
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(command):
    r=subprocess.run(command,cwd=D,capture_output=True,text=True)
    return r
def main():
    rows=[]
    for backend in ('c','multicore'):
        stem='node-noinline' if backend=='c' else 'node-noinline-multicore'
        command=[str(COMPILER),backend,'--library','-o','builds/'+stem,'probes/node-noinline.fut']
        result=run(command)
        assert result.returncode==0,result.stderr
        generated=D/'builds'/(stem+'.c')
        text=generated.read_text()
        constructor=re.search(r'FUTHARK_FUN_ATTR int futrts_allocate_\d+[^;]+?\n\{.*?\n\}',text,re.S)
        assert constructor and 'memblock_alloc' in constructor[0]
        assert re.search(r'\(int64_t\) 16',constructor[0])
        call=re.search(r'if \(futrts_allocate_\d+\(ctx, &(ext_mem_\d+)',text)
        assert call
        temporary=call[1]
        release=re.search(r'memblock_unref\(ctx, &'+temporary+r',',text[call.start():])
        assert release
        caller=text[call.start():call.start()+release.end()+80]
        assert ('lmad_copy' in caller if backend=='c' else '.free_'+temporary+' = '+temporary+'.mem' in caller)
        evidence=D/'results'/(backend+'-allocation-lifetime.txt')
        evidence.write_text(constructor[0]+'\n\n'+caller+'\n')
        rows.append(dict(backend=backend,command=command,generated_c_sha256=sha(generated),
            evidence=str(evidence.relative_to(D)),evidence_sha256=sha(evidence),
            result='Each 16-byte array is allocated, copied into contiguous outer storage, and unreferenced inside the construction loop. It is not retained as a distinct node allocation for later traversal.'))
    failures=[]
    for name in ('recursive-type','foreign-allocator'):
        source=D/'probes'/(name+'.fut')
        command=[str(COMPILER),'check',str(source.relative_to(D))]
        result=run(command)
        assert result.returncode==2
        log=D/'results'/(name+'.log')
        log.write_text(result.stdout+result.stderr)
        failures.append(dict(probe=name,source_sha256=sha(source),command=command,returncode=result.returncode,
            log=str(log.relative_to(D)),log_sha256=sha(log)))
    report=dict(status='unsupported-allocation-model',language='futhark',benchmark='binarytrees',
        compiler=dict(path=str(COMPILER),sha256=sha(COMPILER),version=run([str(COMPILER),'--version']).stdout.strip()),
        sources={str(p.relative_to(D)):sha(p) for p in (D/'probes').glob('*.fut')},
        constructor_probes=rows,rejected_type_and_intrinsic_probes=failures,
        conclusion='The stock 0.27.1 C and multicore backends do not provide a faithful retained per-node heap model. Non-inlined array allocation still copies into flat storage and releases the temporary immediately. A general heap-reference/allocator compiler extension would be needed; no host tree executor or modified generated-C benchmark is admitted.',
        scope='This is capability evidence, not a benchmark timing. Existing nine faithful Futhark workloads and their immutable measurements are unchanged.',
        references=[
            'https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html',
            'https://futhark-lang.org/blog/2021-08-02-value-representation.html',
            'https://futhark-lang.org/blog/2026-08-05-recursion.html',
            'https://futhark-lang.org/blog/2026-10-02-cpu_function.html',
            'https://futhark-lang.org/blog/2026-10-08-end-of-futharkscript.html'],
        new_interpreter_ffi='The October 8 proposal only adds interpreter IO and calls to compiled Futhark entry points. Its foreign effects are unavailable in compiled Futhark, and it is not a compiled C allocator interface.')
    (D/'results/capabilities.json').write_text(json.dumps(report,indent=2)+'\n')
    print('Verified stock C and multicore allocation/lifetime evidence; binarytrees remains excluded.')
if __name__=='__main__':main()

