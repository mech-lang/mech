"""Six metadata-only archive repair cases; no compiler or benchmark."""
import ast
import fcntl
import hashlib
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest

import guarded_prepare_v6 as preparation
import proof_reuse_v6 as reuse

FD = None


def write_archive(path, entries):
    with tarfile.open(path, 'w') as archive:
        for name, kind, value in entries:
            member = tarfile.TarInfo(name)
            if kind == 'file':
                member.size = len(value)
                archive.addfile(member, io.BytesIO(value))
            else:
                member.type = {'directory': tarfile.DIRTYPE, 'symlink': tarfile.SYMTYPE,
                               'hardlink': tarfile.LNKTYPE}[kind]
                if kind != 'directory':
                    member.linkname = value
                archive.addfile(member)


class ArchiveChecks(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(FD, 'Use the common-mutex standalone entry point')
        self.temporary = tempfile.TemporaryDirectory(prefix='crystal-archive-v4-mock-')
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name).resolve()
        self.root = self.base / 'release'
        (self.root / 'package/lib_c').mkdir(parents=True)
        (self.root / 'package/compiler').write_bytes(b'mocked compiler payload')
        self.entries = [('package', 'directory', None), ('package/lib_c', 'directory', None),
                        ('package/compiler', 'file', b'mocked compiler payload')]
        self.archive = self.base / 'release.tar'

    def check(self, extras=()):
        write_archive(self.archive, self.entries + list(extras))
        return reuse.archive_correspondence(preparation, self.archive, self.root)

    def test_regular_payload_and_exact_foreign_target_cycles_pass(self):
        links = [('package/lib_c/self', 'symlink', 'self'),
                 ('package/lib_c/a', 'symlink', 'b'),
                 ('package/lib_c/b', 'symlink', 'a'),
                 ('package/tool', 'symlink', 'compiler'),
                 ('package/lib_c/linux', 'symlink', 'package./../linux')]
        for name, _, target in links:
            (self.root / name).symlink_to(os.path.normpath(target))
        pins = self.check(links)
        self.assertEqual(pins, {str(self.root / 'package/compiler'):
            hashlib.sha256(b'mocked compiler payload').hexdigest()})

    def test_changed_link_metadata_fails_even_with_same_file_target(self):
        target = self.root / 'package/tool'
        for changed in ('./compiler', 'lib_c/../compiler'):
            with self.subTest(changed=changed):
                target.symlink_to(changed)
                with self.assertRaises(AssertionError):
                    self.check([('package/tool', 'symlink', 'compiler')])
                target.unlink()

    def test_absolute_and_parent_traversal_member_names_fail(self):
        for name in ('/outside', '../outside', 'package/../compiler'):
            with self.subTest(name=name), self.assertRaises(AssertionError):
                self.check([(name, 'file', b'no extracted payload')])

    def test_absolute_and_relative_escaping_symlink_targets_fail(self):
        target = self.root / 'package/lib_c/escape'
        for link in (str(self.base / 'outside'), '../../../outside'):
            with self.subTest(link=link):
                target.symlink_to(link)
                with self.assertRaises(AssertionError):
                    self.check([('package/lib_c/escape', 'symlink', link)])
                target.unlink()

    def test_changed_bytes_and_substituted_regular_or_parent_symlinks_fail(self):
        compiler = self.root / 'package/compiler'
        compiler.write_bytes(b'changed payload')
        with self.assertRaises(AssertionError):
            self.check()
        compiler.unlink()
        replacement = self.base / 'outside-compiler'
        replacement.write_bytes(b'mocked compiler payload')
        compiler.symlink_to(replacement)
        with self.assertRaises(AssertionError):
            self.check()
        compiler.unlink()
        compiler.write_bytes(b'mocked compiler payload')
        (self.root / 'package/lib_c').rmdir()
        elsewhere = self.base / 'outside-directory'
        elsewhere.mkdir()
        (elsewhere / 'nested').write_bytes(b'nested payload')
        (self.root / 'package/lib_c').symlink_to(elsewhere, target_is_directory=True)
        with self.assertRaises(AssertionError):
            self.check([('package/lib_c/nested', 'file', b'nested payload')])

    def test_hardlink_topology_and_unchanged_proved_functions(self):
        original = self.root / 'package/compiler'
        alias = self.root / 'package/compiler-copy'
        os.link(original, alias)
        pins = self.check([('package/compiler-copy', 'hardlink', 'package/compiler')])
        self.assertEqual(pins[str(alias)], pins[str(original)])
        extra = self.root / 'package/unlisted.cr'
        extra.write_bytes(b'not in the official archive')
        with self.assertRaises(AssertionError):
            self.check([('package/compiler-copy', 'hardlink', 'package/compiler')])
        extra.unlink()
        alias.unlink()
        alias.write_bytes(original.read_bytes())
        with self.assertRaises(AssertionError):
            self.check([('package/compiler-copy', 'hardlink', 'package/compiler')])
        for link in ('../outside', '/outside'):
            with self.subTest(link=link), self.assertRaises(AssertionError):
                self.check([('package/compiler-copy', 'hardlink', link)])
        def functions(path):
            return {node.name: ast.dump(node, include_attributes=False)
                    for node in ast.parse(path.read_text()).body if isinstance(node, ast.FunctionDef)}
        old = functions(preparation.HERE / 'proof_reuse_v3.py')
        new = functions(preparation.HERE / 'proof_reuse_v6.py')
        self.assertEqual(set(old), set(new))
        self.assertEqual({key: value for key, value in old.items() if key != 'archive_correspondence'},
                         {key: value for key, value in new.items() if key != 'archive_correspondence'})
        old = functions(preparation.HERE / 'guarded_prepare_v3.py')
        new = functions(preparation.HERE / 'guarded_prepare_v6.py')
        self.assertEqual(set(old), set(new))
        changed = {'metadata_paths', 'seal', 'run'}
        self.assertEqual({key: value for key, value in old.items() if key not in changed},
                         {key: value for key, value in new.items() if key not in changed})


