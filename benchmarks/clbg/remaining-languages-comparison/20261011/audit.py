"""Freeze complete application sources, runtimes and native artifacts before timing."""
import collections
import copy
import json
from datetime import datetime,timezone
from pathlib import Path
from dotnet_fetch import C,D,sha
from metrics import normalize,fixtures
from probe import save,WORK

MANIFESTS=['dotnet_manifest.json','ada_pascal_manifest.json','fortran_dart_manifest.json',
    'smalltalk_manifest.json','toit_manifest.json','micropython_manifest.json']
BOUNDARY=('Sequential fresh processes in randomized rounds, three admitted samples per candidate. '
    'Full prescribed workloads and complete output to regular files; process startup, JIT compilation, '
    'image loading, standard libraries, workers and output are included. Ahead-of-time compilation, '
    'source audits, output hashing and comparisons are outside timing. AC power with Low Power Mode off. '
    'Samples overlapping competing compilers or power changes are discarded and retried. '
    'At most eight application workers; native ARM64 execution. No scaled timings.')

def main():
    fixtures()
    programs=[];runtimes={};libraries={};pin_files={};manifest_pins={};pool=[]
    for filename in MANIFESTS:
        path=D/filename;d=json.loads(path.read_text());assert d['status']=='prepared',filename
        manifest_pins[str(path)]=sha(path)
        runtimes.update(d['runtimes']);libraries.update(d['libraries'])
        ids=set(d.get('recommended_timing_ids',[]))
        for original in d['programs']:
            # Preserve compiled alternates, but display only fully measured endpoints.
            if ids and original['id'] not in ids:
                pool.append(dict(id=original['id'],reason='Additional compiled alternate outside compact/published-fast shortlist.'));continue
            if original['id'] in ('nbody-micropython-8','mandelbrot-micropython-8'):
                pool.append(dict(id=original['id'],reason='Larger scalar alternate; compact and optimized candidates retained.'));continue
            r=copy.deepcopy(original)
            assert r['small_validation'].get('reference_match'),r['id']
            assert r.get('algorithm',{}).get('strict',r.get('strict',False)),r['id']
            for field in ('source','binary'):
                assert sha(r[field])==r[field+'_sha256'],(r['id'],field)
            metrics=normalize(Path(r['source']).read_text(),r['language'])
            r.update(loc=metrics['normalized_loc'],physical_loc=metrics['physical_loc'],
                normalized_loc=metrics['normalized_loc'],loc_expansions=metrics['expansions'])
            r['eligible_for_chart']=True
            r.setdefault('key',r['benchmark']+':'+r['id'])
            # Independent application units are included in the aggregate source,
            # and pin the actual units that the compiler consumed as well.
            for path,digest in r.get('compiled_source_files',{}).items():
                assert sha(path)==digest,path;pin_files[path]=digest
            if r['language'].startswith(('csharp','fsharp')):
                r.setdefault('runtime_environment',{})['DOTNET_ThreadPool_ForceMaxWorkerThreads']='8'
            programs.append(r)
    controls=copy.deepcopy(json.loads((C/'lisp-racket-comparison/20261010/source-audit.json').read_text())['controls'])
    for r in controls:
        for field in ('source','binary'):assert sha(r[field])==r[field+'_sha256']
    for path,digest in libraries.items():assert sha(path)==digest,path
    for r in runtimes.values():assert sha(r['path'])==r['sha256'],r['path']
    evidence=[D/'results/root-canaries.json',D/'support/ada-pascal/adversarial-report.json',
        D/'support/ada-pascal/tree-allocation-audit.json',D/'support/smalltalk/adversarial-checks.json',
        D/'support/toit/adversarial-checks.json',D/'support/fortran-dart/adversarial-checks.json']
    for p in evidence:
        if p.exists():pin_files[str(p)]=sha(p)
    for path in (D/'audit.py',D/'run.py',D/'probe.py',D/'metrics.py',D/'support/toit/loc_mask.py',
        C/'compact/run.py',C/'array-comparison/all-20261009/run.py',
        C/'knucleotide/.build/knucleotide-input25000000.txt',C/'regexredux/.build/input5000000.txt'):
        pin_files[str(path)]=sha(path)
    pin_files.update(manifest_pins)
    for language in {p['language'] for p in programs}:
        assert {p['benchmark'] for p in programs if p['language']==language}==set(WORK),language
    assert len({r['key'] for r in programs+controls})==len(programs+controls)
    report=dict(status='reviewed',frozen_at=datetime.now(timezone.utc).isoformat(),programs=programs,controls=controls,
        runtimes=runtimes,libraries=libraries,pin_files=pin_files,timing_boundary=BOUNDARY,
        coverage=dict(collections.Counter(r['language'] for r in programs)),unmeasured_alternates=pool,
        loc_policy='Nonblank, noncomment application lines; independent packed assignments/statements expanded. '
        'All application helpers and FFI declarations counted. General library implementations excluded uniformly.',
        requirements='Full seven growable nucleotide maps with collision-free 36-bit keys; binary64 arithmetic and full '
        'Fasta random steps/CDF decisions; both pi extractions for every tested candidate (generic impossible-candidate '
        'guard allowed equally); distinct equal-size leaf/interior tree records retained and traversed; all permutations '
        'and pixels; full regex counts and ordered substitutions; no specialized native benchmark kernels.')
    save(D/'source-audit.json',report)
    print('FROZEN',len(programs),'candidates',len(controls),'Mech controls',report['coverage'],flush=True)

if __name__=='__main__':main()
