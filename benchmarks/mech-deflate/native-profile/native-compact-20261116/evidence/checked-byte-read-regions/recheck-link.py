"""Rebuild checked-read workers from the exact encoder object passed to rustc."""
from hashlib import file_digest
import json
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
OLD = HERE / 'checks/worker-checks'
OUT = HERE / 'linked-recheck'
ROLES = ('control', 'general-byte-read')


def read(path):
    return json.loads(path.read_bytes())


def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def write(path, record):
    path.write_text(json.dumps(record, indent=2) + '\n')


def link_worker(template, artifact, output, library=None):
    """Replace the actual linker input and bind its bytes to the resulting worker."""
    output.mkdir(parents=True)
    recipe = read(template / 'runtime.json')
    old_output = str(template / 'mech-worker')
    args = recipe['link'].copy()
    encoder_index, = [i for i, arg in enumerate(args)
                      if arg.startswith('link-arg=') and Path(arg[9:]).name == 'program.o']
    output_index = args.index('-o') + 1
    assert args[output_index] == old_output
    for name in ('program.o', 'metadata.json'):
        shutil.copyfile(artifact / name, output / name)
    obj = output / 'program.o'
    before = sha(obj)
    args[encoder_index] = f'link-arg={obj}'
    worker = output / 'mech-worker'
    args[output_index] = str(worker)
    if library is not None:
        index, = [i for i, arg in enumerate(args) if arg.startswith('mech_gpu=')]
        args[index] = f'mech_gpu={library}'
    with (output / 'link.log').open('wb') as log:
        subprocess.run(args, stdout=log, stderr=subprocess.STDOUT, check=True)
    assert sha(obj) == before
    assert Path(args[encoder_index].split('=', 1)[1]).samefile(obj)
    record = recipe | {
        'link': args,
        'linked_encoder': {'argument_index': encoder_index, 'path': str(obj), 'sha256': before},
        'object_sha256': before,
        'metadata_sha256': sha(output / 'metadata.json'),
        'worker_sha256': sha(worker),
    }
    write(output / 'runtime.json', record)
    return record


def check_worker(output, label):
    with (output / 'verify.log').open('wb') as log:
        subprocess.run([sys.executable, '-B', Q / 'verify-worker.py', output / 'mech-worker',
                        output / 'metadata.json', label], stdout=log, stderr=subprocess.STDOUT, check=True)
    for name in ('qualification.json', 'smokes.json', 'smokes.stderr'):
        shutil.copyfile(Q / f'{label}-{name}', output / name)
    measured = read(output / 'qualification.json')
    assert measured['complete_smokes'] == 162 and measured['verified']
    assert measured['worker_sha256'] == read(output / 'runtime.json')['worker_sha256']


def check_contracts(folder, roles):
    driver = (Q / 'backend-work-hardened/verify-linked.py').read_text()
    replacements = {
        'q = study.parent': f'q = Path({str(Q)!r})',
        'roles = ("retained", "size-control", "owner-exchange")': f'roles = {roles!r}',
        'record = [json.loads((q / f"hardened-backend-{role}-smokes.json").read_text()) for role in roles]':
            'record = [json.loads((actor / "smokes.json").read_text()) for actor in actors]',
        '(q / f"hardened-backend-{role}-smokes.stderr").read_text() for role in roles':
            '(actor / "smokes.stderr").read_text() for actor in actors',
        'assert len(record[0]) == 162 and record[0] == record[1] == record[2]':
            'assert len(record[0]) == 162 and all(entry == record[0] for entry in record)',
        'assert origins[0] == origins[1] == origins[2]': 'assert all(origin == origins[0] for origin in origins)',
        'fail_support = out / "failure-support"': '''for actor in actors:
    receipt = json.loads((actor / "runtime.json").read_text())
    linked = receipt["linked_encoder"]
    assert receipt["link"][linked["argument_index"]] == f'link-arg={actor / "program.o"}'
    assert linked["path"] == str(actor / "program.o")
    assert linked["sha256"] == receipt["object_sha256"]

fail_support = out / "failure-support"''',
    }
    for previous, replacement in replacements.items():
        assert driver.count(previous) == 1, previous
        driver = driver.replace(previous, replacement, 1)
    test = folder / 'verify-linked.py'
    test.write_text(driver)
    with (folder / 'verify-linked.log').open('wb') as log:
        return subprocess.run([sys.executable, '-B', test], stdout=log, stderr=subprocess.STDOUT).returncode


def main():
    workers = OUT / 'worker-checks'
    control, outline = ROLES
    receipts = {role: link_worker(OLD / role, OLD / role, workers / role) for role in ROLES}
    assert receipts[control]['worker_sha256'] == read(OLD / control / 'runtime.json')['worker_sha256']
    assert receipts[control]['object_sha256'] != receipts[outline]['object_sha256']
    assert receipts[control]['worker_sha256'] != receipts[outline]['worker_sha256']
    write(OUT / 'link-bindings.json', receipts)
    for role in ROLES:
        check_worker(workers / role, f'checked-read-bound-{role}')
    return check_contracts(OUT, ROLES)


if __name__ == '__main__':
    raise SystemExit(main())
