"""Five isolated snapshot-input checks under the inherited common mutex."""
import ast
import fcntl
import hashlib
import os
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch
import guarded_prepare_v3 as preparation


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class CompileInputs(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve() / 'workspace'
        self.source = self.root / 'package/src/lib.rs'
        self.source.parent.mkdir(parents=True)
        (self.root / 'package/Cargo.toml').write_text('[package]\nname="mock"\nversion="0.0.0"\n')
        self.fixture = self.root / 'fixtures/trace.bin'
        self.fixture.parent.mkdir()
        self.fixture.write_bytes(b'\x00\xff\x00compile-time-fixture\n')
        self.helper = SimpleNamespace(sha=digest)

    def collect(self):
        return preparation.compile_inputs(self.helper, {str(self.source): digest(self.source)},
            self.root, (Path('package/src'),))

    def test_binary_and_text_literal_paths_are_complete(self):
        self.source.write_text('const TRACE: &[u8] = include_bytes!("../../fixtures/trace.bin");\n'
                               'const SOURCE: &str = include_str!( "lib.rs" );\n')
        result = self.collect()
        self.assertEqual(result['pins'], {str(self.fixture): digest(self.fixture), str(self.source): digest(self.source)})
        self.assertEqual(result['sites'][str(self.source)], [str(self.fixture), str(self.source)])

    def test_package_relative_concat_keeps_the_exact_input(self):
        self.source.write_text('const TRACE: &[u8] = include_bytes!(concat!(\n'
            'env!("CARGO_MANIFEST_DIR"), "/../fixtures/trace.bin",\n));\n')
        self.assertEqual(self.collect()['pins'], {str(self.fixture): digest(self.fixture)})

    def test_missing_escaping_and_unreviewed_forms_fail_before_build(self):
        outside = self.root.parent / 'outside.bin'
        outside.write_bytes(b'outside')
        for text in ('include_bytes!("../../fixtures/missing.bin")',
                     'include_bytes!("../../../outside.bin")',
                     'include_bytes!(env!("UNKNOWN_INPUT"))'):
            with self.subTest(text=text):
                self.source.write_text(text)
                with self.assertRaises(AssertionError):
                    self.collect()

    def test_snapshot_copies_binary_bytes_and_rejects_changed_pins(self):
        self.source.write_text('include_bytes!("../../fixtures/trace.bin")')
        inputs = self.collect()['pins']
        inputs[str(self.source)] = digest(self.source)
        destination = self.root.parent / 'copy'
        with patch.object(preparation, 'ROOT', self.root), patch.object(preparation, 'HERE', self.root / 'experiment'):
            preparation.snapshot(self.helper, inputs, destination)
            self.assertEqual((destination / 'fixtures/trace.bin').read_bytes(), self.fixture.read_bytes())
            self.fixture.write_bytes(b'changed')
            with self.assertRaises(AssertionError):
                preparation.snapshot(self.helper, inputs, self.root.parent / 'changed-copy')

    def test_previous_ownership_codegen_and_conformance_functions_are_unchanged(self):
        def functions(path):
            return {node.name: ast.dump(node, include_attributes=False)
                    for node in ast.parse(path.read_text()).body if isinstance(node, ast.FunctionDef)}
        old = functions(preparation.HERE / 'guarded_prepare.py')
        new = functions(preparation.HERE / 'guarded_prepare_v3.py')
        self.assertEqual(set(new), set(old) | {'compile_inputs'})
        self.assertEqual({name: value for name, value in old.items() if name not in {'seal', 'prepare'}},
                         {name: value for name, value in new.items() if name not in {'seal', 'prepare', 'compile_inputs'}})
        self.assertEqual(preparation.VARIANTS, ('strict', 'reduction', 'broad'))
        self.assertEqual(preparation.SIZES, (1, 2, 3, 7, 8, 9, 31, 32, 33, 65, 129))


if __name__ == '__main__':
    descriptor = int(os.environ['MECH_TIMING_LOCK_FD'])
    actual, expected = os.fstat(descriptor), preparation.LOCK.stat()
    assert (actual.st_dev, actual.st_ino) == (expected.st_dev, expected.st_ino)
    fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    unittest.main(verbosity=2)
