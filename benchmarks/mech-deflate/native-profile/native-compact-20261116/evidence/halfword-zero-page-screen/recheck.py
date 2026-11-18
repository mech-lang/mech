"""Screen the halfword mapping policy on the fixed reference encoder."""
from importlib import import_module
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'checked-root-census'))
link = import_module('recheck-link')

OLD = HERE / 'worker-checks/reference'
OUT = HERE / 'bound-reference'
ROLES = ('control', 'halfword-mapped')


def main():
    workers = OUT / 'worker-checks'
    runtime = HERE / 'source/libmech_gpu_runtime.rlib'
    receipts = {}
    for role, library in zip(ROLES, (None, runtime), strict=True):
        destination = workers / role
        receipts[role] = link.link_worker(OLD, OLD, destination, library)
        link.check_worker(destination, f'halfword-mapping-bound-{role}')
    original, changed = (receipts[role] for role in ROLES)
    assert original['worker_sha256'] == link.read(OLD / 'runtime.json')['worker_sha256']
    assert original['worker_sha256'] != changed['worker_sha256']
    for field in ('object_sha256', 'metadata_sha256'):
        assert original[field] == changed[field]
    link.write(OUT / 'link-bindings.json', receipts)
    return link.check_contracts(OUT, ROLES)


if __name__ == '__main__':
    raise SystemExit(main())
