"""Resumable sequential D full-workload samples with freshly timed Mech controls."""
import argparse,json,os,random,statistics,subprocess
from datetime import datetime,timezone
from prepare import D,BENCHES,arrays,compact,execute,sha,save

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--benchmarks',default=','.join(BENCHES));parser.add_argument('--samples',type=int,default=3);parser.add_argument('--timeout',type=int,default=14400)
    args=parser.parse_args();assert args.samples>=3
    audit=json.loads((D/'source-audit.json').read_text());assert audit['status']=='reviewed'
    path=D/'results/timings.json'
    rows={r['key']:dict(r,samples_seconds=[],sample_validation=[]) for r in audit['programs']+audit['controls']}
    report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),sample_count=args.samples,programs=rows,orders={},discarded=[],completed_benchmarks=[],source_audit_sha256=sha(D/'source-audit.json'),
        power=compact.power_profile(),machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),os=subprocess.check_output(['sw_vers'],text=True).strip(),protocol=audit['timing_boundary'],workloads=audit['workloads'],runtime=audit['runtime'])
    assert report['sample_count']==args.samples and report['source_audit_sha256']==sha(D/'source-audit.json')
    assert set(report['programs'])==set(rows)
    pins=audit['pins']|audit['libraries']|{audit['runtime']['path']:audit['runtime']['sha256']}
    for row in rows.values():
        for field in ('source','binary'):pins[row[field]]=row[field+'_sha256']
    stats={}
    def verify():
        for file,digest in pins.items():
            st=os.stat(file);state=(st.st_dev,st.st_ino,st.st_size,st.st_mtime_ns,st.st_ctime_ns)
            if stats.get(file)!=state:assert sha(file)==digest,file;stats[file]=state
    verify();save(path,report)
    for bench in args.benchmarks.split(','):
        candidates=[r for r in report['programs'].values() if r['benchmark']==bench];assert candidates
        rng=random.Random(20261011+sum(map(ord,bench)))
        for sample in range(args.samples):
            order=candidates.copy();rng.shuffle(order)
            rounds=report['orders'].setdefault(bench,[])
            if len(rounds)<=sample:rounds.append([r['key'] for r in order])
            else:assert rounds[sample]==[r['key'] for r in order]
            for row in order:
                if len(row['samples_seconds'])>sample or row.get('full_failure'):continue
                while True:
                    compact.wait_for_environment(arrays.REQUIRED);verify()
                    report['active']=dict(key=row['key'],sample=sample+1,started_at=datetime.now(timezone.utc).isoformat());save(path,report)
                    print('RUN',sample+1,row['id'],flush=True)
                    try:result=execute(row,True,args.timeout)
                    except Exception as error:
                        row['full_failure']=dict(error=str(error),at=datetime.now(timezone.utc).isoformat());report.pop('active',None);save(path,report);print('FULL FAILURE',row['id'],str(error)[:1000],flush=True);break
                    if result['competing_compilers'] or result['power_changes']:
                        report['discarded'].append(dict(key=row['key'],sample=sample+1,result=result));save(path,report);continue
                    row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation']);row.setdefault('full_validation',result['validation'])
                    row['median_seconds']=statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds'])>1:row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
                    report.pop('active',None);save(path,report);print('SAMPLE',sample+1,row['id'],f"{result['seconds']:.6f}s",flush=True);break
        if bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
        save(path,report)
    report['status']='complete' if set(report['completed_benchmarks'])==set(BENCHES) else 'partial'
    report['finished_at']=datetime.now(timezone.utc).isoformat();verify();save(path,report);print('REPORT',report['status'],flush=True)

if __name__=='__main__':main()
