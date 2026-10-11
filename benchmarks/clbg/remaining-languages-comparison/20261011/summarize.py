"""Export validated full-workload endpoints in seconds and ratios to fresh Mech controls."""
import argparse
import csv
import json
import math
import statistics
from pathlib import Path
from dotnet_fetch import C,D,sha
from metrics_final import normalize,fixtures

LABELS={'csharp':'C# (JIT)','csharp-aot':'C# (AOT)','fsharp':'F# (JIT)',
    'fsharp-aot':'F# (AOT)','ada':'Ada','pascal':'Pascal','fortran':'Fortran',
    'dart':'Dart (AOT)','dart-jit':'Dart (JIT)','smalltalk':'Smalltalk (Pharo)'}
BENCHMARKS=('nbody','mandelbrot','knucleotide','spectralnorm','revcomp','fannkuchredux',
    'fasta','binarytrees','regexredux','pidigits')

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--partial',action='store_true');args=ap.parse_args()
    fixtures();audit_path=D/'source-audit-v2.json';audit=json.loads(audit_path.read_text())
    before_path=D/'evidence/site-before.json';before=json.loads(before_path.read_text())
    selected=[r for r in audit['programs'] if r['language'] in LABELS]
    review={r['key']:r for r in selected+audit['controls']}
    audits={audit_path.name:audit};runtimes=dict(audit['runtimes'])
    reports={};controls={};rows={};pending=[];failures=[];final_loc={};repairs=[]
    for name in ('native-timings-v2.json','ada-regex-repair.json'):
        path=D/'results'/name
        if not path.exists():continue
        report=json.loads(path.read_text());reports[name]=report
        report_audit_path=Path(report['source_audit'])
        assert report_audit_path.parent==D
        report_audit=json.loads(report_audit_path.read_text())
        assert report_audit['status']=='reviewed' and report['source_audit_sha256']==sha(report_audit_path)
        audits[report_audit_path.name]=report_audit;runtimes.update(report_audit['runtimes'])
        repairs.extend(report_audit.get('repairs',[]))
        report_review={r['key']:r for r in report_audit['programs']+report_audit['controls']}
        review.update(report_review)
        assert report['sample_count']==3 and report['power']==dict(source='AC Power',low_power_mode=0)
        runtimes.pop('micropython',None)
        if not args.partial:
            assert report['status']=='complete',name
            assert set(report['completed_benchmarks'])==set(report_audit.get('expected_benchmarks',BENCHMARKS))
        for key,row in report['programs'].items():
            check=report_review[key]
            assert row['command']==check['command']
            assert row['source_sha256']==check['source_sha256'] and row['binary_sha256']==check['binary_sha256']
            assert row['algorithm']['strict']
            samples=row['samples_seconds'];validations=row['sample_validation']
            assert len(samples)==len(validations)<=3
            assert all(math.isfinite(s) and s>0 for s in samples) and all(v['reference_match'] for v in validations)
            if samples:assert row['median_seconds']==statistics.median(samples)
            if row['language']=='mech':
                bench=next(b for b in before['benchmarks'] if b['key']==row['benchmark'])
                group=next(g for g in bench['groups'] if g['language']=='mech')
                fast=before['programs'][group['fast']];small=before['programs'][group['small']]
                assert row['loc']==fast['loc'] and row['source_sha256'] in fast['files']
                controls[name+':'+row['benchmark']]=dict(key=key,chart_key=fast['key'],
                    seconds=statistics.median(samples) if len(samples)==3 else None,samples=samples,
                    fastest_loc=fast['loc'],smallest_loc=small['loc'],source_sha256=row['source_sha256'],
                    binary_sha256=row['binary_sha256'],preserved_chart_seconds=fast['seconds'],
                    timing_report='results/'+name)
            else:
                assert row['small_validation']['reference_match'] and row['algorithm']['native_benchmark_kernel'] is False
                metric=normalize(Path(row['source']).read_text(),row['language'])
                final_loc[key]=dict(source_sha256=row['source_sha256'],timed_loc=row['loc'],**metric)
                rows[key]=dict(row,loc=metric['normalized_loc'],physical_loc=metric['physical_loc'],
                    loc_expansions=metric['expansions'],timing_report='results/'+name,control_key=name+':'+row['benchmark'])
                if row.get('full_failure'):failures.append(dict(key=key,language=row['language'],benchmark=row['benchmark'],**row['full_failure']))
                if not args.partial:
                    metric=normalize(Path(row['source']).read_text(),row['language'])
                    assert metric['physical_loc']==row['physical_loc']
    if not args.partial:
        assert 'native-timings-v2.json' in reports
        pins={}
        for report_audit in audits.values():
            for filename,digest in (report_audit['pin_files']|report_audit['libraries']).items():
                assert filename not in pins or pins[filename]==digest
                pins[filename]=digest
        for filename,digest in pins.items():assert sha(filename)==digest,filename
        for runtime in runtimes.values():assert sha(runtime['path'])==runtime['sha256']
        for row in review.values():
            for f in ('source','binary'):assert sha(row[f])==row[f+'_sha256'],row['key']
        assert set(rows)=={r['key'] for check in audits.values() for r in check['programs'] if r['language'] in LABELS}
    resolved=set()
    for repair in repairs:
        old=rows[repair['failed_key']];new=rows[repair['replacement_key']]
        assert old.get('full_failure') and old['benchmark']==new['benchmark'] and old['language']==new['language']
        if len(new['samples_seconds'])==3 and not new.get('full_failure'):resolved.add(old['key'])
    unresolved=[f for f in failures if f['key'] not in resolved]
    comparisons=[]
    for language in LABELS:
        for benchmark in BENCHMARKS:
            candidates=[r for r in rows.values() if r['language']==language and r['benchmark']==benchmark]
            incomplete=[r['key'] for r in candidates if len(r['samples_seconds'])<3 and not r.get('full_failure')]
            done=[r for r in candidates if len(r['samples_seconds'])==3 and not r.get('full_failure')]
            if not done or incomplete or any(controls[r['control_key']]['seconds'] is None for r in done):
                pending.append(dict(language=language,benchmark=benchmark,keys=incomplete));continue
            fastest=min(done,key=lambda r:(r['median_seconds'],r['loc']))
            smallest=min(done,key=lambda r:(r['loc'],r['median_seconds']))
            def endpoint(r,kind):
                control=controls[r['control_key']]
                reference=control['fastest_loc' if kind=='fastest' else 'smallest_loc']
                return dict(key=r['key'],seconds=r['median_seconds'],samples_seconds=r['samples_seconds'],
                    sample_stdev_seconds=statistics.stdev(r['samples_seconds']),time_x_mech=r['median_seconds']/control['seconds'],
                    loc=r['loc'],physical_loc=r['physical_loc'],loc_x_mech=r['loc']/reference,
                    mech_seconds=control['seconds'],mech_loc=reference,source_sha256=r['source_sha256'],
                    binary_sha256=r['binary_sha256'],timing_report=r['timing_report'],control_key=r['control_key'])
            comparisons.append(dict(language=language,benchmark=benchmark,candidates=len(done),
                fastest=endpoint(fastest,'fastest'),smallest=endpoint(smallest,'smallest'),same_source=fastest['key']==smallest['key']))
    if not args.partial:
        assert not unresolved and not pending and len(comparisons)==len(LABELS)*len(BENCHMARKS), (unresolved,pending)
    summary=dict(status='partial' if args.partial else 'passed',comparisons=comparisons,pending=pending,
        controls=controls,final_loc=final_loc,full_failures=failures,unresolved_full_failures=unresolved,repairs=repairs,
        admitted_sources=sum(len(r['samples_seconds'])==3 and not r.get('full_failure') for r in rows.values()),
        sample_count=3,completed_samples=sum(len(r['samples_seconds']) for report in reports.values() for r in report['programs'].values()),
        expected_samples=sum(len(r['samples_seconds']) if r.get('full_failure') else 3
            for report in reports.values() for r in report['programs'].values()),
        planned_samples=3*sum(len(report['programs']) for report in reports.values()),
        source_audit_sha256=sha(audit_path),source_audits={name:sha(D/name) for name in audits},
        report_pins={'results/'+name:sha(D/'results'/name) for name in reports},
        summary_script_sha256=sha(Path(__file__)),loc_tool_sha256=sha(D/'metrics_final.py'),
        loc_corrections=[dict(key=key,previous=r['timed_loc'],corrected=r['normalized_loc'],
            reason='Count a loop header once when it contains nested calls/conditions.') for key,r in final_loc.items() if r['timed_loc']!=r['normalized_loc']],
        supplemental_audit_sha256=sha(D/'evidence/supplemental-audit.json'),
        preserved_explorer_sha256=sha(before_path),runtimes=runtimes,
        ratio_policy='Time × Mech uses the fresh unchanged fastest Mech AOT control from the same cohort for both profiles. '
        'Fastest LOC × Mech compares fastest sources; smallest LOC × Mech compares smallest sources. Below 1× means faster or shorter than Mech.',
        endpoint_policy='Fastest and smallest statement-normalized LOC selected only from faithful candidates with three complete full-workload samples. '
        'These are measured winners from the pinned candidate pool, not a claim of an absolute language speed limit.',
        discarded_samples=sum(len(r['discarded']) for r in reports.values()))
    name='progress' if args.partial else 'summary';(D/f'results/{name}.json').write_text(json.dumps(summary,indent=2)+'\n')
    fields=['language','benchmark','profile','seconds','time_x_mech','loc','loc_x_mech','mech_seconds','mech_loc','key']
    with (D/f'results/{name}.csv').open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader()
        for c in comparisons:
            for profile in ('fastest','smallest'):
                r=c[profile];writer.writerow(dict(language=c['language'],benchmark=c['benchmark'],profile=profile,**{k:r[k] for k in fields if k in r}))
    lines=['# Remaining Benchmark Game languages compared with Mech','',summary['ratio_policy'],'',
        'Full prescribed workloads; medians of three fresh processes. All times are seconds. Complete application helpers/FFI count toward LOC; general library implementations are excluded.','']
    for language,label in LABELS.items():
        lines += ['## '+label,'','| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |','|---|---|---:|---:|---:|---:|']
        for c in comparisons:
            if c['language']!=language:continue
            for profile in ('fastest','smallest'):
                r=c[profile];lines.append(f"| {c['benchmark']} | {profile} | {r['seconds']:.6f} | {r['time_x_mech']:.3f}× | {r['loc']} | {r['loc_x_mech']:.3f}× |")
        lines.append('')
    if pending:lines+=['Groups awaiting complete samples are omitted; no estimated or scaled timings are shown.','']
    if failures:
        lines+=['## Excluded full-workload failures','']
        lines += [f"- {r['key']}: {r['error']}" for r in failures]
    if repairs:
        lines+=['','## Separately measured corrections','']
        lines += [f"- {r['replacement_key']}: {r['changes']} Three fresh full-size samples and a fresh unchanged Mech control; the failed original is retained above." for r in repairs]
    (D/f'results/{name}.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps(dict(status=summary['status'],comparisons=len(comparisons),completed_samples=summary['completed_samples'],
        expected_samples=summary['expected_samples'],pending_groups=len(pending),full_failures=len(failures))))

if __name__=='__main__':main()
