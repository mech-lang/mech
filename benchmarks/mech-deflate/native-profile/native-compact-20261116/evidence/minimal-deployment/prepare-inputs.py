"""Restore and verify the exact fixed inputs for the deployment links."""
from argparse import ArgumentParser
import gzip
from hashlib import sha256
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("output", type=Path, help="Fresh directory for fixed inputs")
    output = cli.parse_args().output.resolve()
    output.mkdir()
    manifest = {}
    for name, pin in json.loads((ROOT / "input-pins.json").read_bytes()).items():
        if "archive" in pin:
            data = gzip.decompress((ROOT / pin["archive"]).read_bytes())
            destination = output / name / pin["filename"]
        else:
            destination = (ROOT / pin["source"]).resolve()
            data = destination.read_bytes()
        if sha256(data).hexdigest() != pin["sha256"]:
            raise ValueError(f"Fixed input changed for {name}")
        if "archive" in pin:
            destination.parent.mkdir()
            destination.write_bytes(data)
        manifest[name] = {"path": str(destination), "sha256": pin["sha256"]}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
