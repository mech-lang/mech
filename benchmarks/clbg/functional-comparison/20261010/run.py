"""Guarded, resumable, sequential full-size functional-language measurements."""
import argparse,hashlib,importlib.util,json,random,statistics,subprocess
from datetime import datetime,timezone
from pathlib import Path
D=Path(__file__).resolve().parent;CLBG=D.parents[1]
spec=importlib.util.spec_from_file_location('array_runner',CLBG/'array-comparison/all-20261009/run.py')
arrays=importlib.util.module_from_spec(spec);spec.loader.exec_module(arrays)
compact=arrays.compact
compact.COMPILERS|={'ghc','ghc-9.10.3','cabal','ocamlc','ocamlc.opt','ocamlopt','ocamlopt.opt','ocamlfind','dune','opam','llc','opt'}
WORK=arrays.WORK;REQUIRED=arrays.REQUIRED
BENCHES=['spectralnorm','pidigits','nbody','mandelbrot','revcomp','fasta','binarytrees','knucleotide','fannkuchredux','regexredux']
REASONS={
 'nbody':'Canonical five bodies, offset momentum, all ten symmetric interactions and five position updates at each of 50,000,000 steps; both energies.',
 'spectralnorm':'Four coefficient/multiplication/transposed/composed functions; all 40 N-squared products evaluate every coefficient during multiplication. No matrix caching.',
 'mandelbrot':'Every pixel uses the ordinary recurrence and up to 50 escape-tested iterations; complete PBM output. Published SIMD and CPU scheduling retained; no interior or periodicity pruning.',
 'knucleotide':'Extract THREE; seven complete overlapping count maps for widths 1,2,3,4,6,12,18. Grow standard hash maps from small initial sizes, lookup/update every valid position; no selective query shortcut.',
 'revcomp':'Read complete input, preserve headers, reverse/complement every sequence and emit sixty-column FASTA.',
 'fasta':'Canonical ALU repetition, all 200,000,000 fresh LCG updates, binary64 division and cumulative floating CDF comparisons, complete output. No scaled-integer lookup, period caching, or subtraction search.',
 'binarytrees':'Fully construct every interior and leaf node with identical node allocation, then traverse each complete tree. Release stretch storage; keep the complete long-lived tree rooted through temporary work and check afterward. Opaque standard compiler barriers prevent leaf sharing; no custom pool or collapsed loop.',
 'fannkuchredux':'All 479,001,600 permutations in canonical rotation order, complete prefix flips, alternating checksum and maximum. Factoradic tasks partition complete rank intervals only.',
 'regexredux':'Read/clean all input; nine complete non-overlapping regex searches and five ordered substitutions. General PCRE2/Str/Re libraries only; control flow remains in the language source.',
 'pidigits':'Single-threaded unbounded LFT spigot using arbitrary-precision arithmetic and both extract(3) and extract(4). Haskell Integer or general GMP/Zarith bindings provide arithmetic; no native spigot executor.'}
def sha(path):
    with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def execute(row,full,timeout):
    if row['benchmark']=='nbody':return compact.execute(row,full,timeout,watch_compilers=True,expected_power=REQUIRED)
    return arrays.execute(row,full,timeout)
def exclusion(row):
    if row['build_status']!='passed':return 'Build failed; pinned compiler diagnostics retained.'
    if row.get('local_adaptation'):return None
    if row['benchmark']=='binarytrees':return 'Original can share leaf nodes, eliminate allocation work, or precompute the long-lived check. A separately labelled corrected source preserves the required allocation and lifetime.'
    adapted={
      'fasta-ghc-2':'Binary32 sampling differs from the common binary64 cohort workload.',
      'fasta-ghc-4':'Integer-scaled probability lookup is prohibited by the current FASTA description.',
      'fasta-ghc-5':'Integer-scaled probability lookup is prohibited by the current FASTA description.',
      'fasta-ghc-6':'Integer-scaled probability lookup is prohibited by the current FASTA description.',
      'fasta-ocaml-6':'Reciprocal multiplication replaced with direct division for cohort comparability; not a blanket CLBG violation.',
      'knucleotide-ocaml-1':'Original preallocates 262,144 slots instead of growing from a small initial table.',
      'knucleotide-ocaml-2':'Original omits the final window and preallocates 262,144 hash slots.',
      'knucleotide-ocaml-3':'Original omits the final window and preallocates 262,144 hash slots.',
      'knucleotide-ghc-1':'Original fails when a requested key is absent; separately corrected for small-input correctness.',
      'mandelbrot-ghc-2':'Original starts byte counter at one and omits the final byte of each row.',
      'mandelbrot-ghc-3':'Original truncates each 64-pixel packed block to one byte.',
      'fannkuchredux-ocaml-3':'Original miscounts identity flips and advances beyond its final assigned permutation.',
      'fannkuchredux-ocaml-4':'Original counts one flip for permutations already headed by zero.'}
    return adapted.get(row['id'])
