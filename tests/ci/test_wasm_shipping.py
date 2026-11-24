"""Shipping omits debug names without rewriting executable WASM sections."""
import importlib.util
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("build_wasm", ROOT / "scripts/build-wasm.py")
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)


class WasmShippingTests(unittest.TestCase):
    def strip(self, module: bytes) -> bytes:
        with tempfile.TemporaryDirectory() as directory:
            wasm = Path(directory) / "module.wasm"
            wasm.write_bytes(module)
            BUILD.strip_debug_names(wasm)
            return wasm.read_bytes()

    def test_executable_and_other_custom_sections_remain_byte_identical(self):
        header = b"\0asm\x01\0\0\0"
        # One exported nullary function returning i32(42).
        types = b"\x01\x05\x01\x60\x00\x01\x7f"
        functions = b"\x03\x02\x01\x00"
        exports = b"\x07\x05\x01\x01f\x00\x00"
        code = b"\x0a\x06\x01\x04\x00\x41\x2a\x0b"
        names = b"\x00\x0b\x04name\x01\x04\x01\x00\x01f"
        other = b"\x00\x0b\x09producers\x00"
        expected = header + types + functions + exports + code + other
        self.assertEqual(self.strip(header + names + types + functions + exports + code + names + other), expected)
        self.assertEqual(self.strip(expected), expected)

    def test_large_debug_section_lengths_are_supported(self):
        header = b"\0asm\x01\0\0\0"
        names = b"\x00\x85\x01\x04name" + b"\x00" * 128
        self.assertEqual(self.strip(header + names), header)

    def test_invalid_or_truncated_modules_fail_before_replacing_the_file(self):
        for invalid in [b"", b"\0asm", b"\0asm\x01\0\0\0\x00\x80", b"\0asm\x01\0\0\0\x00\x01\x04"]:
            with self.subTest(module=invalid), tempfile.TemporaryDirectory() as directory:
                wasm = Path(directory) / "module.wasm"
                wasm.write_bytes(invalid)
                with self.assertRaises(ValueError):
                    BUILD.strip_debug_names(wasm)
                self.assertEqual(wasm.read_bytes(), invalid)


if __name__ == "__main__":
    unittest.main()
