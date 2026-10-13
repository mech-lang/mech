"""Save completed benchmark evidence and the verified GitLab Pages release."""
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
OUT = HERE / 'jobs/followup-git-checkpoint-v11'
JOBS = (
    'c3-current-candidate-v1', 'nim-full90-independent-reconcile-v2',
    'gleam-full90-independent-reconcile-v2', 'clojure-full90-independent-reconcile-v2',
    'formatted-crystal20-plan-prepare-v1', 'formatted-crystal20-measure-v1',
    'pony-two-tree-reviewed-next-stage-v2', 'prolog-private-runtime-reviewed-next-stage-v2',
    'prolog-directory4-fixtures-v1',
)
SUFFIXES = {'.py', '.json', '.md', '.log', '.stdout', '.stderr', '.cr', '.diff'}


def git(args, name):
    result = subprocess.run(['git', *args], cwd=ROOT, capture_output=True, timeout=600)
    (OUT / (name + '.stdout')).write_bytes(result.stdout)
    (OUT / (name + '.stderr')).write_bytes(result.stderr)
    assert result.returncode == 0, 'Git failed; journal retained: ' + name
    return result.stdout.decode().strip()


def main():
    closed = {}
    for name in JOBS:
        job = HERE / 'owned-jobs' / name
        with (job / 'owner.lock').open('a+') as lease:
            fcntl.flock(lease, fcntl.LOCK_EX)
            terminal = json.loads((job / 'terminal.json').read_text())
            assert type(terminal['exit_code']) is int and terminal['exit_code'] in (0, 1)
            assert terminal['actual_command_started'] is True and terminal['stop_signals'] == []
            assert terminal['command_identity_observation_missing'] is False
            # Preserve normal exits and original failures without relabelling either.
            closed[name] = dict(status=terminal['status'], exit_code=terminal['exit_code'])
    print('WAIT common mutex: verified Pages and completed benchmark checkpoint', flush=True)
    with LOCK.open('a+') as common:
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        OUT.mkdir(parents=True, exist_ok=False)
        assert not git(['diff', '--cached', '--name-only'], 'prior-index')
        assert git(['branch', '--show-current'], 'branch') == 'codex/clbg-native-benchmarks'
        publication = json.loads((HERE / 'jobs/gitlab-pages-publication-v1/published.json').read_text())
        assert publication['status'] == 'GitLab-Pages-published-and-all-assets-verified'
        assert publication['verified_files'] == 29 and publication['pipeline_status'] == 'success'
        assert publication['dataset_sha256'] == '4fe402c21360fbbbe9d650c15d1fd4c62422c17cfad8cafd24721fe7f0bd2947'
        directories = [
            *[HERE / 'jobs' / name for name in (
                'c3-current-candidate-actual-review-v1', 'c3-current-candidate-released-digests-v1',
                'c3-chart-session-released-digests-v1', 'c3-source-opening-v1',
                'c3-chart-session-v1', 'gitlab-pages-publication-v1',
                'formatted-crystal-plan-source-capture-v1', 'formatted-crystal20-plan-actual-review-v1',
                'formatted-crystal20-actual-review-v1', 'followup-v2-reviewed-freeze-v1',
                'python-framework-host-linkage-v1', 'pony-work-reviewed-next-stage-v2',
                'prolog-runtime-reviewed-next-stage-v2', 'prolog-directory-reviewed-freeze-v1',
                'prolog-directory4-actual-review-v1', 'prolog-runtime-source-archive-audit-v1',
                'followup-git-checkpoint-v10')],
            HERE / 'formatted-crystal-plan-v1/jobs/actual-plans-v1',
            CLBG / 'loc-normalization/20261013/formatted-loc-v1/jobs/crystal20-format-v1/profiles',
            CLBG / 'prolog-comparison/20261013/revisions/runtime-directory-v1',
        ]
        selected = set()
        def select(path):
            assert path.is_file() and not path.is_symlink(), str(path)
            assert path.is_relative_to(ROOT)
            selected.add(str(path.relative_to(ROOT)))
        for directory in directories:
            assert directory.is_dir(), 'Missing completed evidence: ' + str(directory)
            for path in directory.rglob('*'):
                if not path.is_file() or path.is_symlink() or '__pycache__' in path.parts:
                    continue
                if path.suffix not in SUFFIXES or path.stat().st_size > 8 * 1024**2:
                    continue
                if path.suffix in ('.py', '.md', '.cr') and path.stat().st_mode & 0o222:
                    continue
                select(path)
        for name in JOBS:
            for leaf in ('request.json', 'launch.json', 'worker-started.json', 'command-started.json',
                         'terminal.json', 'command.stdout', 'command.stderr', 'worker.log'):
                path = HERE / 'owned-jobs' / name / leaf
                if path.is_file() and path.stat().st_size <= 8 * 1024**2:
                    select(path)
        exact = [
            CLBG / 'BENCHMARK_PROGRESS.md', Path(__file__).resolve(),
            CLBG / 'overview/explorer/dist/data.json', CLBG / 'overview/explorer/dist/index.html',
            *[HERE / name for name in ('qualify_c3_current_candidate_v1.py',
                'c3-ready-publication-v1.json', 'C3_CURRENT_CANDIDATE_V1_REVIEW.md',
                'c3_chart_session_v1.py', 'C3_CHART_SESSION_V1_REVIEW.md',
                'enter_reviewed_followup_v2.py', 'ENTER_REVIEWED_FOLLOWUP_V2_REVIEW.md')],
            *[HERE / 'jobs/c3-current-candidate-v1' / name for name in
                ('reused-qualification.json', 'outcome.json', 'candidate-data.json')],
            *[CLBG / folder / 'reconciliation' / name for folder, name in (
                ('nim-comparison/20261012', 'full-v6-suite-v2.json'),
                ('gleam-comparison/20261012', 'full-v4-suite-v2.json'),
                ('clojure-comparison/20261012', 'full-v1-suite-v2.json'))],
            HERE / 'jobs/completed-native-review-v1/review.json',
            HERE / 'jobs/completed-native-review-v1/terminal.json',
            *[HERE / 'formatted-crystal-plan-v1' / name for name in
                ('prepare_plan.py', 'physical_crystal.py', 'REVIEW.md')],
            *[CLBG / 'loc-normalization/20261013/formatted-loc-v1/jobs/crystal20-format-v1' / name
                for name in ('qualification.json', 'terminal.json')],
            *[HERE / 'jobs/gitlab-pages-publication-v1' / name for name in
                ('BROWSER_QA.md', 'prepared.json', 'website-existing-files.diff',
                 'website-commit.json', 'website-push.json', 'published.json')],
        ]
        for path in exact:
            select(path)
        secret = re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9]{36,}|sk-proj-[A-Za-z0-9_-]{40,}')
        pins = {}
        for name in sorted(selected):
            raw = (ROOT / name).read_bytes()
            assert not secret.search(raw), 'Potential credential: ' + name
            pins[name] = hashlib.sha256(raw).hexdigest()
        assert pins['benchmarks/clbg/overview/explorer/dist/data.json'] == publication['dataset_sha256']
        pathspec = OUT / 'selected-paths.nul'
        pathspec.write_bytes(b''.join(name.encode() + b'\0' for name in sorted(selected)))
        (OUT / 'selection.json').write_text(json.dumps(dict(paths=pins, closed_jobs=closed), indent=2) + '\n')
        git(['add', '-f', '--pathspec-from-file=' + str(pathspec), '--pathspec-file-nul'], 'stage')
        staged = git(['diff', '--cached', '--name-only'], 'staged-paths').splitlines()
        assert set(staged) <= selected and all(p.startswith('benchmarks/clbg/') for p in staged)
        git(['diff', '--cached', '--check'], 'check')
        git(['diff', '--cached', '--stat'], 'stat')
        git(['commit', '-m', 'Publish benchmark explorer on GitLab Pages and save qualified cohort evidence'], 'commit')
        head = git(['rev-parse', 'HEAD'], 'head')
        branch = 'refs/heads/codex/clbg-native-benchmarks'
        git(['-c', 'http.version=HTTP/1.1', 'push', 'origin', 'HEAD:' + branch], 'push')
        remote = git(['ls-remote', 'origin', branch], 'remote').split()[0]
        assert remote == head
        receipt = dict(status='committed-pushed-remote-sha-verified', commit=head, remote_commit=remote,
                       selected_files=len(pins), common_mutex_held=True, benchmark_runs=0,
                       website_commit=publication['website_commit'], publication_url=publication['publication_url'],
                       finished_at=datetime.now(timezone.utc).isoformat())
        (OUT / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt), flush=True)


if __name__ == '__main__':
    main()
