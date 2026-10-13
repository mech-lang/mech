import datetime,fcntl,hashlib,json
from pathlib import Path

W=Path('/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech')
X=W/'benchmarks/clbg/overview/explorer/dist/data.json'
S=Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site/data.json')
OUT=W/'benchmarks/clbg/r-apl-uiua-comparison/20261011/evidence'
LOCK=Path('/tmp/mech-clbg-timing.lock')

def digest(data):return hashlib.sha256(data).hexdigest()

print('WAIT LOCK: selected-runtime coverage audit',flush=True)
with LOCK.open('a+') as lock:
    fcntl.flock(lock,fcntl.LOCK_EX)
    now=datetime.datetime.now(datetime.timezone.utc)
    print('LOCKED: selected-runtime coverage audit',now.isoformat(),flush=True)
    original=X.read_bytes();data=json.loads(original)
    site=S.read_bytes()
    assert original==site,'Actual W/S dataset differs; report this before claiming current live baseline'
    workloads=[b['key'] for b in data['benchmarks']]
    selected={}
    for benchmark in data['benchmarks']:
        for group in benchmark['groups']:
            keys=group['programs']
            qualified=[];rejected=[]
            for key in keys:
                assert key in data['programs'],('Missing selected source',key)
                row=data['programs'][key]
                samples=row.get('samples',[])
                valid=bool(len(samples)>=3 and all(type(x) in (float,int) and x>0 for x in samples)
                    and row.get('algorithm',{}).get('strict') is not False
                    and row.get('eligible_for_chart') is not False)
                (qualified if valid else rejected).append(key)
            selected[(group['language'],benchmark['key'])]=dict(qualified_programs=qualified,rejected_programs=rejected)
    rows=[]
    for language in data['languages']:
        key=language['key'];coverage=[];missing=[]
        for benchmark in workloads:
            record=selected.get((key,benchmark),{})
            if record.get('qualified_programs'):coverage.append(benchmark)
            else:
                exclusions=[r for r in data.get('exclusions',[]) if r.get('language')==key and r.get('benchmark')==benchmark]
                missing.append(dict(benchmark=benchmark,selected_record=record,explicit_exclusions=exclusions))
        rows.append(dict(language=key,label=language['label'],covered=len(coverage),required=len(workloads),coverage=coverage,missing=missing))
    stamp=now.strftime('%Y%m%dT%H%M%S%fZ')
    destination=OUT/('coverage-audit-'+stamp+'.json')
    result=dict(status='passed',created_at=now.isoformat(),dataset=str(X),site_dataset=str(S),dataset_sha256=digest(original),site_dataset_sha256=digest(site),
        source_or_report_mutations=False,data_admission=False,git_or_publication=False,common_mutex=str(LOCK),
        workload_count=len(workloads),runtime_count=len(rows),endpoint_program_count=len(data['programs']),
        policy='Actual selected measured source coverage only, requiring at least three positive samples and no explicit required-work/chart exclusion. Complete independent studies not yet imported remain separate evidence.',
        rows=rows,incomplete=[r for r in rows if r['missing']])
    with destination.open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
    print(json.dumps(dict(status='passed',receipt=str(destination),sha256=digest(destination.read_bytes()),dataset_sha256=result['dataset_sha256'],runtimes=len(rows),incomplete=[dict(language=r['language'],covered=r['covered'],missing=[m['benchmark'] for m in r['missing']]) for r in result['incomplete']]),indent=2),flush=True)
