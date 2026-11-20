"""Package diagnostic records after all acquisition has finished."""
import argparse
from hashlib import sha256
import io
import json
from pathlib import Path
import tarfile


def digest(data):
    return sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("config", type=Path)
    parser.add_argument("diagnostics", type=Path)
    parser.add_argument("observations", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--bytecode", type=Path, required=True)
    parser.add_argument("--preliminary", type=Path)
    args = parser.parse_args()
    config = json.loads(args.config.read_text())
    summary = json.loads((args.observations / "summary.json").read_text())
    assert summary["complete"] and len(summary["rows"]) == 72
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    archives = {}

    def archive(name, members):
        pins = {}
        with tarfile.open(out / name, "w:gz", compresslevel=6) as bundle:
            for relative, value in sorted(members.items()):
                data = value if isinstance(value, bytes) else Path(value).read_bytes()
                info = tarfile.TarInfo(relative)
                info.size, info.mtime, info.mode = len(data), 0, 0o644
                bundle.addfile(info, io.BytesIO(data))
                pins[relative] = {"sha256": digest(data), "bytes": len(data)}
        archives[name] = {"sha256": digest((out / name).read_bytes()),
                          "bytes": (out / name).stat().st_size, "members": pins}

    proofs = {"config.json": args.config}
    for p in args.diagnostics.rglob("*"):
        if p.is_file():
            proofs["diagnostics/" + str(p.relative_to(args.diagnostics))] = p
    original = Path(config["runtime_source"])
    for name in ("mod.rs", "terminal_appends.rs", "zero_pages.rs"):
        proofs["original-runtime/" + name] = original / name
    proofs["original-runtime/runtime.rlib"] = Path(config["runtime_library"])
    native = Path(config["selected_object"])
    for relative, p in {"selected/program.o": native,
                        "selected/metadata.json": Path(config["metadata"]),
                        "selected/program.mecb": args.bytecode}.items():
        proofs[relative] = p
    for index, p in enumerate(config["c_objects"]):
        proofs["libdeflate/" + str(index) + "-" + Path(p).name] = Path(p)
    archive("runtime-native-proofs.tar.gz", proofs)
    observations = {str(p.relative_to(args.observations)): p
                    for p in args.observations.rglob("*") if p.is_file()}
    observations["fixtures-original.json"] = Path(config["fixtures"])
    observations["C-oracle.json"] = Path(config["C_oracle"])
    fixtures = {v["name"]: v for v in json.loads(Path(config["fixtures"]).read_text())}
    portable = {}
    for name in summary["plan"]["cases"]:
        portable[name] = {}
        for key in ("input", "prefix", "expected"):
            relative = "fixtures/" + name + "/" + key + ".bin"
            path = Path(fixtures[name][key])
            observations[relative] = path
            portable[name][key] = {"member": relative, "sha256": digest(path.read_bytes())}
    observations["fixtures-portable.json"] = (json.dumps(portable, indent=2) + "\n").encode()
    archive("diagnostic-observations.tar.gz", observations)
    if args.preliminary:
        preliminary = {}
        for directory in ("diagnostics", "smoke"):
            for p in (args.preliminary / directory).rglob("*"):
                if p.is_file():
                    preliminary[directory + "/" + str(p.relative_to(args.preliminary / directory))] = p
        archive("preliminary-cold-smoke.tar.gz", preliminary)
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    (out / "archives.json").write_text(json.dumps(archives, indent=2) + "\n")
    print(json.dumps({k: {a: b for a, b in v.items() if a != "members"}
                      for k, v in archives.items()}, indent=2), flush=True)


if __name__ == "__main__":
    main()
