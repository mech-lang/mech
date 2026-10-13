"""Eight unexecuted mocked layout/unchanged-guard checks; no native work."""
import ast
import fcntl
import io
from pathlib import Path
import tempfile
import unittest

import guarded_prepare_v2 as preparation

FD = None


class LayoutChecks(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(FD, 'Use the locked standalone fixture entry point')
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.release = Path(self.temporary.name) / 'release'
        self.package = self.release / 'crystal-1.21.1-1'
        for name, data in (
            ('bin/crystal', b'#!/usr/bin/env bash\n'),
            ('embedded/bin/crystal', bytes.fromhex('cafebabe') + b'mocked-universal-header'),
            ('src/prelude.cr', b'require "object"\n'),
            ('embedded/lib/libgc.a', b'mocked-archive'),
            ('embedded/lib/libpcre2-8.a', b'mocked-other-pcre'),
        ):
            path = self.package / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)

    def test_exact_native_path_and_external_library_priority(self):
        compiler, stdlib, libraries, wrapper = preparation.package_layout(self.release)
        self.assertEqual(compiler, self.package / 'embedded/bin/crystal')
        self.assertEqual(wrapper, self.package / 'bin/crystal')
        self.assertEqual(stdlib, self.package / 'src')
        self.assertEqual(libraries, [preparation.GMP / 'lib', preparation.PCRE2 / 'lib', self.package / 'embedded/lib'])

    def test_missing_embedded_compiler_has_no_wrapper_fallback(self):
        (self.package / 'embedded/bin/crystal').unlink()
        with self.assertRaises(AssertionError):
            preparation.package_layout(self.release)

    def test_missing_prelude_rejected(self):
        (self.package / 'src/prelude.cr').unlink()
        with self.assertRaises(AssertionError):
            preparation.package_layout(self.release)

    def test_unknown_version_root_rejected(self):
        (self.release / 'other-version').mkdir()
        with self.assertRaises(AssertionError):
            preparation.package_layout(self.release)

    def test_missing_gc_rejected_without_installation(self):
        (self.package / 'embedded/lib/libgc.a').unlink()
        with self.assertRaises(AssertionError):
            preparation.package_layout(self.release)

    def test_unexpected_outer_launcher_rejected(self):
        (self.package / 'bin/crystal').write_bytes(b'not-the-expected-shell-launcher')
        with self.assertRaises(AssertionError):
            preparation.package_layout(self.release)

    def test_shell_embedded_or_non_native_payload_rejected(self):
        (self.package / 'embedded/bin/crystal').write_bytes(b'#!/bin/sh\n')
        with self.assertRaises(AssertionError):
            preparation.package_layout(self.release)

    def test_previous_proved_guard_functions_are_exact_text(self):
        texts = [(preparation.HERE / name).read_text() for name in ('guarded_prepare.py', 'guarded_prepare_v2.py')]
        nodes = [{node.name: ast.get_source_segment(text, node) for node in ast.parse(text).body
                  if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))} for text in texts]
        for name in ('owned_command', 'probe', 'output_validation', 'build_command', 'native_images',
                     'merge', 'sha', 'load_guard', 'new_json', 'download', 'extract', 'pin_tree', 'main'):
            with self.subTest(function=name):
                self.assertEqual(nodes[0][name], nodes[1][name])


def main():
    global FD
    print('WAIT LOCK: Crystal additive layout-v2 mocks', flush=True)
    with preparation.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        FD = lock.fileno()
        output = io.StringIO()
        result = unittest.TextTestRunner(stream=output, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(LayoutChecks))
        text = output.getvalue()
        print(text, end='', flush=True)
        log = preparation.HERE / 'layout-v2-fixtures.log'
        assert not log.exists(), 'No overwrite or automatic fixture retry'
        with log.open('x') as stream:
            stream.write(text)
        preparation.new_json(preparation.HERE / 'layout-v2-fixtures.json', dict(
            status='passed' if result.wasSuccessful() else 'failed', tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors), native_probes=0,
            actual_signals_sent=0, never_chart_eligible=True, source_revision='layout-v2',
            prior24cases_reexecuted=False, shared_timing_lock=str(preparation.LOCK),
            pins={str(path): preparation.sha(path) for path in (
                Path(__file__).resolve(), preparation.HERE / 'guarded_prepare_v2.py',
                preparation.HERE / 'guarded_prepare.py', preparation.HERE / 'LAYOUT_V2.md', log)}))
        if not result.wasSuccessful() or result.testsRun != 8:
            raise SystemExit(1)


if __name__ == '__main__':
    main()
