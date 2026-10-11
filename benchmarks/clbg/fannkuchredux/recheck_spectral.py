#!/usr/bin/env python3
"""Resolve a short multiworker spectral-norm timing drift using preserved binaries."""
import copy, json, os, random, statistics, tempfile, time
import run

report=json.loads((run.HERE/'results/nbody-spectral-regression.json').read_text())
base=[r for r in report['programs'] if r['id'].startswith('mech-integer-')]
assert len(base)==2 and base[0]['native_object_sha256']==base[1]['native_object_sha256']
batches=[]
for workers in (8,1):
    rows=copy.deepcopy(base)
    for row in rows: row['samples_seconds']=[]
    rng=random.Random(20261007+workers);orders=[]
    for sample in range(9):
        order=rows.copy();rng.shuffle(order);orders.append([r['id'] for r in order])
        for row in order:
            env=os.environ|row['runtime_environment']|{'RAYON_NUM_THREADS':str(workers)}
            with tempfile.TemporaryFile() as output,tempfile.TemporaryFile() as errors:
                start=time.perf_counter_ns()
                run.process.run_checked(row['command'],cwd=run.ROOT,env=env,
                    stdout=output,stderr=errors,timeout=300)
                elapsed=(time.perf_counter_ns()-start)/1e9
                output.seek(0);assert output.read().decode()==row['expected_full']
            row['samples_seconds'].append(elapsed)
            print(workers,sample+1,row['id'],elapsed,flush=True)
    for row in rows: row['median_seconds']=statistics.median(row['samples_seconds'])
    batches.append(dict(workers=workers,samples=9,sample_order=orders,programs=rows))
output=run.HERE/'results/spectral-drift-recheck.json'
output.write_text(json.dumps(dict(all_outputs_passed=True,native_objects_identical=True,
    measurement_batches=batches,protocol='Shuffled sequential fresh processes; output comparison excluded; no concurrent builds/tests.'),indent=2)+'\n')
