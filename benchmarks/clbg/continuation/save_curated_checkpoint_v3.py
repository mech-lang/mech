"""Persist the reviewed benchmark inventory, omitting toolchains and large binaries."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
EXCLUDED = {'snapshot', 'neutral-dependencies', 'python-dependencies', 'python',
            'runtime-dependencies', 'dependencies', 'toolchains', 'node_modules', 'target', '__pycache__'}
SUFFIXES = {'.py', '.rs', '.c', '.h', '.nim', '.cr', '.pony', '.c3', '.gleam', '.erl',
            '.clj', '.pl', '.mech', '.ua', '.apl', '.jl', '.m', '.sh', '.mjs', '.js', '.ts',
            '.json', '.md', '.csv', '.log', '.txt', '.stdout', '.stderr', '.wam', '.ll', '.s',
            '.asm', '.disassembly'}


def command(argv, output, name):
    result = subprocess.run(argv, cwd=ROOT, capture_output=True, timeout=600)
    (output / (name + '.stdout')).write_bytes(result.stdout)
    (output / (name + '.stderr')).write_bytes(result.stderr)
    assert result.returncode == 0, ('Git operation failed; preserve journal', name, result.returncode)
    return result.stdout


with Path('/tmp/mech-clbg-timing.lock').open('a+') as lock:
    print('WAIT common mutex: curated source/evidence Git checkpoint', flush=True)
    fcntl.flock(lock, fcntl.LOCK_EX)
    output = HERE / 'jobs/curated-git-checkpoint-v3'
    output.mkdir(parents=True)
    staged = command(['git', 'diff', '--cached', '--name-only'], output, 'prior-index')
    assert not staged.strip(), 'Existing staged work requires separate review'
    selected = set()
    inventory = HERE / 'jobs/git-persistence-inspection-v2/untracked-sizes.json'
    for item in json.loads(inventory.read_text()):
        path = Path(item['path'])
        if (str(path).startswith('benchmarks/clbg/') and not item['symlink'] and
                item.get('bytes') is not None and item['bytes'] <= 8 * 1024**2 and
                path.suffix in SUFFIXES and
                not any(part.startswith('.build') or part in EXCLUDED for part in path.parts)):
            assert (ROOT / path).is_file(), ('Inventoried artifact disappeared', str(path))
            selected.add(str(path))
    # Include the concrete follow-up sources and journals authored since that
    # completed inventory. A file still being written by a job is not selected.
    for directory in (HERE, HERE.parent / 'prolog-comparison/20261013/drafts',
                      HERE.parent / 'prolog-comparison/20261013/revisions/runtime-v2',
                      HERE.parent / 'nim-comparison/20261012/revisions/full-seal-v6'):
        for path in directory.rglob('*'):
            relative = path.relative_to(ROOT)
            if (path.is_file() and not path.is_symlink() and path.suffix in SUFFIXES and
                    path.stat().st_size <= 8 * 1024**2 and
                    not any(part.startswith('.build') or part in EXCLUDED for part in relative.parts)):
                # Running target logs/ledgers are omitted until their terminal.
                if 'owned-jobs' in relative.parts and not (path.parent / 'terminal.json').exists():
                    continue
                if 'jobs' in relative.parts and output.is_relative_to(path.parent):
                    continue
                selected.add(str(relative))
    tracked = command(['git', 'diff', '--name-only'], output, 'tracked-diff-paths').decode().splitlines()
    assert all(path.startswith('benchmarks/clbg/') for path in tracked)
    selected.update(tracked)
    secret = re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9]{36,}|sk-proj-[A-Za-z0-9_-]{40,}')
    pins = {}
    for name in sorted(selected):
        raw = (ROOT / name).read_bytes()
        assert not secret.search(raw), ('Potential credential in checkpoint input', name)
        pins[name] = hashlib.sha256(raw).hexdigest()
    paths = output / 'selected-paths.nul'
    paths.write_bytes(b''.join(name.encode() + b'\0' for name in sorted(selected)))
    (output / 'selection.json').write_text(json.dumps(dict(paths=pins,
        selected_files=len(selected), excluded='toolchains, vendor snapshots, caches and artifacts over8MiB'), indent=2) + '\n')
    command(['git', 'add', '--pathspec-from-file=' + str(paths), '--pathspec-file-nul'], output, 'stage')
    command(['git', 'diff', '--cached', '--stat'], output, 'staged-stat')
    command(['git', 'commit', '-m', 'Preserve authored benchmark suites and qualification evidence'], output, 'commit')
    head = command(['git', 'rev-parse', 'HEAD'], output, 'head').decode().strip()
    command(['git', 'push', 'origin', 'HEAD:refs/heads/codex/clbg-native-benchmarks'], output, 'push')
    remote = command(['git', 'ls-remote', 'origin', 'refs/heads/codex/clbg-native-benchmarks'], output, 'remote-head').decode().split()[0]
    assert remote == head
    (output / 'receipt.json').write_text(json.dumps(dict(status='committed-pushed-remote-sha-verified',
        commit=head, remote_commit=remote, selected_files=len(selected),
        benchmark_runs=0, shared_timing_mutex_held=True,
        finished_at=datetime.now(timezone.utc).isoformat()), indent=2) + '\n')
    print(json.dumps(dict(status='committed-pushed-remote-sha-verified', commit=head,
        selected_files=len(selected))), flush=True)
