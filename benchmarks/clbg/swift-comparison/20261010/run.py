"""Guarded, resumable, sequential Swift measurements at all ten full workloads."""
import argparse,hashlib,importlib.util,json,random,statistics,subprocess
from datetime import datetime,timezone
from pathlib import Path
D=Path(__file__).resolve().parent;CLBG=D.parents[1]
spec=importlib.util.spec_from_file_location('array_runner',CLBG/'array-comparison/all-20261009/run.py')
arrays=importlib.util.module_from_spec(spec);spec.loader.exec_module(arrays)
compact=arrays.compact
compact.COMPILERS|={'swift','swiftc','swift-frontend'}
WORK=arrays.WORK;REQUIRED=arrays.REQUIRED
BENCHES=['spectralnorm','pidigits','nbody','mandelbrot','revcomp','fasta','binarytrees','knucleotide','fannkuchredux','regexredux']
REASONS={
 'nbody':'Canonical five bodies, offset momentum, every symmetric pair and position update at each of 50,000,000 steps; both energies. No omitted steps or analytic orbit.',
 'spectralnorm':'Four required functions; all 40 N-squared matrix/vector products evaluate every coefficient during multiplication. No coefficient/result caching.',
 'mandelbrot':'Every grid pixel executes the ordinary z=0 recurrence with up to 50 escape-tested iterations and complete PBM output. SIMD and CPU scheduling only; no cardioid/periodicity pruning.',
 'knucleotide':'Read THREE in Swift and build all seven complete overlapping-count maps for widths 1,2,3,4,6,12,18. Every position updates a map; no selective queries or histogram shortcut.',
 'revcomp':'Read complete input, preserve headers, reverse/complement every sequence and emit sixty-column FASTA.',
 'fasta':'Canonical ALU repetition, all 200,000,000 fresh LCG updates, binary64 division and cumulative floating CDF comparisons, complete output. No integer cutoffs or subtraction search.',
 'binarytrees':'Allocate every interior and leaf node with two child slots using Swift ARC classes or indirect enum boxes. Release stretch storage before long-lived allocation; keep long-lived tree rooted through all temporary-tree construction/traversal and check afterward. No shared leaf or collapsed iteration loop.',
 'fannkuchredux':'Enumerate all 479,001,600 permutations in canonical rotation order, compute all prefix flips, alternating checksum and maximum. Factoradic tasks only partition complete work.',
 'regexredux':'Read/clean all input; nine complete regex searches and five ordered substitutions using general Foundation regular expressions.',
 'pidigits':'Sequential unbounded LFT spigot in Swift, q>r guard and extract(3)/extract(4) agreement before every digit. GMP provides general arithmetic only.'}
def sha(path):
    with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def execute(row,full,timeout):
    if row['benchmark']=='nbody':return compact.execute(row,full,timeout,watch_compilers=True,expected_power=REQUIRED)
    return arrays.execute(row,full,timeout)
def exclusion(row):
    if row['build_status']!='passed':return 'Build failed; pinned compiler diagnostics retained.'
    if row['id'].startswith('binarytrees-swift-') and not row.get('local_adaptation'):
        if row['id'] in ('binarytrees-swift-1','binarytrees-swift-5'):
            return 'The original stretch tree remains globally rooted during temporary work. The current description requires releasing it before long-lived allocation; a corrected Swift candidate is retained separately.'
        return 'The original long-lived tree is checked in an independently scheduled closure, whose lifetime ends before temporary work is guaranteed complete. The current description requires keeping it alive throughout temporary work; a corrected Swift candidate is retained separately.'
    if row['id'].startswith('fasta-swift-') and not row.get('local_adaptation'):
        return {
            'fasta-swift-1':'Reciprocal multiplication replaced with explicit binary64 division for our common sampling workload. This is a cohort comparability decision, not a blanket claim that this submission violates CLBG.',
            'fasta-swift-2':'Binary32 sampling replaced with binary64 sampling and explicit division for our common workload. This is a cohort comparability decision, not a blanket claim that this submission violates CLBG.',
            'fasta-swift-3':'Integer-scaled cumulative cutoffs conflict with the current FASTA description, which explicitly prohibits lookup scaling. A separate corrected candidate uses binary64 probabilities and division.',
            'fasta-swift-8':'Probability subtraction does not implement the cumulative-probability lookup in the current FASTA description. A separate corrected candidate builds and searches the cumulative distribution.'
        }[row['id']]
    return None
