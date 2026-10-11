"""Freeze audited D candidates and unchanged Mech controls before measurements."""
import copy,hashlib,importlib.util,json,subprocess
from pathlib import Path
from datetime import datetime,timezone
from loc import normalize,fixtures

D=Path(__file__).resolve().parent
C=D.parents[1]
MANIFESTS=['numerical-manifest.json','sequence-manifest.json','tree-digit-manifest.json']
BENCHES=['pidigits','spectralnorm','nbody','mandelbrot','regexredux','binarytrees','fannkuchredux','fasta','knucleotide','revcomp']
spec=importlib.util.spec_from_file_location('arrays',C/'array-comparison/all-20261009/run.py')
arrays=importlib.util.module_from_spec(spec);spec.loader.exec_module(arrays)
compact=arrays.compact
compact.COMPILERS.update(['ldc2','ldc','dmd','gdc'])

def sha(path):
    with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    tmp=path.with_suffix('.json.tmp');tmp.write_text(json.dumps(value,indent=2)+'\n');tmp.replace(path)

def execute(row,full,timeout=14400):
    old=compact.ROOT;compact.ROOT=Path(row.get('cwd',old))
    try:
        if row['benchmark']=='nbody':return compact.execute(row,full,timeout,True,arrays.REQUIRED)
        return arrays.execute(row,full,timeout)
    finally:compact.ROOT=old

def main():
    fixtures();programs=[];libraries={};exclusions=[]
    for filename in MANIFESTS:
        manifest=json.loads((D/filename).read_text())
        libraries.update(manifest.get('libraries',{}));exclusions+=manifest.get('excluded',[])
        for original in manifest['programs']:
            row=copy.deepcopy(original)
            row.setdefault('language','d');row.setdefault('key',row['id'] if row['id'].startswith(row['benchmark']+':') else row['benchmark']+':'+row['id'])
            assert row['algorithm']['strict'] and row['algorithm']['native_benchmark_kernel'] is False
            for field in ('source','binary'):
                digest=sha(row[field]);assert not row.get(field+'_sha256') or row[field+'_sha256']==digest
                row[field+'_sha256']=digest
            if row.get('upstream_source'):row['upstream_sha256']=sha(row['upstream_source'])
            metric=normalize(Path(row['source']).read_text())
            row.update(loc=metric['normalized_loc'],physical_loc=metric['physical_loc'],loc_expansions=metric['expansions'],eligible_for_chart=True)
            programs.append(row)
    assert {r['benchmark'] for r in programs}==set(BENCHES)
    controls=copy.deepcopy(json.loads((C/'remaining-languages-comparison/20261011/source-audit-v2.json').read_text())['controls'])
    assert {r['benchmark'] for r in controls}==set(BENCHES)
    for row in controls:
        for field in ('source','binary'):assert sha(row[field])==row[field+'_sha256']
    # Small correctness is preparation, not an admitted performance sample.
    small={'status':'running','programs':{}}
    before=compact.COMPILERS.copy();compact.COMPILERS.clear()
    try:
        for row in programs:
            print('SMALL',row['id'],flush=True)
            result=execute(row,False,120)
            row['small_validation']=result['validation'];small['programs'][row['key']]=result
            save(D/'results/small-checks.json',small)
    finally:compact.COMPILERS.update(before)
    small['status']='passed';save(D/'results/small-checks.json',small)
    compiler='/opt/homebrew/bin/ldc2'
    runtime=dict(path=compiler,sha256=sha(compiler),version=subprocess.check_output([compiler,'--version'],text=True).split('Registered Targets:')[0].strip())
    pins={str(D/f):sha(D/f) for f in MANIFESTS+['prepare.py','run.py','loc.py','results/small-checks.json']}
    pins[str(D/'evidence/site-before.json')]=sha(D/'evidence/site-before.json')
    for filename in MANIFESTS:
        manifest=json.loads((D/filename).read_text())
        for field in ('allocation_audit','work_audit'):
            evidence=manifest.get(field)
            if isinstance(evidence,str) and Path(evidence).is_file():pins[evidence]=sha(evidence)
            elif isinstance(evidence,dict) and evidence.get('path'):
                pins[evidence['path']]=sha(evidence['path'])
    for file in (D/'results').glob('*adversarial.json'):pins[str(file)]=sha(file)
    for file in (D/'audit').rglob('*.json'):pins[str(file)]=sha(file)
    pins[str(D/'numerical-odd-size-audit.json')]=sha(D/'numerical-odd-size-audit.json')
    for file in [C/'compact/run.py',C/'array-comparison/all-20261009/run.py',compact.FULL_INPUT,C/'regexredux/.build/input5000000.txt']:
        pins[str(file)]=sha(file)
    for file,digest in libraries.items():assert sha(file)==digest
    audit=dict(status='reviewed',frozen_at=datetime.now(timezone.utc).isoformat(),programs=programs,controls=controls,runtime=runtime,libraries=libraries,pins=pins,excluded=exclusions,
        workloads=arrays.WORK,loc_policy='Nonblank, noncomment D application LOC; packed independent statements and initialized bindings expanded. All application helpers and FFI counted; ordinary libraries excluded uniformly.',
        timing_boundary='Three sequential shuffled fresh-process samples of the unchanged full workloads. Startup, runtime libraries, workers, input, computation, all output to regular files and shutdown included. Ahead-of-time compilation, output validation and hashing excluded. Native ARM64, at most eight workers, AC power and Low Power Mode off. Compiler/power overlap invalidates and retries a sample; no scaled timings.')
    save(D/'source-audit.json',audit);print('FROZEN',len(programs),'D candidates',len(controls),'Mech controls',flush=True)

if __name__=='__main__':main()
