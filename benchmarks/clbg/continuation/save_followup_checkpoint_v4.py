"""Save completed follow-up evidence and source packets under the timing mutex."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
EXCLUDED = {'snapshot', 'neutral-dependencies', 'python-dependencies', 'python',
    'runtime-dependencies', 'dependencies', 'toolchains', 'node_modules', 'target', '__pycache__'}
SUFFIXES = {'.py', '.rs', '.c', '.h', '.nim', '.cr', '.pony', '.c3', '.gleam', '.erl',
    '.clj', '.pl', '.mech', '.ua', '.apl', '.jl', '.m', '.sh', '.mjs', '.js', '.ts',
    '.json', '.md', '.csv', '.log', '.txt', '.stdout', '.stderr', '.wam', '.ll', '.s', '.asm'}
BRANCH = 'refs/heads/codex/clbg-native-benchmarks'
TRACKED = set()
SOURCE_SUFFIXES = {'.py', '.md', '.c', '.h', '.rs', '.nim', '.cr', '.pony', '.c3',
    '.gleam', '.erl', '.clj', '.pl', '.mech', '.ua', '.apl', '.jl', '.m', '.sh', '.js', '.ts'}


def git(argv, output, name):
    result = subprocess.run(['git', *argv], cwd=ROOT, capture_output=True, timeout=600)
    (output / (name + '.stdout')).write_bytes(result.stdout)
    (output / (name + '.stderr')).write_bytes(result.stderr)
    assert result.returncode == 0, ('Git failed; retain journal', name, result.returncode)
    return result.stdout


def eligible(path, output):
    relative = path.relative_to(ROOT)
    if (not path.is_file() or path.is_symlink() or path.stat().st_size > 8 * 1024**2 or
        (path.suffix not in SUFFIXES and path.name not in {'stdout', 'stderr'}) or
        any(part.startswith('.build') or part in EXCLUDED for part in relative.parts) or
        path.is_relative_to(output)):
        return False
    if (path.suffix in SOURCE_SUFFIXES and str(relative) not in TRACKED and
        path.stat().st_mode & 0o222):
        # Source-only authors may be working while a measurement owns the
        # mutex. A newly authored packet enters the checkpoint after freezing.
        return False
    # Never commit logs of an active durable child or a partial timing study.
    if 'owned-jobs' in relative.parts and not (path.parent / 'terminal.json').exists():
        return False
    if 'studies' in relative.parts:
        for parent in path.parents:
            report = parent / 'report.json'
            if report.exists():
                if json.loads(report.read_text()).get('status') != 'complete': return False
                break
    return True


with LOCK.open('a+') as lock:
    print('WAIT common mutex: completed follow-up Git checkpoint', flush=True)
    fcntl.flock(lock, fcntl.LOCK_EX)
    held, named = lock.fileno(), LOCK.stat()
    import os
    assert (os.fstat(held).st_dev, os.fstat(held).st_ino) == (named.st_dev, named.st_ino)
    output = HERE / 'jobs/followup-git-checkpoint-v4'
    output.mkdir(parents=True)
    assert not git(['diff', '--cached', '--name-only'], output, 'prior-index').strip()
    assert git(['branch', '--show-current'], output, 'branch').decode().strip() == 'codex/clbg-native-benchmarks'
    TRACKED.update(git(['ls-files'], output, 'tracked-paths').decode().splitlines())
    roots = [HERE]
    for language in ('nim', 'pony', 'prolog', 'julia-aot', 'c3', 'crystal', 'gleam', 'clojure'):
        cohort = CLBG / (language + '-comparison') / ('20261013' if language == 'prolog' else '20261012')
        roots += [cohort / name for name in ('revisions', 'proposals', 'proposal-seals',
            'reconciliation', 'source-counts', 'work-seal-outcomes')]
    roots += [CLBG / 'loc-normalization/20261013/prolog-proposal-v1',
        CLBG / 'v-report-only/20261012/reconciliation']
    roots += [CLBG / 'loc-normalization/20261012' / name / 'jobs' for name in
        ('c3-source-replay-v1', 'gleam-proposal-v2', 'gleam-erlang-helpers-v1',
         'nim-source-replay-v1', 'pony-grammar-v2', 'matlab-regexp-grammar-v1')]
    roots += [CLBG / 'c3-comparison/20261012/studies/full-v1',
        CLBG / 'julia-aot-comparison/20261012/jobs/attach-environment-v1']
    selected = {str(path.relative_to(ROOT)) for root in roots if root.exists()
        for path in root.rglob('*') if eligible(path, output)}
    for name in ('reconcile_suite_v3.py', 'reconcile_suite_v4.py'):
        path = CLBG / 'v-report-only/20261012' / name
        assert eligible(path, output)
        selected.add(str(path.relative_to(ROOT)))
    progress = CLBG / 'BENCHMARK_PROGRESS.md'
    assert eligible(progress, output)
    selected.add(str(progress.relative_to(ROOT)))
    modified = git(['diff', '--name-only'], output, 'tracked-changes').decode().splitlines()
    assert all(path.startswith('benchmarks/clbg/') for path in modified)
    selected.update(modified)
    secret = re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9]{36,}|sk-proj-[A-Za-z0-9_-]{40,}')
    pins = {}
    for name in sorted(selected):
        raw = (ROOT / name).read_bytes()
        assert not secret.search(raw), ('Potential credential', name)
        pins[name] = hashlib.sha256(raw).hexdigest()
    paths = output / 'selected-paths.nul'
    paths.write_bytes(b''.join(name.encode() + b'\0' for name in sorted(selected)))
    (output / 'selection.json').write_text(json.dumps(dict(paths=pins, selected_files=len(pins),
        omitted='active timing studies, active durable logs, toolchains, vendor snapshots and files over8MiB'), indent=2)+'\n')
    git(['add', '--pathspec-from-file='+str(paths), '--pathspec-file-nul'], output, 'stage')
    git(['diff', '--cached', '--stat'], output, 'staged-stat')
    git(['commit', '-m', 'Save completed C3 timings and benchmark qualification follow-ups'], output, 'commit')
    head = git(['rev-parse', 'HEAD'], output, 'head').decode().strip()
    push = subprocess.run(['git', '-c', 'http.version=HTTP/1.1', '-c', 'http.postBuffer=524288000',
        'push', 'origin', 'HEAD:'+BRANCH], cwd=ROOT, capture_output=True, timeout=600)
    (output/'push.stdout').write_bytes(push.stdout); (output/'push.stderr').write_bytes(push.stderr)
    # Remote object identity is authoritative even if Git's HTTP completion
    # response is lost. Do not silently retry a push or invent success.
    remote = git(['ls-remote', 'origin', BRANCH], output, 'remote-head').decode().split()[0]
    assert remote == head, ('Remote checkpoint differs', head, remote, push.returncode)
    receipt = dict(status='committed-pushed-remote-sha-verified', commit=head,
        remote_commit=remote, push_exit_code=push.returncode, selected_files=len(pins),
        shared_timing_mutex_held=True, benchmark_runs=0,
        finished_at=datetime.now(timezone.utc).isoformat())
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt), flush=True)
