"""Link one unchanged encoder to both medium word-backing policies."""

from difflib import unified_diff
from importlib import import_module
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROADMAP = HERE.parent
BASE = '8fb7fa9683a3034866104db196ca5a72d7d312da'
TEMPLATE = ROADMAP / 'scorer-vector-latch/latch-kept-root-layout/bound'
PAIRED = HERE / 'workers'
SOURCE = HERE / 'runtime'
ROLES = ('control', 'word-pages-32')

sys.path.insert(0, str(ROADMAP / 'checked-root-census'))
core = import_module('recheck-link')
read, write, sha = core.read, core.write, core.sha


def build():
    old_link = read(TEMPLATE / 'runtime.json')
    old_binary = TEMPLATE / 'mech-worker'
    assert sha(old_binary) == old_link['worker_sha256']
    assert sha(TEMPLATE / 'program.o') == old_link['object_sha256']
    (lib,) = [Path(item.removeprefix('mech_gpu=')) for item in old_link['link']
              if item.startswith('mech_gpu=')]
    assert lib.parent.name == 'word-zero-pages'
    runtime_dir = lib.parent / 'compact_runtime'
    shutil.copytree(runtime_dir, SOURCE / 'compact_runtime', dirs_exist_ok=True)
    original = runtime_dir / 'zero_pages.rs'
    candidate = SOURCE / 'compact_runtime/zero_pages.rs'
    past = original.read_text()
    needle = 'let minimum = if layout.align() == 4 { 16 } else { MINIMUM_MAPPING_PAGES };'
    assert past.count(needle) == 1
    new = past.replace(needle, 'let minimum = if layout.align() == 4 { 32 } else { MINIMUM_MAPPING_PAGES };')
    candidate.write_text(new)
    path = 'compact_runtime/zero_pages.rs'
    (HERE / 'word-page-threshold.patch').write_text(''.join(unified_diff(
        past.splitlines(keepends=True), new.splitlines(keepends=True), f'a/{path}', f'b/{path}')))
    src = SOURCE / 'runtime-lib.rs'
    src.write_text((lib.parent / src.name).read_text().replace(str(lib.parent), str(SOURCE)))
    library = SOURCE / lib.name
    command = ['rustc', '+nightly-2026-03-03', '--edition=2024', '--crate-name', 'mech_gpu',
               '--crate-type=rlib', '-C', 'opt-level=3', str(src), '-o', str(library)]
    with (SOURCE / 'compile.log').open('wb') as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    tests = SOURCE / 'runtime-tests'
    test_args = [arg for arg in command[:-1] if arg not in ('--crate-type=rlib',)]
    test_args[3:3] = ['--test']
    test_args.append(str(tests))
    with (SOURCE / 'tests.log').open('wb') as log:
        subprocess.run(test_args, stdout=log, stderr=subprocess.STDOUT, check=True)
        subprocess.run([tests], stdout=log, stderr=subprocess.STDOUT, check=True)
    write(SOURCE / 'result.json', {'source_pin': BASE, 'command': command, 'test_command': test_args,
                                  'runtime_library_sha256': sha(library), 'old_runtime_library_sha256': sha(lib),
                                  'runtime_change_sha256': sha(HERE / 'word-page-threshold.patch'),
                                  'source_sha256': {str(p.relative_to(SOURCE)): sha(p)
                                                    for p in SOURCE.rglob('*.rs')}})
    return library


def main():
    library = build()
    checked = HERE / 'checks'
    for role in ROLES:
        worker = checked / 'worker-checks' / role
        receipt = core.link_worker(TEMPLATE, TEMPLATE, worker, None if role == 'control' else library)
        if role == 'control':
            assert receipt['worker_sha256'] == sha(TEMPLATE / 'mech-worker')
        core.check_worker(worker, f'overlap-margin-{role}')
    assert core.check_contracts(checked, ROLES) == 0, (checked / 'verify-linked.log').read_text()
    fact = read(checked / 'linked-proofs/result.json')
    assert fact['matches_success_observations'] == 162
    assert len(fact['checked_failure_cases']) == 6
    print(fact['peak_stack_bytes'], flush=True)
    for role in ROLES:
        assert fact['peak_stack_bytes'][role] <= 5280


if __name__ == '__main__':
    main()
