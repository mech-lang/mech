"""Count file regions and attribute linked text from the three deployment maps."""
from argparse import ArgumentParser
from importlib import import_module
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "linked-size-census"))
mach = import_module("linked-size-census")
other_input = mach.family


def family(path):
    if "libmech_deploy.a(" in path:
        member = path.partition("(")[2]
        if member.startswith("libmech_gpu_runtime."):
            return "fixed Mech runtime"
        if member.startswith("libmech_deploy."):
            return "Mech adapter"
        if member.startswith(("object-", "addr2line-", "gimli-", "rustc_demangle-")):
            return "Rust backtrace and symbolization"
        return "other Rust standard libraries"
    return other_input(path)


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("directory", type=Path, help="Fresh comparison build output")
    cli.add_argument("report", type=Path)
    args = cli.parse_args()
    mach.family = family
    results = {}
    for name in ("mech", "roc", "libdeflate"):
        file = args.directory / name
        sections, sizes = mach.file_regions(file.read_bytes())
        _, stripped = mach.file_regions(file.with_name(name + "-stripped").read_bytes())
        text = sections["__TEXT", "__text"]
        results[name] = {
            "unstripped": sizes, "stripped": stripped, "linked_text_bytes": text[1],
            "attributed_text": mach.text_attribution(file.with_suffix(".map"), text),
        }
    args.report.write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
