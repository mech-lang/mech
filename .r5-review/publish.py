from pathlib import Path
import os
import subprocess
import sys

root = Path(sys.argv[1])
def git(*args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
base = os.environ['BASE']
assert git('rev-parse', 'HEAD') == base
remote = git('ls-remote', 'origin', 'refs/heads/codex/v0.4-r5-memory-planner').split()[0]
assert remote == base, f'R5 moved to {remote}; refusing to publish against a stale base'
git('config', 'user.name', 'Corey Montella')
git('config', 'user.email', 'cmontella@live.com')
git('add', '--all')
changed = git('diff', '--cached', '--name-only').splitlines()
assert changed and not any(path.startswith('.r5-review/') or path.startswith('.github/workflows/') for path in changed)
git('commit', '-m', 'fix(r5): plan live turn footprints scratch allocations and aggregate peaks')
sha = git('rev-parse', 'HEAD')
ref = 'refs/heads/codex/r5-review-correction-' + sha[:12]
git('push', 'origin', f'{sha}:{ref}')
print('R5_TESTED_COMMIT=' + sha)
print('R5_TESTED_BRANCH=' + ref)
print(git('show', '--stat', '--oneline', 'HEAD'))