def preserve_metadata_failure():
    """Verify the unchanged complete normalization diagnostic and old failures."""
    diagnostic = preparation.HERE / 'metadata-failures/archive-v5-seal-002/link-diagnostic.json'
    evidence = json.loads(diagnostic.read_text())
    assert evidence['status'] == 'diagnostic-only-no-source-or-runtime-change'
    assert evidence['symlinks'] == 19 and len(evidence['mismatches']) == 8
    assert all(row['matches_normpath'] is True for row in evidence['mismatches'])
    for path, digest in evidence['pins'].items():
        assert preparation.sha(path) == digest, ('Recorded extraction diagnostic changed', path)
    fixture = json.loads((preparation.HERE / 'archive-v5-fixtures.json').read_text())
    assert fixture['status'] == 'passed' and fixture['tests_run'] == 6
    assert fixture['failures'] == fixture['errors'] == 0
    for path, digest in fixture['pins'].items():
        assert preparation.sha(path) == digest
    assert not (preparation.HERE / 'proposals/private-input-archive-fixture-v5.json').exists()
    return diagnostic


def main():
    global FD
    print('WAIT common mutex: Crystal recorded archive-filter-v6 six metadata mocks', flush=True)
    with preparation.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        FD = lock.fileno()
        failure = preserve_metadata_failure()
        output = io.StringIO()
        result = unittest.TextTestRunner(stream=output, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(ArchiveChecks))
        text = output.getvalue()
        print(text, end='', flush=True)
        log = preparation.HERE / 'archive-v6-fixtures.log'
        with log.open('x') as stream:
            stream.write(text)
        preparation.new_json(preparation.HERE / 'archive-v6-fixtures.json', dict(
            status='passed' if result.wasSuccessful() else 'failed', tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors), native_probes=0,
            actual_signals_sent=0, source_revision='input-label-archive-filter-v6', never_chart_eligible=True,
            original112probes_reexecuted=False, prior_fixture_methods_reexecuted=False,
            shared_timing_lock=str(preparation.LOCK),
            pins={str(path): preparation.sha(path) for path in (
                Path(__file__).resolve(), preparation.HERE / 'guarded_prepare_v6.py',
                preparation.HERE / 'proof_reuse_v6.py', preparation.HERE / 'ARCHIVE_FILTER_V6.md',
                preparation.HERE / 'guarded_prepare_v3.py', preparation.HERE / 'proof_reuse_v3.py',
                preparation.HERE / 'input-v3-fixtures.json', failure, log)}))
        if not result.wasSuccessful() or result.testsRun != 6:
            raise SystemExit(1)


if __name__ == '__main__':
    main()
