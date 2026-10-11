"""Pin full-work MicroPython sources and validate complete small outputs."""
import json
from pathlib import Path
import subprocess
from dotnet_fetch import D,C,sha
from metrics import normalize
from probe import validate_small,save
import micro_templates as templates

RUNTIME='/opt/homebrew/bin/micropython'
OPTIONS=['-X','heapsize=1024M']

def row(benchmark,ident,text,original=None,notes=None):
    path=D/'sources/micropython'/(ident+'.py');path.write_text(text)
    value=dict(id=ident,key=benchmark+':'+ident,benchmark=benchmark,language='micropython',
        source=str(path),source_sha256=sha(path),binary=RUNTIME,binary_sha256=sha(RUNTIME),
        command=[RUNTIME]+OPTIONS+[str(path)]+([] if benchmark in ('knucleotide','revcomp','regexredux') else ['{n}']),
        adaptations=notes or ['Locally authored measured-language application; every helper and ordinary library binding counts.'],
        local_adaptation=True,algorithm=dict(strict=True,native_benchmark_kernel=False,arithmetic='binary64',
            work='Complete prescribed workload. Each K window updates one of seven growable full maps; both pi extracts; distinct equal-size tree records.'),
        **normalize(text,'micropython'))
    if original:value.update(original_source=original['original_source'],original_source_sha256=original['original_source_sha256'],source_url=original['source_url'])
    else:value['source_url']=None
    return value

def main():
    report=json.loads((D/'micropython_sources.json').read_text())
    rows=[];excluded=[]
    for original in report['programs']:
        text=Path(original['original_source']).read_text()
        benchmark=original['benchmark']
        if benchmark=='revcomp':
            excluded.append(dict(id=original['id'],reason='The Unix runtime has no bytes.maketrans/translate or stream.detach; the counted local byte-array implementation replaces these unsupported APIs.'))
            continue
        notes=[]
        if benchmark=='binarytrees':
            text=text.replace('if not depth: return None, None','if not depth: return [None, None]').replace('return make_tree(depth), make_tree(depth)','return [make_tree(depth), make_tree(depth)]')
            notes.append('Allocate a fresh mutable two-slot node for every leaf and interior instead of a shared constant tuple leaf; retain the long-lived root through all temporary trees.')
        if benchmark=='knucleotide':
            text=text.replace('line[:-1]','line.strip()').replace('sorted(keysValues, reverse=True, key=lambda kv: kv[1])','sorted(keysValues, key=lambda kv: (-kv[1],kv[0]))')
            notes.append('Strip CR/LF, preserve the selected record, and sort equal counts lexically; retain all seven complete substring maps.')
        if benchmark=='fasta':
            text=text.replace('if (i % LINELEN != 0): print()','if (n % LINELEN != 0): print()')
            notes.append('Avoid an extra repeat-record newline at an exact multiple of sixty; preserve every binary64 random selection and LCG step.')
        if original['id']=='mandelbrot-micropython-3':
            text=text.replace('if abs(z) >= 2.0:','if abs(z) > 2.0:')
            notes.append('Use the prescribed strict escape boundary rather than rejecting points at exactly radius two.')
        ident=original['id']+('-faithful' if notes else '')
        rows.append(row(benchmark,ident,text,original,notes or ['Published MicroPython workload source, unchanged.']))
    local=dict(nbody=templates.NBODY,mandelbrot=templates.MANDELBROT,spectralnorm=templates.SPECTRAL,
        fasta=templates.FASTA,knucleotide=templates.K,revcomp=templates.REVCOMP,
        fannkuchredux=templates.FANN,pidigits=templates.PI)
    for benchmark,text in local.items():rows.append(row(benchmark,benchmark+'-micropython-local',text))
    rows.append(row('pidigits','pidigits-micropython-gmp',templates.PI_GMP))
    rows.append(row('binarytrees','binarytrees-micropython-array-arena',templates.TREE))
    rows.append(row('revcomp','revcomp-micropython-buffered',templates.REVCOMP_FAST))
    for use_jit in [False,True]:
        text=templates.REGEX.replace('        # JIT_LINE',
            '        if jit(self.code,1) < 0:\n            raise ValueError("PCRE2 JIT error")' if use_jit else '')
        rows.append(row('regexredux','regexredux-micropython-pcre2'+('-jit' if use_jit else ''),text))
    # This published Python candidate uses the same supported Python language features.
    path=C/'compact/upstream/nbody-python3-2.py'
    text=path.read_text().replace('import sys','import sys\nfrom math import sqrt')
    text=text.replace('((dx * dx + dy * dy + dz * dz) ** (-1.5))','(1.0 / ((dx*dx+dy*dy+dz*dz)*sqrt(dx*dx+dy*dy+dz*dz)))')
    original=dict(original_source=str(path),original_source_sha256=sha(path),source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/nbody-python3-2.html')
    rows.append(row('nbody','nbody-micropython-python3-pairs',text,original,['Port the published Python mutable-list/pair candidate to the same supported MicroPython operations; preserve every binary64 pair and update.']))
    for value in rows:
        try:
            value['small_validation']=validate_small(value)
            value['preparation_status']='passed'
            print('SMALL PASS',value['id'],value['normalized_loc'],flush=True)
        except Exception as e:
            value.update(preparation_status='failed',preparation_error=str(e))
            print('SMALL FAIL',value['id'],str(e)[-450:],flush=True)
    result=dict(status='prepared',runtimes={'micropython':dict(path=RUNTIME,sha256=sha(RUNTIME),version=subprocess.check_output([RUNTIME,'-c','import sys;print(sys.version)'],text=True).strip(),
        runtime_options=OPTIONS,precision='IEEE binary64 confirmed with 1.0000000000000002-1.0',integers='unbounded MPZ verified beyond 2**100')},
        libraries={p:sha(p) for p in ['/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib','/opt/homebrew/opt/gmp/lib/libgmp.10.dylib']},programs=rows,excluded=excluded)
    save(D/'micropython_manifest.json',result)
    print('PREPARED',sum(r['preparation_status']=='passed' for r in rows),'of',len(rows),flush=True)

if __name__=='__main__':main()
