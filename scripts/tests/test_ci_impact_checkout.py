"""Exercise Impact's configured shallow/sparse checkout against a real PR merge."""

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ImpactCheckoutTests(unittest.TestCase):
    def test_shallow_sparse_merge_selects_the_same_native_checks_as_full_checkout(self):
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        impact = workflow.split('\n  impact:\n', 1)[1].split('\n  early-native-plan:', 1)[0]
        depth = int(re.search(r'(?m)^          fetch-depth: (\d+)$', impact)[1])
        patterns = impact.split('          sparse-checkout: |\n', 1)[1]
        patterns = [line.strip() for line in patterns.splitlines() if line.startswith('            /')]
        self.assertIn('          sparse-checkout-cone-mode: false', impact)
        self.assertIn('          ref: ${{ github.sha }}', impact)

        with tempfile.TemporaryDirectory(prefix='ci-impact-checkout-') as temporary:
            root = Path(temporary)
            source = root / 'origin'
            source.mkdir()

            def git(directory, *args, input=None):
                return subprocess.check_output(['git', '-C', str(directory), *args],
                                               input=input, text=True, stderr=subprocess.PIPE).strip()

            git(source, 'init', '--quiet', '--initial-branch=main')
            for key, value in [('user.name', 'CI fixture'), ('user.email', 'ci@example.invalid'),
                               ('commit.gpgsign', 'false'), ('uploadpack.allowFilter', 'true')]:
                git(source, 'config', key, value)
            for name in ['scripts/ci-impact.py', 'scripts/ci_owners.py', '.github/ci/owners.toml']:
                destination = source / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / name, destination)
            native = source / 'hosts/gpu/src/backend.rs'
            native.parent.mkdir(parents=True)
            native.write_text('fn before() {}\n')
            (source / 'README.md').write_text('initial\n')
            git(source, 'add', '.')
            git(source, 'commit', '--quiet', '-m', 'base')
            git(source, 'checkout', '--quiet', '-b', 'feature')
            native.write_text('fn after() {}\n')
            git(source, 'commit', '--quiet', '-am', 'native change')
            git(source, 'checkout', '--quiet', 'main')
            (source / 'README.md').write_text('independent base change\n')
            git(source, 'commit', '--quiet', '-am', 'advanced base')
            base = git(source, 'rev-parse', 'HEAD')
            git(source, 'merge', '--quiet', '--no-ff', 'feature', '-m', 'PR integration merge')
            head = git(source, 'rev-parse', 'HEAD')

            def classify(directory):
                env = {key: value for key, value in os.environ.items() if not key.startswith('GITHUB_')}
                return json.loads(subprocess.check_output(
                    [sys.executable, 'scripts/ci-impact.py', '--base', base, '--head', head],
                    cwd=directory, env=env, text=True))

            expected = classify(source)
            checkout = root / 'checkout'
            subprocess.run(['git', 'clone', '--quiet', '--single-branch', '--no-checkout',
                            '--depth', str(depth), '--filter=blob:none', source.as_uri(), str(checkout)],
                           check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            git(checkout, 'sparse-checkout', 'set', '--no-cone', '--stdin', input='\n'.join(patterns)+'\n')
            git(checkout, 'checkout', '--quiet')
            self.assertEqual(git(checkout, 'rev-parse', 'HEAD'), head)
            self.assertEqual(git(checkout, 'rev-parse', 'HEAD^1'), base)
            self.assertEqual(git(checkout, 'rev-parse', '--is-shallow-repository'), 'true')
            self.assertEqual(git(checkout, 'rev-list', '--count', 'HEAD'), '3')
            self.assertFalse((checkout / 'hosts/gpu/src/backend.rs').exists())
            observed = classify(checkout)
            self.assertEqual(observed, expected)
            self.assertEqual(observed['paths'], ['hosts/gpu/src/backend.rs'])
            self.assertTrue(observed['native_validation_required'])


if __name__ == '__main__':
    unittest.main()
