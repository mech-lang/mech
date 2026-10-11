"""Application LOC, required work, and immutable source/library/build pins."""
import hashlib,json,re,subprocess
from pathlib import Path
D=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def loc(path):
    count=0;doc=False
    for line in path.read_text().splitlines():
        text=line.strip()
        if path.suffix=='.mojo' and '"""' in text:
            if text.count('"""')%2:doc=not doc
            continue
        if doc or not text or text.startswith('#' if path.suffix=='.mojo' else '--'):continue
        assert ';' not in text,'Packed statement needs audit: '+str(path)
        count+=1
    return count
REASONS={
'nbody':'Canonical five bodies, momentum offset, all ten symmetric pairs and all five position updates at each of 50,000,000 steps; initial/final energies. No analytic orbit or omitted steps.',
'spectralnorm':'Four required functions are explicit. All 40 matrix/vector products evaluate all N² coefficients during multiplication. No stored coefficient matrix. SIMD/parallelism changes execution only.',
'mandelbrot':'Canonical grid, z=0, at most 50 updates per pixel, ordinary escape check and complete PBM. No cardioid/periodicity pruning. Contracted coordinate arithmetic is disabled to preserve reference pixels.',
'knucleotide':'Extract THREE in the language; all seven widths 1,2,3,4,6,12,18. Every one of 874,999,961 overlapping keys is looked up and updated in growing hash tables, distributed over 32 ranges in eight CPU workers. All keys are retained per range before queries; no selective query counting or histogram substitution.',
'revcomp':'Read full stdin with growing buffers, preserve each header, reverse and complement every sequence and output sixty-column FASTA. No file-size knowledge, cached answers or native DNA executor.',
'fasta':'Canonical 287-byte ALU repetition; every one of 200,000,000 random LCG updates, division by 139968.0 and cumulative floating probability comparisons. No period caching, skip-ahead generation or integer cutoffs.',
'binarytrees':'Two child slots for every node, including leaves. All stretch, temporary and long-lived trees are constructed and recursively traversed; temporary storage is reclaimed/reused by ordinary array containers. No shared leaves or closed-form checks. Futhark barriers force all node stores to remain inside the per-tree iteration; generated C was checked.',
'fannkuchredux':'All 479,001,600 permutations in canonical rotation order; factoradic starting ranks partition only the enumeration. Every prefix reversal, alternating checksum and maximum is computed in the language, using eight CPU workers.',
'regexredux':'Full input cleanup, nine separate complete regex searches, then five ordered global substitutions. Mojo uses general PCRE2; Futhark parsing/matching/substitution is written in Futhark. No combined benchmark pattern executor.',
'pidigits':'Sequential unbounded LFT spigot in the language, published q>r guard, extract(3) and extract(4) checked before each emitted digit. Mojo uses ordinary GMP operations; Futhark uses dynamically growing signed limb arithmetic. No Chudnovsky algorithm or precision bound chosen from N.'}
SUPPORT={
 'mojo':{b:['stdlib/common.mojo'] for b in REASONS},
 'futhark':{b:['stdlib/common.fut','adapters/futhark.c'] for b in REASONS}}
SUPPORT['mojo']['binarytrees']=[]
SUPPORT['mojo']['fannkuchredux']=[]
for b,module in [('pidigits','integer'),('regexredux','regex')]:
    SUPPORT['mojo'][b]+=['stdlib/'+module+'.mojo','stdlib/native.c','stdlib/khash.h']
for b,module in [('pidigits','bigint'),('regexredux','regex'),('knucleotide','hashmap')]:
    SUPPORT['futhark'][b]+=['stdlib/'+module+'.fut']
def main():
    built=json.loads((D/'results/builds.json').read_text());rows=[]
    for r in built:
        source=D/r['source'];assert sha(source)==r['source_sha256']
        assert sha(r['binary'])==r['binary_sha256']
        benchmark=source.stem.split('-')[0]
        rows.append(dict(id=benchmark+':'+r['language']+'-'+source.stem,benchmark=benchmark,language=r['language'],file=r['source'],source=str(source),sha256=sha(source),physical_loc=loc(source),normalized_loc=loc(source),expansions=[],binary=r['binary'],binary_sha256=r['binary_sha256'],algorithm=dict(strict=True,status='source-and-generated-code-reviewed',reason=REASONS[benchmark],native_benchmark_kernel=False),stdlib_files=SUPPORT[r['language']][benchmark],source_provenance=('Pinned published Mojo source adapted to the official workload and four-function requirement.' if source.stem in ('nbody','nbody-small','spectralnorm-small') and r['language']=='mojo' else 'Local faithful CPU port; benchmark computation is in the named language.')))
    assert {(r['benchmark'],r['language']) for r in rows}=={(b,l) for b in REASONS for l in ('mojo','futhark')}
    report=dict(status='required-work-reviewed',policy='Nonblank, noncomment LOC. No packed independent statements. Structural tuples/arrays/loop state remain one language construct. Generic library and IO adapter support is excluded from application LOC and separately inspectable.',rows=rows,
       source_provenance_sha256=sha(D/'provenance.json'),builds_sha256=sha(D/'results/builds.json'),stdlib_test_report_sha256=sha(D/'results/stdlib-tests.json'),libraries={str(p.relative_to(D)):sha(p) for p in list((D/'stdlib').glob('*'))+list((D/'adapters').glob('*')) if p.is_file()},
       runtimes={l:dict(path=p,sha256=sha(p),version=subprocess.check_output([p,'--version'],text=True,stderr=subprocess.STDOUT).strip()) for l,p in [('mojo','/tmp/mech-extra-languages-20261009/mojo-runtime/bin/mojo'),('futhark','/opt/homebrew/bin/futhark')]},
       max_version='26.6.0',futhark_backend='multicore CPU',maximum_workers=8)
    (D/'source-audit.json').write_text(json.dumps(report,indent=2)+'\n')
    for r in rows:print(r['language'],r['file'],r['normalized_loc'],'LOC')
if __name__=='__main__':main()