def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--samples',type=int,default=3);ap.add_argument('--timeout',type=int,default=7200)
    ap.add_argument('--benchmarks',default=','.join(BENCHES));ap.add_argument('--validate-only',action='store_true')
    args=ap.parse_args();assert args.samples>=3
    built=json.loads((D/'results/builds.json').read_text())
    path=D/'results/timings.json'
    audit=json.loads((D/'source-audit.json').read_text())
    assert audit['status']=='reviewed' and audit['builds_sha256']==sha(D/'results/builds.json')
    reviewed={r['key']:r for r in audit['rows']}
    initial_rows={};excluded=[]
    for r in built['programs']:
        reason=exclusion(r)
        if reason:
            excluded.append(dict(id=r['id'],benchmark=r['benchmark'],language='swift',reason=reason,
                                 source=r['source'],source_sha256=r['source_sha256'],source_url=r['source_url']))
            continue
        row=r.copy();row.update(loc=r['source_metrics']['nonblank_noncomment_lines'],physical_loc=r['source_metrics']['nonblank_noncomment_lines'],
             loc_expansions=[],algorithm=dict(strict=True,status='required-work-reviewed',reason=REASONS[r['benchmark']],native_benchmark_kernel=False),
             command=[r['binary']]+([] if r['benchmark'] in ('knucleotide','revcomp','regexredux') else ['{n}']),
             samples_seconds=[],sample_validation=[],eligible_for_chart=True)
        initial_rows[row['key']]=row
        assert reviewed[row['key']]['admitted'] and reviewed[row['key']]['sha256']==row['source_sha256']
    for row in audit['controls']:
        row=row.copy();row.update(samples_seconds=[],sample_validation=[],eligible_for_chart=True)
        initial_rows[row['key']]=row
    report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),
       sample_count=args.samples,programs=initial_rows,excluded=excluded,completed_benchmarks=[],orders={},discarded=[],
       machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),
       os=subprocess.check_output(['sw_vers'],text=True).strip(),power=compact.power_profile(),workloads=WORK,
       compiler=built['compiler'],compiler_flags=built['flags'],libraries=built['libraries'],support_files=built['support_files'],
       source_pins={f:sha(D/f) for f in ['manifest.json','corrections.json','results/builds.json','source-audit.json']},
       protocol='Every admitted Swift candidate and one current fastest Mech AOT control per benchmark have three shuffled sequential fresh-process full-workload samples. Startup, full input, computation, complete stdout to a regular file, and teardown are timed; compilation and output validation/hashing are excluded. AC power, Low Power Mode off, eight logical CPUs; published Swift Dispatch scheduling is retained (task counts can exceed core counts). Competing compiler and power changes invalidate a sample. Correct output and prescribed algorithm work are both required. Smallest means nonblank/noncomment LOC after separating packed statements; no gzip selection.')
    assert report['sample_count']==args.samples
    assert report['source_pins']['results/builds.json']==sha(D/'results/builds.json')
    def save():
        temp=path.with_suffix('.json.tmp');temp.write_text(json.dumps(report,indent=2)+'\n');temp.replace(path)
    def pins():
        for f,h in report['source_pins'].items():assert sha(D/f)==h,f
        assert sha(report['compiler']['path'])==report['compiler']['sha256']
        for kind in ('driver','frontend'):
            assert sha(report['compiler'][kind+'_path'])==report['compiler'][kind+'_sha256']
        for f,h in report['libraries'].items():assert sha(f)==h,f
        for f,h in report['support_files'].items():assert sha(D/f)==h,f
    save();pins()
    selected=args.benchmarks.split(',');assert set(selected)<=set(BENCHES)
    for bench in selected:
        rows=[p for p in report['programs'].values() if p['benchmark']==bench and p['eligible_for_chart']]
        for row in rows:
            assert sha(row['source'])==row['source_sha256'];assert sha(row['binary'])==row['binary_sha256']
            for full,label in [(False,'small'),(True,'full')]:
                if row.get(label+'_validation'):continue
                compact.wait_for_environment(REQUIRED)
                print('VALIDATE',label,row['id'],flush=True)
                try:result=execute(row,full,args.timeout)
                except Exception as error:
                    if row['language']=='mech':raise
                    row['eligible_for_chart']=False;row['algorithm']['strict']=False
                    reason=str(error);row['exclusion_reason']=reason
                    report['excluded'].append(dict(id=row['id'],benchmark=bench,language='swift',stage=label+' output validation',reason=reason,source=row['source'],source_sha256=row['source_sha256'],source_url=row['source_url']))
                    print('EXCLUDED',row['id'],reason[:250],flush=True);save();break
                row[label+'_validation']=result['validation'];row[label+'_validation_seconds']=result['seconds']
                print('PASS',label,row['id'],f'{result["seconds"]:.6f}s',flush=True);save()
        rows=[row for row in rows if row['eligible_for_chart']]
        assert rows,'No admitted Swift candidates for '+bench
        if args.validate_only:continue
        rng=random.Random(20261010+sum(map(ord,bench)))
        for round_no in range(args.samples):
            order=rows.copy();rng.shuffle(order)
            report['orders'].setdefault(bench,[])
            if len(report['orders'][bench])<=round_no:report['orders'][bench].append([r['key'] for r in order])
            else:assert report['orders'][bench][round_no]==[r['key'] for r in order]
            for row in order:
                if len(row['samples_seconds'])>round_no:continue
                while True:
                    compact.wait_for_environment(REQUIRED);pins()
                    assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
                    print('RUN',round_no+1,row['id'],flush=True)
                    result=execute(row,True,args.timeout)
                    if result['competing_compilers'] or result['power_changes']:
                        report['discarded'].append(dict(key=row['key'],round=round_no+1,result=result));save();continue
                    row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation'])
                    row['median_seconds']=statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds'])>1:row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
                    print('SAMPLE',round_no+1,row['id'],f'{result["seconds"]:.6f}s',flush=True);save();break
        if bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
        save()
    report['status']='validated' if args.validate_only else 'complete' if set(report['completed_benchmarks'])==set(BENCHES) else 'partial'
    report['finished_at']=datetime.now(timezone.utc).isoformat();pins();save()
    print('REPORT',path,report['status'],flush=True)
if __name__=='__main__':main()
