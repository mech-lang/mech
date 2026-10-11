"""Check native APL affine-prefix arrays and canonical FASTA; no full timings."""
from pathlib import Path
import copy
import datetime
import fcntl
import hashlib
import json
import os
import re
import subprocess

HERE = Path(__file__).resolve().parent
G = HERE.parent
C = G.parents[1]
SOURCE = HERE / 'sources/fasta.optimized.apl'
RESULTS = HERE / 'results'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def execute(path, n=None, workers=1):
    command = [str(G / 'run-apl.sh'), '--cc', str(workers), '-f', str(path)]
    if n is not None:
        command += ['--', str(n)]
    result = subprocess.run(command, capture_output=True, cwd=G,
        env=os.environ | {'LC_ALL': 'C'}, timeout=120)
    assert result.returncode == 0, result.stderr.decode()
    assert not re.search(rb'(?:DOMAIN|RANK|LENGTH|INDEX|SYNTAX|VALUE|WS FULL) ERROR', result.stderr), result.stderr.decode()
    return command, result


def main():
    preserved = json.loads((HERE / 'preserved.json').read_text())
    for path, digest in preserved['pins'].items():
        assert sha(path) == digest
    rows = []
    source_text = SOURCE.read_text()
    # All generic affine operations are visible application code and count in LOC.
    assert all('⋄' not in line and '◊' not in line for line in source_text.splitlines())
    assert all(line.split('⍝', 1)[0].count('←') <= 1 for line in source_text.splitlines())
    d_rows = []
    for name in ['numerical-manifest.json', 'sequence-manifest.json', 'tree-digit-manifest.json']:
        d_rows += json.loads((C / 'd-comparison/20261011' / name).read_text())['programs']
    reference = next(row for row in d_rows if row['benchmark'] == 'fasta' and 'compact' in row['id'])
    reference_binary = Path(reference['command'][0])
    reference_hash = sha(reference_binary)
    sizes = [1, 2, 7, 19, 20, 21, 59, 60, 61, 1000, 11999, 12000, 12001, 19999, 20000, 20001, 40003]
    for n in sizes:
        assert sha(reference_binary) == reference_hash
        golden = subprocess.run([arg.replace('{n}', str(n)) for arg in reference['command']],
            capture_output=True, timeout=30, check=True).stdout
        for workers in ([1, 8] if n in [1, 1000, 20001] else [8]):
            command, result = execute(SOURCE, n, workers)
            name = f'fasta-{n}-cc{workers}'
            (RESULTS / (name + '.stdout')).write_bytes(result.stdout)
            (RESULTS / (name + '.stderr')).write_bytes(result.stderr)
            assert result.stdout == golden, (name, result.stdout[:100], golden[:100])
            rows.append(dict(case='canonical-small' if n == 1000 else name, n=n,
                workers=workers, command=command, passed=True, reference_match=True,
                bytes=len(result.stdout), sha256=hashlib.sha256(result.stdout).hexdigest(),
                reference_source_sha256=reference['source_sha256'], reference_binary_sha256=reference_hash))
            print('PASS', name, len(result.stdout), flush=True)
    # Independent scalar recurrence checks arbitrary coefficients, seeds and tails.
    generic = [(139968, 3877, 29573, 42, n) for n in [0, 1, 2, 3, 7, 60, 61, 257, 1024]]
    generic += [(97, 13, 7, 96, 129), (2, 1, 1, 0, 33),
                (100003, 99991, 981, 738, 513), (65537, 17, 9, 65536, 257)]
    prefix = source_text.split('Run ⍎,⊃¯1↑⎕ARG', 1)[0]
    for modulus, multiplier, increment, seed, size in generic:
        path = RESULTS / f'affine-{modulus}-{size}.apl'
        path.write_text(prefix + f"Write ⍕{modulus} Affine {multiplier} {increment} {seed} {size}\n⊣⎕FIO[4]OUT\n)OFF\n")
        command, result = execute(path, workers=8)
        observed = [int(value) for value in result.stdout.decode().split()]
        expected = []
        state = seed
        for _ in range(size):
            state = (multiplier * state + increment) % modulus
            expected.append(state)
        assert observed == expected, (modulus, size, observed[:10], expected[:10])
        (path.with_suffix('.stdout')).write_bytes(result.stdout)
        (path.with_suffix('.stderr')).write_bytes(result.stderr)
        rows.append(dict(case='generic-affine', modulus=modulus, multiplier=multiplier,
            increment=increment, initial_seed=seed, size=size, passed=True,
            command=command, fixture=str(path), fixture_sha256=sha(path),
            states_checked=len(expected), bytes=len(result.stdout), sha256=hashlib.sha256(result.stdout).hexdigest()))
        print('PASS generic', modulus, size, flush=True)
    for path, digest in preserved['pins'].items():
        assert sha(path) == digest
    report_path = RESULTS / 'validation.json'
    report = dict(passed=True, rows=rows, source=str(SOURCE), source_sha256=sha(SOURCE),
        reference_binary=str(reference_binary), reference_binary_sha256=reference_hash,
        reference_source_sha256=reference['source_sha256'], preserved_pins=preserved['pins'],
        all_draws='Every prefix state is materialized; the last state seeds the next chunk/record.',
        floating_cdf='Every state undergoes binary64 division by 139968.0 and unscaled cumulative probability comparisons.',
        scan='General modular affine composition with doubling shifts; no period/output/table cache.',
        planning_only=False, full_measurements=False)
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    old = next(row for row in json.loads((G / 'apl-manifest.json').read_text())['programs']
               if row['benchmark'] == 'fasta' and row['variant'] == 'optimized')
    row = copy.deepcopy(old)
    for name in ['full_failure', 'timing_alias_of', 'median_seconds', 'sample_stdev_seconds', 'full_validation']:
        row.pop(name, None)
    loc = sum(bool(line.strip()) and not line.lstrip().startswith('⍝') and line.strip() != '∇'
              for line in source_text.splitlines())
    row.update(id='fasta-apl-affine-optimized-v2', key='fasta:fasta-apl-affine-optimized-v2',
        source=str(SOURCE), source_sha256=sha(SOURCE), loc=loc, normalized_loc=loc,
        physical_loc=loc, loc_expansions=[],
        command=[str(G / 'run-apl.sh'), '--cc', '8', '-f', str(SOURCE)], cwd=str(G),
        samples_seconds=[], sample_validation=[], attempts=[], eligible_for_chart=False,
        measurement_status='prepared-not-fully-timed', supersedes_keys=[old['key']],
        small_validation=next(item for item in rows if item['case'] == 'canonical-small' and item['workers'] == 8),
        adversarial_validation=dict(passed=True, report=str(report_path), sha256=sha(report_path), total_checks=len(rows)),
        adaptations=['Replace the scalar per-draw interpreter loop with ordinary modular affine-prefix vector operations.',
            'Retain complete bounded output, every RNG draw, binary64 division, cumulative thresholds and record-to-record state.',
            'Count all local affine-function code; no native helper or new runtime module.'],
        algorithm=dict(strict=True, native_benchmark_kernel=False,
            reason='Every random state is materialized by a general associative affine-prefix scan in counted APL; every value is divided in binary64 before comparisons with the original unscaled CDF. No generator-period shortcut or output cache.'))
    manifest = dict(schema_version=1, status='prepared-validated', language='apl', programs=[row],
        created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        validation=dict(path=str(report_path), sha256=sha(report_path), passed=True, total_checks=len(rows)),
        preserved_original=preserved,
        policy='Separate additive candidate. No full timings; do not mutate frozen inputs during measurements.')
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print('READY', loc, 'LOC;', len(rows), 'checks', flush=True)


if __name__ == '__main__':
    RESULTS.mkdir(parents=True, exist_ok=True)
    print('WAIT shared timing lock for APL FASTA validation', flush=True)
    with open('/tmp/mech-clbg-timing.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        main()
