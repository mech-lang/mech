"""Save reviewed C3 closure and the actual formatted23 fixture evidence."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
ROOT = CLBG.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
OUT = HERE / 'jobs/followup-git-checkpoint-v10'
BRANCH = 'refs/heads/codex/clbg-native-benchmarks'
SUFFIXES = {'.py', '.json', '.md', '.log', '.stdout', '.stderr', '.c3'}
JOBS = ('c3-reviewed-loc-seal-v2', 'c3-reviewed-metadata-v1', 'formatted-loc23-fixtures-v1')


def require(value, why):
    if not value:
        raise ValueError(why)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def git(arguments, name, timeout=600):
    result = subprocess.run(['git', *arguments], cwd=ROOT, capture_output=True, timeout=timeout)
    (OUT / (name + '.stdout')).write_bytes(result.stdout)
    (OUT / (name + '.stderr')).write_bytes(result.stderr)
    require(result.returncode == 0, 'Git failed; journal retained: ' + name)
    return result.stdout


def main():
    # Each selected owned job is complete; never archive an active child's logs.
    for name in JOBS:
        job = HERE / 'owned-jobs' / name
        with (job / 'owner.lock').open('a+') as lease:
            fcntl.flock(lease, fcntl.LOCK_EX)
            terminal = json.loads((job / 'terminal.json').read_text())
            require(terminal['status'] == 'command-exited-zero-awaiting-report-review' and
                    type(terminal['exit_code']) is int and terminal['exit_code'] == 0 and
                    terminal['actual_command_started'] is True and terminal['stop_signals'] == [] and
                    terminal['command_identity_observation_missing'] is False,
                    'Selected owned job is not normal: ' + name)
            require(not any(terminal.get(k) for k in ('error', 'error_type', 'exception_cleanup',
                    'error_log_write_failure')), 'Selected owned job has a worker error')
    print('WAIT common mutex: reviewed source/evidence checkpoint v10', flush=True)
    with LOCK.open('a+') as common:
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), LOCK.stat()
        require((held.st_dev, held.st_ino) == (named.st_dev, named.st_ino), 'Common mutex changed')
        OUT.mkdir(parents=True, exist_ok=False)
        require(not git(['diff', '--cached', '--name-only'], 'prior-index').strip(), 'Prior index is not empty')
        require(git(['branch', '--show-current'], 'branch').decode().strip() ==
                'codex/clbg-native-benchmarks', 'Different branch')
        roots = [
            CLBG / 'loc-normalization/20261013/formatted-loc-v1',
            CLBG / 'c3-comparison/20261012/source-counts/reviews',
            CLBG / 'c3-comparison/20261012/source-counts/canonical-loc-overlay-v2',
            CLBG / 'c3-comparison/20261012/source-counts/overlay-seal-outcomes/canonical-loc-overlay-v2',
            HERE / 'publication-c3-metadata-v1',
            *[HERE / 'owned-jobs' / name for name in JOBS],
            *[HERE / 'jobs' / name for name in (
                'c3-reviewed-loc-seal-entry-v1', 'c3-reviewed-loc-seal-actual-review-v1',
                'c3-reviewed-metadata-entry-v1', 'c3-reviewed-metadata-actual-review-v1',
                'formatted-loc23-actual-review-v1')],
        ]
        selected = set()
        for root in roots:
            require(root.is_dir(), 'Missing reviewed evidence root: ' + str(root))
            for path in root.rglob('*'):
                if (not path.is_file() or path.is_symlink() or path.suffix not in SUFFIXES or
                    path.stat().st_size > 8 * 1024**2 or '__pycache__' in path.parts):
                    continue
                if path.suffix in ('.py', '.md', '.c3') and path.stat().st_mode & 0o222:
                    continue  # A future source author may be working alongside this save.
                selected.add(str(path.relative_to(ROOT)))
        for path in [CLBG / 'BENCHMARK_PROGRESS.md', Path(__file__).resolve(),
            *[HERE / name for name in ('enter_c3_reviewed_loc_seal_v1.py',
                'C3_REVIEWED_LOC_SEAL_ENTRY_V1_REVIEW.md', 'enter_c3_reviewed_metadata_v1.py',
                'C3_REVIEWED_METADATA_ENTRY_V1_REVIEW.md')]]:
            require(path.is_file() and not path.is_symlink(), 'Missing exact checkpoint path')
            selected.add(str(path.relative_to(ROOT)))
        secret = re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9]{36,}|sk-proj-[A-Za-z0-9_-]{40,}')
        pins = {}
        for name in sorted(selected):
            raw = (ROOT / name).read_bytes()
            require(not secret.search(raw), 'Potential credential in selected path')
            pins[name] = hashlib.sha256(raw).hexdigest()
        pathspec = OUT / 'selected-paths.nul'
        pathspec.write_bytes(b''.join(name.encode() + b'\0' for name in sorted(selected)))
        (OUT / 'selection.json').write_text(json.dumps(dict(paths=pins, selected_files=len(pins)), indent=2) + '\n')
        git(['add', '-f', '--pathspec-from-file=' + str(pathspec), '--pathspec-file-nul'], 'stage')
        staged = git(['diff', '--cached', '--name-only'], 'staged-paths').decode().splitlines()
        require(set(staged) <= selected and all(p.startswith('benchmarks/clbg/') for p in staged),
                'Staged paths escaped the reviewed selection')
        git(['diff', '--cached', '--stat'], 'staged-stat')
        if staged:
            git(['commit', '-m', 'Save qualified C3 source closure and formatted LOC fixture results'], 'commit')
        head = git(['rev-parse', 'HEAD'], 'head').decode().strip()
        git(['-c', 'http.version=HTTP/1.1', '-c', 'http.postBuffer=524288000',
             'push', 'origin', 'HEAD:' + BRANCH], 'push')
        remote = git(['ls-remote', 'origin', BRANCH], 'remote').decode().split()[0]
        require(remote == head, 'Remote checkpoint differs')
        receipt = dict(status='committed-pushed-remote-sha-verified', commit=head, remote_commit=remote,
            selected_files=len(pins), new_commit_created=bool(staged), common_mutex_held=True,
            mutex_inode=held.st_ino, benchmark_runs=0, finished_at=datetime.now(timezone.utc).isoformat())
        (OUT / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt), flush=True)


if __name__ == '__main__':
    main()
