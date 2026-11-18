"""Measure actual placed scorer work, including cold-path padding admission."""

from importlib import import_module
from pathlib import Path

from campaign import CANDIDATE, CONTROL, HERE, Q, driver, study, read, sha, write


def measure(role):
    artifact = HERE / role / 'bound'
    folder = HERE / role / 'placed-counts'
    base = Q / 'count-matcher-instructions.py'
    source = base.read_text()
    substitutions = {
        '    root = q / f"{label}-native/final-candidate"': f'    root = Path({str(artifact)!r})',
        '    directory = q / "matcher-dynamic-blocks" / actor': f'    directory = Path({str(folder)!r})',
        '    assert assembly[end - 4][0] in {"ret", "retab", "udf", "brk", "b"}': '''
    last = end - 4
    targets = {target for target, _ in edges.values()}
    while assembly[last][0] == "nop":
        assert raw[last] == 0xd503201f and last not in targets
        last -= 4
    assert assembly[last][0] in {"ret", "retab", "udf", "brk", "b"}
'''.rstrip()
    }
    for old, new in substitutions.items():
        assert source.count(old) == 1
        source = source.replace(old, new)
    scope = {'__name__': 'cost_length_placed_counter', '__file__': str(base)}
    exec(compile(source, str(base), 'exec'), scope)
    parsed = study.rebase.parse(role)
    bound = read(artifact / 'runtime.json')
    assert parsed['object_sha256'] == sha(artifact / 'program.o') == bound['linked_encoder']['sha256']
    scope['count'](role, role, study.count.TRAINING, driver.SCORER, parsed['pool'])
    proof = read(folder / 'counts.json')
    assert proof['object_sha256'] == bound['object_sha256']
    receipt = {'source_script_sha256': sha(base), 'object_sha256': sha(artifact / 'program.o'),
               'worker_sha256': sha(artifact / 'mech-worker'), 'observations_sha256': sha(folder / 'counts.json'),
               'patched_script': source}
    write(folder / 'plan.json', receipt)
    return proof


def main():
    controls = {name: measure(name) for name in (CONTROL, CANDIDATE)}
    sample = 'prefix-natural-8192'
    report = {'case': sample, 'roles': {}}
    for role, data in controls.items():
        row = data['cases'][sample]
        source = HERE / role / 'placed-counts/counts.json'
        counts = row['counts']
        report['roles'][role] = {'calls': row['calls'], 'counts': counts, 'raw_sha256': sha(source),
                                 'object_sha256': data['object_sha256']}
    before = report['roles'][CONTROL]['counts']['stack_loads']
    after = report['roles'][CANDIDATE]['counts']['stack_loads']
    report['fewer_stack_loads'] = before - after
    write(HERE / 'placed-counts.json', report)
    print(sample, before, after, flush=True)


if __name__ == '__main__':
    main()
