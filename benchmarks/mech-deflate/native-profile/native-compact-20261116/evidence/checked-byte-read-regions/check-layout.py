"""Bind the size candidate and verify output, failure origins and stack use."""
from pathlib import Path
import shutil
import subprocess
import sys

from screen import read, sha, write

here = Path(__file__).resolve().parent
q = here.parents[1]
work = here / 'checks'
fixed = q / 'refused-ssa-maintained/worker-checks/refused-ssa'
roles = ('control', 'general-byte-read')


def build_actor(role):
    actor = work / 'worker-checks' / role
    actor.mkdir(parents=True)
    plan = read(fixed / 'runtime.json')
    artifact = (fixed if role == 'control' else here / 'screen' / role / 'compiled')
    for filename in ('program.o', 'metadata.json'):
        shutil.copyfile(artifact / filename, actor / filename)
    link = [argument.replace(str(fixed / 'program.o'), str(actor / 'program.o')) for argument in plan['link']]
    link[link.index('-o') + 1] = str(actor / 'mech-worker')
    with (actor / 'link.log').open('w') as log:
        subprocess.run(link, stdout=log, stderr=subprocess.STDOUT, check=True)
    if role == 'control':
        assert sha(actor / 'mech-worker') == sha(fixed / 'mech-worker')
    assert sha(actor / 'mech-worker') != ''
    plan.update(link=link, worker_sha256=sha(actor / 'mech-worker'), object_sha256=sha(actor / 'program.o'),
                metadata_sha256=sha(actor / 'metadata.json'))
    write(actor / 'runtime.json', plan)
    label = f'root-read-layout-{role}'
    subprocess.run([sys.executable, '-B', q / 'verify-worker.py', actor / 'mech-worker',
                    actor / 'metadata.json', label], check=True)
    for suffix in ('qualification.json', 'smokes.json', 'smokes.stderr'):
        shutil.copyfile(q / f'{label}-{suffix}', actor / suffix)


def main():
    work.mkdir()
    for role in roles:
        build_actor(role)
    driver = (q / 'backend-work-hardened/verify-linked.py').read_text()
    replacements = {
        'q = study.parent': f'q = Path({str(q)!r})',
        'roles = ("retained", "size-control", "owner-exchange")': f'roles = {roles!r}',
        'record = [json.loads((q / f"hardened-backend-{role}-smokes.json").read_text()) for role in roles]':
            'record = [json.loads((actor / "smokes.json").read_text()) for actor in actors]',
        '(q / f"hardened-backend-{role}-smokes.stderr").read_text() for role in roles':
            '(actor / "smokes.stderr").read_text() for actor in actors',
        'assert len(record[0]) == 162 and record[0] == record[1] == record[2]':
            'assert len(record[0]) == 162 and record[0] == record[1]',
        'assert origins[0] == origins[1] == origins[2]': 'assert origins[0] == origins[1]',
    }
    for old, new in replacements.items():
        assert driver.count(old) == 1, old
        driver = driver.replace(old, new)
    helper = work / 'verify-linked.py'
    helper.write_text(driver)
    with (work / 'verify-linked.log').open('w') as log:
        subprocess.run([sys.executable, '-B', helper], stdout=log, stderr=subprocess.STDOUT, check=True)
    print(read(work / 'linked-proofs/result.json')['peak_stack_bytes'], flush=True)


if __name__ == '__main__':
    main()
