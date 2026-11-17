"""Collect fresh scorer block counts without using the probe in runtime timings."""
from importlib import import_module
from pathlib import Path
import sys

from emit import HERE, Q, SCORER, read, save, sha

rebase = import_module('rebase-layout')
TRAINING = ('prefix-natural-4096', 'prefix-natural-8192', 'reymont-prefix-32769', 'prefix-natural-32769')


def main():
    role, = sys.argv[1:]
    original = Q / 'count-matcher-instructions.py'
    program = original.read_text()
    target = HERE / role / 'bound'
    needle = '    root = q / f"{label}-native/final-candidate"'
    assert program.count(needle) == 1
    program = program.replace(needle, f'    root = Path({str(target)!r})', 1)
    folder = HERE / role / 'instruction-counts'
    needle = '    directory = q / "matcher-dynamic-blocks" / actor'
    assert program.count(needle) == 1
    program = program.replace(needle, f'    directory = Path({str(folder)!r})', 1)
    headers = {'__name__': 'pinned_scorer_instrumentation', '__file__': str(original)}
    exec(compile(program, str(original), 'exec'), headers)
    ref = rebase.parse(role)
    bound = read(target / 'runtime.json')
    assert ref['object_sha256'] == sha(target / 'program.o') == bound['linked_encoder']['sha256']
    headers['count'](role, role, TRAINING, SCORER, ref['pool'])
    counts = read(folder / 'counts.json')
    assert counts['object_sha256'] == ref['object_sha256']
    save(folder / 'plan.json', {'object_sha256': ref['object_sha256'], 'director': str(original),
         'counter_script_sha256': sha(original), 'patched_script': program,
         'worker_sha256': bound['worker_sha256'], 'training_cases': TRAINING, 'symbol': SCORER,
         'counts_sha256': sha(folder / 'counts.json'), 'untimed': True})


if __name__ == '__main__':
    main()
