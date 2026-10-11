"""Select validated D endpoints and export seconds, time × Mech and LOC × Mech."""
import argparse,csv,json,math,statistics
from pathlib import Path
from prepare import D,BENCHES,sha,save
from loc import normalize

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--partial',action='store_true');args=ap.parse_args()
    audit=json.loads((D/'source-audit.json').read_text());report=json.loads((D/'results/timings.json').read_text())
    before=json.loads((D/'evidence/site-before.json').read_text())
    assert report['source_audit_sha256']==sha(D/'source-audit.json')
    controls={r['benchmark']:r for r in report['programs'].values() if r['language']=='mech'}
    comparisons=[];pending=[];metrics={};failures=[]
    for bench in BENCHES:
        candidates=[r for r in report['programs'].values() if r['language']=='d' and r['benchmark']==bench]
        for row in candidates:
            assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
            metric=normalize(Path(row['source']).read_text());assert metric['normalized_loc']==row['loc'];metrics[row['key']]=metric
            if row.get('full_failure'):failures.append(dict(key=row['key'],benchmark=bench,**row['full_failure']))
        done=[r for r in candidates if len(r['samples_seconds'])==report['sample_count'] and not r.get('full_failure')]
        control=controls[bench]
        if len(done)!=len(candidates) or len(control['samples_seconds'])!=report['sample_count']:
            pending.append(bench);continue
        mech_group=next(g for b in before['benchmarks'] if b['key']==bench for g in b['groups'] if g['language']=='mech')
        mfast=before['programs'][mech_group['fast']];msmall=before['programs'][mech_group['small']]
        assert control['source_sha256'] in mfast['files'] and control['loc']==mfast['loc']
        def endpoint(row,profile):
            assert all(v['reference_match'] for v in row['sample_validation'])
            ref=mfast if profile=='fastest' else msmall
            return dict(key=row['key'],seconds=row['median_seconds'],samples_seconds=row['samples_seconds'],
                stdev_seconds=statistics.stdev(row['samples_seconds']),time_x_mech=row['median_seconds']/control['median_seconds'],
                loc=row['loc'],loc_x_mech=row['loc']/ref['loc'],mech_seconds=control['median_seconds'],mech_loc=ref['loc'],
                source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'])
        fast=min(done,key=lambda r:(r['median_seconds'],r['loc']));small=min(done,key=lambda r:(r['loc'],r['median_seconds']))
        comparisons.append(dict(benchmark=bench,candidates=len(done),fastest=endpoint(fast,'fastest'),smallest=endpoint(small,'smallest'),same_source=fast['key']==small['key']))
    if not args.partial:
        assert report['status']=='complete' and not failures and not pending
        assert len(comparisons)==10
        for file,digest in (audit['pins']|audit['libraries']).items():assert sha(file)==digest,file
    policy='Time × Mech uses the freshly measured unchanged fastest Mech AOT control for both D profiles. Fastest LOC compares fastest sources; smallest LOC compares smallest sources. Below 1× means faster or shorter than Mech. Explorer measurements of existing languages are preserved.'
    summary=dict(status='partial' if args.partial else 'passed',comparisons=comparisons,pending=pending,failures=failures,
        controls={b:dict(seconds=r.get('median_seconds'),loc=r['loc'],samples=r['samples_seconds'],key=r['key'],source_sha256=r['source_sha256']) for b,r in controls.items()},
        admitted_sources=sum(len(r['samples_seconds'])==3 for r in report['programs'].values() if r['language']=='d'),
        samples_completed=sum(len(r['samples_seconds']) for r in report['programs'].values()),samples_expected=3*len(report['programs']),
        source_audit_sha256=sha(D/'source-audit.json'),timing_report_sha256=sha(D/'results/timings.json'),
        loc_tool_sha256=sha(D/'loc.py'),summary_script_sha256=sha(Path(__file__)),preserved_explorer_sha256=sha(D/'evidence/site-before.json'),
        loc=metrics,ratio_policy=policy,discarded_samples=len(report['discarded']),compiler=audit['runtime'])
    if comparisons:
        summary['geometric_means']={profile:{field:math.exp(statistics.mean(math.log(c[profile][field]) for c in comparisons)) for field in ('time_x_mech','loc_x_mech')} for profile in ('fastest','smallest')}
    name='progress' if args.partial else 'summary';save(D/f'results/{name}.json',summary)
    with (D/f'results/{name}.csv').open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=['benchmark','profile','seconds','time_x_mech','loc','loc_x_mech','mech_seconds','mech_loc','key']);writer.writeheader()
        for c in comparisons:
            for profile in ('fastest','smallest'):writer.writerow(dict(benchmark=c['benchmark'],profile=profile,**{k:v for k,v in c[profile].items() if k in writer.fieldnames}))
    lines=['# D compared with Mech','',policy,'','Full workloads; medians of three fresh processes. All times are seconds.','',
        '| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |','|---|---|---:|---:|---:|---:|']
    for c in comparisons:
        for profile in ('fastest','smallest'):
            r=c[profile];lines.append(f"| {c['benchmark']} | {profile} | {r['seconds']:.6f} | {r['time_x_mech']:.3f}× | {r['loc']} | {r['loc_x_mech']:.3f}× |")
    (D/f'results/{name}.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps(dict(status=summary['status'],comparisons=len(comparisons),samples=summary['samples_completed'],expected=summary['samples_expected'],pending=pending,failures=failures)))

if __name__=='__main__':main()
