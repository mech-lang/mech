"""Record a complete Git checkpoint inventory under the benchmark timing lock."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'benchmarks/clbg/continuation/jobs/git-persistence-inspection-v2'
LOCK = Path('/tmp/mech-clbg-timing.lock')


def main():
    print('WAIT common mutex: durable Git persistence inventory', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        OUT.mkdir(parents=True)
        records = []
        commands = {
            'tracked-status': ['git', 'status', '--porcelain=v1', '--untracked-files=no'],
            'tracked-diff-stat': ['git', 'diff', '--stat'],
            'untracked-paths': ['git', 'ls-files', '--others', '--exclude-standard'],
            'tracked-paths': ['git', 'ls-files'],
            'remotes': ['git', 'remote'],
            'head': ['git', 'rev-parse', 'HEAD'],
            'branch': ['git', 'branch', '--show-current'],
        }
        for name, command in commands.items():
            result = subprocess.run(command, cwd=ROOT, capture_output=True)
            for suffix, data in [('txt', result.stdout), ('stderr', result.stderr)]:
                with (OUT / (name + '.' + suffix)).open('xb') as stream:
                    stream.write(data)
            records.append(dict(command=command, exit_code=result.returncode))
            assert result.returncode == 0, (name, result.returncode)
        assert (OUT / 'branch.txt').read_text().strip() == 'codex/clbg-native-benchmarks'
        files = []
        for line in (OUT / 'untracked-paths.txt').read_text().splitlines():
            p = ROOT / line
            files.append(dict(path=line, bytes=p.stat().st_size if p.is_file() else None,
                              symlink=p.is_symlink()))
        with (OUT / 'untracked-sizes.json').open('x') as stream:
            json.dump(files, stream, indent=2)
            stream.write('\n')
        pins = {}
        for p in OUT.iterdir():
            if p.is_file():
                with p.open('rb') as stream:
                    pins[str(p)] = hashlib.file_digest(stream, 'sha256').hexdigest()
        receipt = dict(status='git-inventory-completed-awaiting-curated-staging',
                       recorded_at=datetime.now(timezone.utc).isoformat(),
                       head=(OUT / 'head.txt').read_text().strip(),
                       branch=(OUT / 'branch.txt').read_text().strip(),
                       tracked_changes=len((OUT / 'tracked-status.txt').read_text().splitlines()),
                       untracked_paths=len(files), commands=records, pins=pins,
                       common_timing_mutex_held=True, staged_files=0, commits_created=0,
                       pushes_attempted=0, benchmark_runs=0)
        with (OUT / 'receipt.json').open('x') as stream:
            json.dump(receipt, stream, indent=2)
            stream.write('\n')
        print(json.dumps({k: receipt[k] for k in ['status', 'head', 'tracked_changes', 'untracked_paths']}), flush=True)


if __name__ == '__main__':
    main()
