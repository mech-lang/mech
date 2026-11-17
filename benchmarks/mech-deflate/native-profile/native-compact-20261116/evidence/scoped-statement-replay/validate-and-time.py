"""Check limits and exact artifacts, then time standalone parser variants."""
from itertools import permutations
import json
from statistics import median

from statement_candidate import digest, here, pin, profile, run, write
import run as prior_probe


def verify(name, root, filename):
    file = root / filename
    receipt = json.loads((root / 'build.json').read_text())
    assert receipt['binary'] == digest(file), name
    return file


def validate():
    output = here / 'validation'
    binary = verify('validation', output, 'test-syntax')
    for label, flags in [('focused', ['scoped_statement_']), ('suite', ['--skip', 'scoped_statement_'])]:
        log = output / f'{label}.log'
        command = run([binary, '--test-threads=1', '--nocapture', *flags], log)
        outcome = next(line for line in log.read_text().splitlines() if line.startswith('test result:'))
        write(output / f'{label}.json', {'command': command, 'status': outcome})
        print(outcome, flush=True)


def time():
    roles = ('control', 'candidate', 'capture-only')
    paths = (profile / 'plain', here / 'candidate', here / 'capture-only')
    binaries = {role: verify(role, path, 'parser') for role, path in zip(roles, paths, strict=True)}
    assert len(set(map(digest, binaries.values()))) == 3
    output = here / 'measured'
    output.mkdir()
    orders = list(permutations(roles))
    write(output / 'plan.json', {'pin': pin, 'orders': orders, 'warmups': roles,
                                 'hashes': {role: digest(binary) for role, binary in binaries.items()}})
    for role, binary in binaries.items():
        prior_probe.sample(binary, output, f'warm-{role}')
    rows = [{name: prior_probe.sample(binaries[name], output, f'{i:02d}-{name}')
             for name in order} for i, order in enumerate(orders)]
    samples = lambda row, name: row[name]['observable']['parse_seconds']
    medians = {name: median(samples(row, name) for row in rows) for name in roles}
    ratios = {name: median(samples(row, name) / samples(row, 'control') for row in rows) for name in roles[1:]}
    write(output / 'comparison.json', {'rows': rows, 'median_s': medians, 'paired_ratio_to_control': ratios})
    print(medians, ratios, flush=True)


if __name__ == '__main__':
    validate()
    time()
