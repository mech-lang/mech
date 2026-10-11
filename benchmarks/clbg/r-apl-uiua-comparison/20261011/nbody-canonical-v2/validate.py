"""Mutexed small nbody checks against independent canonical pairwise reference."""
import fcntl
import hashlib
import json
import math
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
G = HERE.parent
sys.path.insert(0, str(G))
from prepare import LOCK, REQUIRED, compact, sha, save, support_pins
from run import PinGuard, kill_group, group_members


def reference(n):
    solar = 4 * math.pi ** 2
    masses = [solar * x for x in [1, .000954791938424326609, .000285885980666130812,
                                  .0000436624404335156298, .0000515138902046611451]]
    p = [[0., 0., 0.], [4.84143144246472090, -1.16032004402742839, -.103622044471123109],
         [8.34336671824457987, 4.12479856412430479, -.403523417114321381],
         [12.8943695621391310, -15.1111514016986312, -.223307578892655734],
         [15.3796971148509165, -25.9193146099879641, .179258772950371181]]
    v = [[0., 0., 0.], [.00166007664274403694, .00769901118419740425, -.0000690460016972063023],
         [-.00276742510726862411, .00499852801234917238, .0000230417297573763929],
         [.00296460137564761618, .00237847173959480950, -.0000296589568540237556],
         [.00268067772490389322, .00162824170038242295, -.0000951592254519715870]]
    v = [[x * 365.24 for x in row] for row in v]
    v[0] = [-sum(masses[i] * v[i][j] for i in range(5)) / solar for j in range(3)]
    pairs = [(i, j) for i in range(5) for j in range(i + 1, 5)]
    def energy():
        kinetic = sum(.5 * masses[i] * sum(x * x for x in v[i]) for i in range(5))
        potential = sum(masses[i] * masses[j] / math.sqrt(sum(
            (p[i][k] - p[j][k]) ** 2 for k in range(3))) for i, j in pairs)
        return kinetic - potential
    initial = energy()
    for step in range(n):
        for i, j in pairs:
            d = [p[i][k] - p[j][k] for k in range(3)]
            q = sum(x * x for x in d)
            magnitude = .01 / (q * math.sqrt(q))
            for k in range(3):
                v[i][k] -= d[k] * masses[j] * magnitude
                v[j][k] += d[k] * masses[i] * magnitude
        p = [[p[i][k] + .01 * v[i][k] for k in range(3)] for i in range(5)]
    return [initial, energy()]


def main():
    results = []
    manifests = {language: json.loads((HERE / (language + '-manifest.json')).read_text())
                 for language in ['apl', 'uiua']}
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        compact.wait_for_environment(REQUIRED)
        for language, manifest in manifests.items():
            pins = support_pins(manifest)
            pins[manifest['runtime']['path']] = manifest['runtime']['sha256']
            for row in manifest['programs']:
                pins[row['source']] = row['source_sha256']
            guard = PinGuard(pins)
            guard.verify()
            for row in manifest['programs']:
                for n in [0, 1, 2, 10, 100, 1000]:
                    command = [x.replace('{n}', str(n)) for x in row['command']]
                    process = subprocess.Popen(command, cwd=row['cwd'], stdin=subprocess.DEVNULL,
                                               stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                               env=compact.ENV | row['runtime_environment'], start_new_session=True)
                    try:
                        stdout, stderr = process.communicate(timeout=30)
                    except BaseException:
                        kill_group(process.pid)
                        process.wait()
                        raise
                    assert process.returncode == 0, (row['key'], n, stderr)
                    assert not group_members(process.pid), 'Leftover validation process group'
                    expected = reference(n)
                    actual = [float(x) for x in stdout.decode().splitlines()]
                    assert len(actual) == 2 and all(math.isfinite(x) for x in actual)
                    assert max(abs(a - b) for a, b in zip(actual, expected)) <= 1e-8, (row['key'], n, actual, expected)
                    record = dict(key=row['key'], source=row['source'], source_sha256=row['source_sha256'],
                                  n=n, command=command, returncode=process.returncode,
                                  bytes=len(stdout), sha256=hashlib.sha256(stdout).hexdigest(),
                                  energies=actual, reference_energies=expected, tolerance=1e-8,
                                  reference_match=True, stderr=stderr.decode(errors='replace'))
                    output_dir = HERE / 'results' / language
                    output_dir.mkdir(parents=True, exist_ok=True)
                    output_path = output_dir / (row['id'] + '-' + str(n) + '.stdout')
                    output_path.write_bytes(stdout)
                    record['stdout_file'] = str(output_path)
                    results.append(record)
                    if n == 1000:
                        row['small_validation'] = record
                    print('PASS', row['key'], n, flush=True)
                guard.verify()
    report_path = HERE / 'results/validation.json'
    save(report_path, {'passed': True, 'cases': results, 'count': len(results),
                      'canonical_jupiter_z': '-0.103622044471123109',
                      'canonical_workload_n': 50000000, 'full_attempts': 0})
    for language, manifest in manifests.items():
        manifest['status'] = 'prepared-validated'
        manifest['validation'] = {'path': str(report_path), 'sha256': sha(report_path),
                                  'checks': 12, 'passed': True}
        save(HERE / (language + '-manifest.json'), manifest)


if __name__ == '__main__':
    main()
