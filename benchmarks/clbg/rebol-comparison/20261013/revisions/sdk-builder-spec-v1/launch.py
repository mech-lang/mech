"""Reviewed effective cwd + unchanged official Builder exec under inherited lock."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path

LOCK = Path('/tmp/mech-clbg-timing.lock')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


parser = argparse.ArgumentParser()
parser.add_argument('--plan', required=True)
parser.add_argument('--plan-sha256', required=True)
args = parser.parse_args()
plan_path = Path(args.plan).resolve(strict=True)
assert sha(plan_path) == args.plan_sha256
plan = json.loads(plan_path.read_bytes())
assert plan['schema'] == 'rebol-builder-single-owned-exec-v1'
fd = plan['common_fd']
assert type(fd) is int and fd >= 3
held, named = os.fstat(fd), LOCK.stat()
assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
for path, digest in plan['pins'].items():
    assert sha(path) == digest, ('Changed Builder exec input', path)
builder, cwd = Path(plan['builder']).resolve(strict=True), Path(plan['cwd']).resolve(strict=True)
assert str(builder) in plan['pins'] and cwd.is_dir()
if plan['phase'] == 'version':
    assert plan['command'] == [str(builder), '--version'] and not any(cwd.iterdir())
else:
    assert plan['phase'] == 'spec-only'
    nest = cwd / 'make/rebol3.nest'
    assert plan['command'] == [str(builder), str(nest), '--test', '%rebol3-bulk-macos-arm64']
    assert str(nest) in plan['pins']
os.chdir(cwd)
with Path(plan['startup_receipt']).open('x') as stream:
    json.dump(dict(schema=plan['schema'], phase=plan['phase'], pid=os.getpid(),
        actual_cwd=str(Path.cwd()), command=plan['command'], inherited_common_mutex=True,
        common_mutex_device=held.st_dev, common_mutex_inode=held.st_ino,
        official_builder_modified=False, kernel_image_after_exec_observed=False), stream, indent=2)
    stream.write('\n')
os.set_inheritable(fd, True)
os.execv(str(builder), plan['command'])
