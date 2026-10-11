"""Untimed allocator instrumentation and generic pool checks for the corrected trees."""
import hashlib,json,os,subprocess
from pathlib import Path
D=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def write(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
def main():
    output=subprocess.check_output([str(D/'probes/pool-test')],text=True).strip()
    assert output=='Passed 34816 distinct allocations across 17 pool clear/reuse rounds'
    apr=Path('/opt/homebrew/opt/apr/lib/libapr-1.0.dylib').resolve()
    pool_report=dict(status='passed',allocations=34816,clear_reuse_rounds=17,
        allocation_sizes_bytes=[8,248],allocation_contents_checked=True,
        binary=str(D/'probes/pool-test'),binary_sha256=sha(D/'probes/pool-test'),
        source_sha256=sha(D/'probes/pool-test.mojo'),library_sha256=sha(D/'stdlib/pool.mojo'),
        external_library_pins={str(apr):sha(apr)})
    write(D/'results/stdlib-pool-tests.json',pool_report)
    rows=[]
    for name in ('binarytrees-compact','binarytrees-optimized'):
        source=D/'sources'/(name+'.mojo');binary=D/'binaries'/(name+'-mojo')
        evidence=D/'results'/(name+'-mojo-apr-machine.txt')
        machine=evidence.read_text()
        assert '_apr_palloc' in machine and '_apr_pool_clear' in machine
        assert 'build(' in machine and 'check(' in machine and 'w1, #0x10' in machine
        cases=[]
        for depth in (6,10):
            iterations=[(d,1<<(depth-d+4)) for d in range(4,depth+1,2)]
            expected_nodes=(1<<(depth+2))-1+(1<<(depth+1))-1+sum(n*((1<<(d+1))-1) for d,n in iterations)
            expected_clears=sum(n for _,n in iterations)
            expected_output=f'stretch tree of depth {depth+1}\t check: {(1<<(depth+2))-1}\n'
            expected_output+=''.join(f'{n}\t trees of depth {d}\t check: {n*((1<<(d+1))-1)}\n' for d,n in iterations)
            expected_output+=f'long lived tree of depth {depth}\t check: {(1<<(depth+1))-1}\n'
            env=os.environ|{'DYLD_INSERT_LIBRARIES':str(D/'probes/allocation-counter.dylib')}
            result=subprocess.run([str(binary),str(depth)],env=env,capture_output=True,text=True,check=True,timeout=30)
            assert result.stdout==expected_output,result.stdout
            counts=json.loads(result.stderr.strip())
            assert counts==dict(allocations=expected_nodes,bytes=16*expected_nodes,non_16_byte_allocations=0,clears=expected_clears),counts
            cases.append(dict(depth=depth,output_match=True,**counts))
        rows.append(dict(source=str(source.relative_to(D)),source_sha256=sha(source),binary_sha256=sha(binary),
            evidence=str(evidence.relative_to(D)),evidence_sha256=sha(evidence),
            review='Machine code allocates 16 bytes before the depth/leaf branch and recurses twice for interiors. Every temporary iteration calls build, check and apr_pool_clear; the worker backedge encloses all three. Stretch pool is destroyed before long-lived creation. No invariant tree is reused.',
            allocation_counter_cases=cases,status='admitted'))
    report=dict(status='passed',rows=rows,
        test_only_instrumentation=dict(source='probes/allocation-counter.c',source_sha256=sha(D/'probes/allocation-counter.c'),binary_sha256=sha(D/'probes/allocation-counter.dylib'),excluded_from_timed_environment=True),
        rejected_candidate='snapshots/rejected-tree-loop/rejection.json',
        rejected_allocation_candidates='snapshots/pre-allocation-audit/archive.json',
        rejected_result_excluded=True,futhark_binarytrees_admitted=False,
        allocation_policy_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html')
    write(D/'results/tree-work-audit.json',report)
    aggregate_path=D/'results/stdlib-tests.json';aggregate=json.loads(aggregate_path.read_text())
    for file,digest in aggregate['libraries'].items():assert sha(D/file)==digest,file
    aggregate['reports']['stdlib-pool-tests.json']=dict(report=pool_report,sha256=sha(D/'results/stdlib-pool-tests.json'))
    aggregate['libraries']['stdlib/pool.mojo']=sha(D/'stdlib/pool.mojo')
    for file in ('test_tree_allocation.py','probes/pool-test.mojo','probes/allocation-counter.c'):
        aggregate['test_source_sha256'][file]=sha(D/file)
    aggregate['external_library_pins']=pool_report['external_library_pins']
    write(aggregate_path,aggregate)
    print('Passed generic APR pool checks and exact per-node allocation/clear counts for both tree sources at depths 6 and 10.')
if __name__=='__main__':main()
