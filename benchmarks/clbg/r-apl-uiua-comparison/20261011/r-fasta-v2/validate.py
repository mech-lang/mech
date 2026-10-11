"""Mutexed canonical output, generic affine/seed/CDF and chunk-tail checks."""
from pathlib import Path
import fcntl
import hashlib
import json
import math
import subprocess
import sys

HERE = Path(__file__).resolve().parent
G = HERE.parent
C = G.parents[1]
sys.path.insert(0, str(G))
from prepare import LOCK, REQUIRED, compact, save, sha, support_pins
from run import PinGuard, kill_group, group_members


def execute(command, environment):
    process = subprocess.Popen(command, cwd=G, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        env=compact.ENV | environment, start_new_session=True)
    try:
        stdout, stderr = process.communicate(timeout=120)
    except BaseException:
        kill_group(process.pid)
        process.wait()
        raise
    assert process.returncode == 0, (command, stderr.decode(errors='replace'))
    assert not group_members(process.pid), 'Leftover validation worker group'
    return stdout, stderr


def main():
    path = HERE / 'manifest.json'
    manifest = json.loads(path.read_text())
    row = manifest['programs'][0]
    source = Path(row['source'])
    pins = support_pins(manifest) | manifest['preserved_pins']
    pins[source.as_posix()] = row['source_sha256']
    guard = PinGuard(pins)
    reference_rows = []
    for name in ['numerical-manifest.json', 'sequence-manifest.json', 'tree-digit-manifest.json']:
        reference_rows.extend(json.loads((C / 'd-comparison/20261011' / name).read_text())['programs'])
    reference = next(item for item in reference_rows if item['benchmark'] == 'fasta' and 'compact' in item['id'])
    reference_binary = Path(reference['command'][0])
    reference_hash = sha(reference_binary)
    results = HERE / 'results'
    results.mkdir(parents=True, exist_ok=True)
    output_cases = []
    generic_cases = []
    sizes = [1, 2, 7, 19, 20, 21, 59, 60, 61, 1000, 20000,
             49151, 49152, 49153, 81919, 81920, 81921, 122879, 122880, 122881]
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        compact.wait_for_environment(REQUIRED)
        guard.verify()
        for n in sizes:
            assert sha(reference_binary) == reference_hash
            expected, reference_stderr = execute([arg.replace('{n}', str(n)) for arg in reference['command']], {})
            command = [arg.replace('{n}', str(n)) for arg in row['command']]
            stdout, stderr = execute(command, row['runtime_environment'])
            assert stdout == expected, ('Canonical output differs', n)
            stdout_path = results / ('fasta-' + str(n) + '.stdout')
            stdout_path.write_bytes(stdout)
            (results / ('fasta-' + str(n) + '.stderr')).write_bytes(stderr)
            record = dict(n=n, source=str(source), source_sha256=sha(source), command=command,
                returncode=0, bytes=len(stdout), sha256=hashlib.sha256(stdout).hexdigest(),
                expected_sha256=hashlib.sha256(expected).hexdigest(), reference_match=True,
                stdout_file=str(stdout_path), reference_binary=str(reference_binary),
                reference_binary_sha256=reference_hash, passed=True)
            output_cases.append(record)
            print('PASS canonical', n, len(stdout), 'bytes', flush=True)
        helper = source.read_text().split('run <- function(n)', 1)[0]
        assert helper.count('coefficients <- function') == 1
        cases = [(139968, 3877, 29573, 42, n) for n in [0, 1, 2, 3, 7, 60, 61, 257, 1024]]
        cases += [(97, 13, 7, 96, 129), (2, 1, 1, 0, 33),
                  (100003, 99991, 981, 738, 513), (65537, 17, 9, 65536, 257)]
        for modulus, multiplier, increment, seed, size in cases:
            probe = results / f'affine-{modulus}-{size}.R'
            probe.write_text(helper + f'c <- coefficients({size},{multiplier},{increment},{modulus})\n'
                + f'seeds <- (c$multipliers*{seed}+c$increments) %% {modulus}\n'
                + 'cat(format(seeds,scientific=FALSE,trim=TRUE),sep="\\n")\n')
            command = [row['runtime']['path'], '--vanilla', str(probe)]
            stdout, stderr = execute(command, row['runtime_environment'])
            observed = [int(value) for value in stdout.decode().split()]
            expected = []
            current = seed
            for _ in range(size):
                current = (multiplier * current + increment) % modulus
                expected.append(current)
            assert observed == expected, (modulus, size, observed[:20], expected[:20])
            generic_cases.append(dict(kind='affine', modulus=modulus, multiplier=multiplier,
                increment=increment, seed=seed, size=size, source=str(probe), source_sha256=sha(probe),
                command=command, sha256=hashlib.sha256(stdout).hexdigest(), reference_match=True))
            print('PASS affine', modulus, size, flush=True)
        # Every possible direct-divided binary64 draw, including all CDF edge neighbours.
        probabilities = [[.27,.12,.12,.27]+[.02]*11,
                         [.3029549426680,.1979883004921,.1975473066391,.3015094502008]]
        for index, probability in enumerate(probabilities):
            probe = results / f'cdf-{index}.R'
            probe.write_text('probabilities <- c(' + ','.join(repr(value) for value in probability) + ')\n'
                + 'cdf <- cumsum(probabilities)\n'
                + 'random <- (0:139967)/139968\n'
                + 'cat(findInterval(random,cdf),sep="\\n")\n')
            command = [row['runtime']['path'], '--vanilla', str(probe)]
            stdout, stderr = execute(command, row['runtime_environment'])
            observed = [int(value) for value in stdout.decode().split()]
            cumulative = []
            total = 0.0
            for value in probability:
                total += value
                cumulative.append(total)
            expected = [sum(bound <= state/139968.0 for bound in cumulative) for state in range(139968)]
            assert observed == expected, ('CDF direct binary64 strict threshold', index)
            generic_cases.append(dict(kind='CDF-all-direct-draws', profile=index, draws=139968,
                source=str(probe), source_sha256=sha(probe), command=command,
                sha256=hashlib.sha256(stdout).hexdigest(), reference_match=True))
            print('PASS CDF', index, 139968, 'draws', flush=True)
        guard.verify()
    validation_path = results / 'validation.json'
    save(validation_path, dict(passed=True, outputs=output_cases, generic=generic_cases,
        checks=len(output_cases)+len(generic_cases), full_performance_runs=0,
        scope='20canonical full-output small/block-boundary cases;13generic exact modular seed arrays;2all-state unscaled binary64 CDF comparisons.'))
    row['small_validation'] = next(case for case in output_cases if case['n'] == 1000)
    row['adversarial_validation'] = dict(path=str(validation_path), sha256=sha(validation_path),
        checks=len(output_cases)+len(generic_cases), reference_match=True)
    row['measurement_status'] = 'prepared-not-fully-timed'
    manifest.update(status='prepared-validated', validation=dict(path=str(validation_path),
        sha256=sha(validation_path), checks=len(output_cases)+len(generic_cases), passed=True))
    save(path, manifest)
    print('MANIFEST', path, sha(path), flush=True)


if __name__ == '__main__':
    main()
