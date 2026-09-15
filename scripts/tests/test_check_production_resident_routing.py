import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "production_resident_routing",
    Path(__file__).resolve().parents[1] / "check-production-resident-routing.py",
)
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class BrowserProjectSeamTests(unittest.TestCase):
    def test_shipping_seams_pass(self):
        self.assertEqual(CHECK.check_required_product_seams(), [])

    def test_losing_interactive_root_symbols_is_rejected(self):
        read_text = Path.read_text

        def without_interactive_root(path, *args, **kwargs):
            source = read_text(path, *args, **kwargs)
            if path == CHECK.ROOT / "src/wasm/src/project.rs":
                source = source.replace("load_interactive_root_program", "load_root_program")
            return source

        with patch.object(Path, "read_text", without_interactive_root):
            failures = CHECK.check_required_product_seams()
        self.assertIn(
            "src/wasm/src/project.rs: missing resident production seam load_interactive_root_program",
            failures,
        )


if __name__ == "__main__":
    unittest.main()
