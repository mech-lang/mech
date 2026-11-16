from pathlib import Path
import json, subprocess, time, hashlib, random

R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
P = Path('/private/tmp/mech-related-scalar-home-phase-profile-20261107-v1')
INPUT = Path('/private/tmp/mech-related-post-rust-local-20261102/posts.json')
ORACLE = Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json')
read = lambda p: json.loads(p.read_text())
write = lambda p, d: p.write_text(json.dumps(d, indent=2) + '\n')
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

def run(command, receipt):
    command = list(map(str, command)); start = time.monotonic()
    r = subprocess.run(command, capture_output=True, text=True)
    result = dict(command=command, exit=r.returncode, seconds=time.monotonic()-start,
                  stdout=r.stdout, stderr=r.stderr)
    write(receipt, result)
    assert r.returncode == 0, r.stdout+r.stderr
    return result

for label, base in [('before', 'child-entry-facts-integration'), ('after', 'scalar-home-retirement')]:
    q = P/label
    O = Path('/private/tmp/mech-related-phase-attribution-'+label+'-opt-20261107-v1')
    run(['python3', R/'build-host.py', '--compiler-build', O/'build.json', '--source', q/'runner.rs',
         '--out', q/'runner-aot', '--bundle', q/'build', '--linked'], q/'host-build-run.json')
    run(['python3', R/'audit-native-link.py', '--object', q/'build/object.o', '--executable', q/'runner-aot',
         '--output', q/'native-link-audit.json'], q/'audit-run.json')
    expected = read(Path('/private/tmp/mech-related-'+base+'-runners-20261107-v1/reuse-4/native-link-audit.json'))
    assert read(q/'native-link-audit.json')['normalized_text_sha256'] == expected['normalized_text_sha256']
    print(label, 'actual native bytes match the uninstrumented Object', flush=True)

oracle = read(ORACLE)
rng = random.Random(99024191)
schedule = []
for sample in range(7):
    labels = ['before', 'after']; rng.shuffle(labels)
    schedule.append(labels)
write(P/'schedule.json', dict(seed=99024191, rounds=schedule, primary_benchmark_timings=False))
rows = []
for sample, labels in enumerate(schedule):
    for label in labels:
        q = P/label
        output = q/f'output-{sample}.json'
        receipt = q/f'run-{sample}.json'
        result = run([q/'runner-aot', q/'build/source.mec', INPUT, output, '1'], receipt)
        assert read(output) == oracle
        lines = [line.removeprefix('MECH_PHASE_PROFILE ') for line in result['stderr'].splitlines() if line.startswith('MECH_PHASE_PROFILE ')]
        assert len(lines) == 1
        profile = json.loads(lines[0])
        assert profile['post_json_hardware'] is not None
        windows = profile['parallel_run_windows']
        assert windows and all(w['hardware'] is not None for w in windows)
        total = profile['post_json_hardware']
        parallel = {key: sum(w['hardware'][key] for w in windows) for key in
                    ['instructions_retired', 'cycles', 'user_cpu_seconds', 'system_cpu_seconds']}
        outside = {key: total[key]-parallel[key] for key in parallel}
        assert all(v >= 0 for v in outside.values())
        row = dict(label=label, sample=sample, profile=profile, parallel_hardware_sum=parallel,
                   outside_parallel_hardware=outside, exact_standard_rust_output=True,
                   output_sha256=sha(output), executable_sha256=sha(q/'runner-aot'),
                   primary_benchmark_timings=False)
        rows.append(row)
        write(q/f'profile-{sample}.json', row)
        print(label, sample, 'total/parallel/outside instructions', total['instructions_retired'],
              parallel['instructions_retired'], outside['instructions_retired'], flush=True)
write(P/'captures.json', dict(schema='mech.related-post-gen.phase-captures.v1', rows=rows,
     native_objects_unchanged_and_actual_linked_bytes_audited=True, all14_outputs_exact=True,
     timings_excluded=True, schedule=schedule,
     limitations=['Post-JSON counters include the diagnostic observer.',
       'Canonical timers are inclusive wall clocks, may overlap and are not CPU-time shares.',
       'Outside-parallel counters are computed per invocation from nested cumulative counters, not differences of independent medians.']))
