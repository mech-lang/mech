"""One meaningful regression: old failure and real frozen compile boundary."""
import importlib
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock
import directory_init_v8 as repair

COHORT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(COHORT))
source_proofs = importlib.import_module('source_proofs')
assert Path(source_proofs.__file__).resolve() == COHORT / 'source_proofs.py'


class DirectoryInitialization(unittest.TestCase):
    def test_fresh_nested_build_root_reaches_owned_compile_boundary(self):
        class StopBeforeCompiler(Exception): pass
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve(); (root / '.build').mkdir()
            build = root / '.build/private-v8'; snapshot = root / 'snapshot'
            output = root / 'output'; output.mkdir()
            preparation = SimpleNamespace(build_command=Mock(return_value=['mock-compiler']),
                owned_command=Mock(side_effect=StopBeforeCompiler))
            guard = SimpleNamespace(OUT=output)
            def invoke():
                return source_proofs.run(preparation, guard, None, {}, snapshot, build,
                    Path('/mock/compiler'), Path('/mock/regex'), Path('/mock/gmp'),
                    Path('/mock/sdk'), {}, {}, {}, 0, {})
            build.mkdir()
            with self.assertRaises(FileNotFoundError): invoke()
            preparation.owned_command.assert_not_called()
            build.rmdir()
            repair.initialize_build_root(build)
            with self.assertRaises(StopBeforeCompiler): invoke()
            self.assertTrue((build / 'bin/standard-primitives').is_dir())
            self.assertEqual(preparation.owned_command.call_count, 1)
            self.assertEqual(preparation.owned_command.call_args.args[2], ['mock-compiler'])
            with self.assertRaises(AssertionError): repair.initialize_build_root(build)
            self.assertTrue((build / 'bin/standard-primitives').is_dir())