def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--samples',type=int,default=3);ap.add_argument('--timeout',type=int,default=14400)
    ap.add_argument('--benchmarks',default=','.join(BENCHES));args=ap.parse_args();assert args.samples>=3
    built=json.loads((D/'results/builds.json').read_text());audit=json.loads((D/'source-audit.json').read_text())
    assert audit['status']=='reviewed' and audit['builds_sha256']==sha(D/'results/builds.json')
    reviewed={r['key']:r for r in audit['rows']};initial={};excluded=[]
    for r in built['programs']:
        reason=exclusion(r)
        if reason:excluded.append(dict(id=r['id'],benchmark=r['benchmark'],language=r['language'],reason=reason,source=r['source'],source_sha256=r['source_sha256'],source_url=r['source_url']));continue
        a=r['loc_audit'];row=r.copy();row.update(loc=a['normalized_loc'],physical_loc=a['physical_loc'],loc_expansions=a['expansions'],
            algorithm=dict(strict=True,status='required-work-reviewed',reason=REASONS[r['benchmark']],native_benchmark_kernel=False),samples_seconds=[],sample_validation=[],eligible_for_chart=True)
        assert reviewed[row['key']]['admitted'] and reviewed[row['key']]['sha256']==row['source_sha256'];initial[row['key']]=row
    for r in audit['controls']:
        row=r.copy();row.update(samples_seconds=[],sample_validation=[],eligible_for_chart=True);initial[row['key']]=row
    path=D/'results/timings.json'
    report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),sample_count=args.samples,
        programs=initial,excluded=excluded,completed_benchmarks=[],orders={},discarded=[],machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),
        os=subprocess.check_output(['sw_vers'],text=True).strip(),power=compact.power_profile(),workloads=WORK,compilers=built['compilers'],llvm=built['llvm'],
        libraries=built['libraries'],support_files=built['support_files'],source_pins={f:sha(D/f) for f in ['fetch.py','prepare.py','build.py','loc.py','run.py','audit.py','manifest.json','corrections.json','results/builds.json','source-audit.json']},
        protocol='Every admitted Haskell/OCaml candidate and an unchanged Mech AOT control have three shuffled sequential fresh-process full-workload samples per benchmark. Startup, complete input, computation, full stdout to a regular file and teardown are timed; compilation, validation and hashing are excluded. Apple M3, AC power, Low Power Mode off, eight logical CPUs. Haskell uses eight runtime capabilities; OCaml published fork/task partitioning is retained. Default GC: no heap-tuning flags in tree runs. Competing compiler/power changes invalidate samples. Small and full output checks and source/compiler work review are required. LOC excludes blanks/comments, includes semantic pragmas, and counts packed independent statements/bindings separately. General standard libraries only; no native benchmark executor. Published Linux times are never reused.')
    assert report['sample_count']==args.samples
    def save():
        tmp=path.with_suffix('.json.tmp');tmp.write_text(json.dumps(report,indent=2)+'\n');tmp.replace(path)
    def pins():
        for f,h in report['source_pins'].items():assert sha(D/f)==h,f
        for r in report['compilers'].values():assert sha(r['path'])==r['sha256']
        h=report['compilers']['haskell'];assert sha(h['frontend_path'])==h['frontend_sha256']
        assert sha(report['llvm']['path'])==report['llvm']['sha256']
        for f,h in report['libraries'].items():assert sha(f)==h,f
        for f,h in report['support_files'].items():assert sha(D/f)==h,f
    selected=args.benchmarks.split(',');assert set(selected)<=set(BENCHES);save();pins()
    for bench in selected:
        rows=[r for r in report['programs'].values() if r['benchmark']==bench and r['eligible_for_chart']]
        for row in rows:
            assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
            for full,label in [(False,'small'),(True,'full')]:
                if row.get(label+'_validation'):continue
                compact.wait_for_environment(REQUIRED);pins();print('VALIDATE',label,row['id'],flush=True)
                try:result=execute(row,full,args.timeout)
                except Exception as err:
                    if row['language']=='mech':raise
                    row['eligible_for_chart']=False;row['algorithm']['strict']=False;row['exclusion_reason']=str(err)
                    report['excluded'].append(dict(id=row['id'],benchmark=bench,language=row['language'],stage=label+' output validation',reason=str(err),source=row['source'],source_sha256=row['source_sha256'],source_url=row['source_url']))
                    print('EXCLUDED',row['id'],str(err)[:240],flush=True);save();break
                if result['competing_compilers'] or result['power_changes']:raise RuntimeError('Environment changed during validation; rerun to retry.')
                row[label+'_validation']=result['validation'];row[label+'_validation_seconds']=result['seconds'];print('PASS',label,row['id'],f'{result["seconds"]:.6f}s',flush=True);save()
        rows=[r for r in rows if r['eligible_for_chart']]
        assert {'haskell','ocaml','mech'}<=set(r['language'] for r in rows),bench
        rng=random.Random(20261010+sum(map(ord,bench)))
        for round_no in range(args.samples):
            order=rows.copy();rng.shuffle(order);report['orders'].setdefault(bench,[])
            if len(report['orders'][bench])<=round_no:report['orders'][bench].append([r['key'] for r in order])
            else:assert report['orders'][bench][round_no]==[r['key'] for r in order]
            for row in order:
                if len(row['samples_seconds'])>round_no:continue
                while True:
                    compact.wait_for_environment(REQUIRED);pins();assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
                    print('RUN',round_no+1,row['id'],flush=True);result=execute(row,True,args.timeout)
                    if result['competing_compilers'] or result['power_changes']:report['discarded'].append(dict(key=row['key'],round=round_no+1,result=result));save();continue
                    row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation']);row['median_seconds']=statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds'])>1:row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
                    print('SAMPLE',round_no+1,row['id'],f'{result["seconds"]:.6f}s',flush=True);save();break
        if bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
        save()
    report['status']='complete' if set(report['completed_benchmarks'])==set(BENCHES) else 'partial';report['finished_at']=datetime.now(timezone.utc).isoformat();pins();save();print('REPORT',path,report['status'])
if __name__=='__main__':main()
