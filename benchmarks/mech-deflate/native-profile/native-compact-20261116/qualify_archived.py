"""Read the committed benchmark campaigns through the maintained validator."""

import argparse
import gzip
from hashlib import sha256
import json
from pathlib import Path, PurePosixPath


RUNTIME_ARCHIVES = {
    "cold-tails": ("cold-tails",),
    "fresh-owner-counts": ("fresh-owner-counts/counts",),
    "fresh-owner-counts-cleanup": ("fresh-owner-counts/counts-cleanup",),
    "owner-ranges": ("owner-proof-runtime/owner-ranges",),
    "owner-all-inline-4879": ("owner-proof-runtime/owner-all-inline-4879",),
    "owner-cleanup": ("owner-cleanup-full", "owner-cleanup-focus"),
    "width-6-owners-cold": ("continuation-widths/width-6-owners-cold",),
    "width-16-owners-cold": ("continuation-widths/width-16-owners-cold",),
}
ACTORS = ("mech-control", "mech-experiment", "roc", "libdeflate")
PACKET = PurePosixPath("benchmarks/mech-deflate/native-profile/native-compact-20261116")


class Archive:
    def __init__(self, root):
        self.root = root
        ledger = json.loads((root / "source-and-evidence-manifest.json").read_bytes())
        self.files = {}
        for file in ledger["files"]:
            path = PurePosixPath(file["path"])
            if path.is_relative_to(PACKET):
                self.files[path.relative_to(PACKET)] = file["sha256"]
        self.contract = self.load(PurePosixPath("fixtures.json"))
        self.case_order = {row["name"]: index for index, row in enumerate(self.contract)}
        self.static = {}
        for folder in ("size-screens", "continuation-widths"):
            index = PurePosixPath("evidence", folder, "index.json")
            for case in self.load(index)["cases"]:
                self.static[case["name"]] = case

    def ref(self, path):
        return {"path": str(path), "sha256": self.files[path]}

    def load(self, path):
        data = (self.root / path).read_bytes()
        if sha256(data).hexdigest() != self.files[path]:
            raise ValueError(f"Archive file digest differs from its committed ledger: {path}")
        return json.loads(gzip.decompress(data) if path.suffix == ".gz" else data)

    def first(self, directories, *names):
        return next((directory / name for directory in directories for name in names
                     if directory / name in self.files), None)

    def folders(self, directories, filename):
        return sorted({path.parent for path in self.files if path.name == filename
                       and any(path.is_relative_to(directory) for directory in directories)})

    def run(self, directory):
        summary = self.load(directory / "summary.json")
        indices = [self.case_order[row["case"]] for row in summary]
        reverse = indices[0] > indices[-1] if len(indices) > 1 else directory.name.endswith("-reverse")
        order = "reverse" if reverse else "forward"
        return {
            "id": str(directory),
            "order": order,
            "kind": "full" if directory.name in {"full", "full-reverse"} else "focus",
            "pins": self.ref(directory / "pins.json"),
            "summary": self.ref(directory / "summary.json"),
            "complete": self.ref(directory / "complete.json"),
            "cleanup": {actor: self.ref(path) for actor in ACTORS
                        if (path := directory / (actor + ".stderr")) in self.files},
            "observations": {path.parent.name: self.ref(path) for path in self.files
                             if path.name == "observations.json.gz"
                             and path.parent.parent == directory},
        }

    def repeatability(self, directories):
        return [{
            "id": str(directory),
            "study": self.ref(directory / "study.json"),
            "results": self.ref(directory / "results.json"),
            "observations": {path.name.removesuffix(".json.gz"): self.ref(path) for path in self.files
                             if path.suffixes == [".json", ".gz"] and path.parent == directory},
        } for directory in self.folders(directories, "study.json")]

    def mech(self, directories, worker):
        artifact = {}
        if directory_receipt := self.first(directories, "receipt.json"):
            recorded = self.load(directory_receipt)
            for target, field in (
                ("worker", "worker_sha256"), ("metadata", "metadata_sha256"), ("object", "object_sha256")
            ):
                if digest := recorded.get(field):
                    artifact[target] = {"sha256": digest}
        if "worker" not in artifact and worker:
            artifact["worker"] = {"sha256": worker}
        if metadata := self.first(directories, "metadata.json"):
            if self.files[metadata] == artifact.get("metadata", {}).get("sha256"):
                artifact["metadata"] = self.ref(metadata)
        if options := self.first(directories, "runtime.json", "recipe-as-run.json"):
            artifact["options"] = self.ref(options)
            run = self.load(options)
            if digest := run.get("emitter_sha256"):
                artifact["compiler"] = {"sha256": digest}
            if digest := run.get("image_sha256"):
                image = PurePosixPath("program.mecb")
                artifact["bytecode"] = self.ref(image) if self.files[image] == digest else {"sha256": digest}
        return artifact

    def checks(self, directories):
        checks = {}
        correctness = {}
        for target, names in (("receipt", ("correctness.json",)), ("calls", ("correctness-calls.json.gz",))):
            if path := self.first(directories, *names):
                correctness[target] = self.ref(path)
        if correctness:
            checks["correctness"] = correctness
        if path := self.first(directories, "checked-failures.json", "checked-failures.json.gz"):
            checks["checked_failures"] = self.ref(path)
        if path := self.first(directories, "stack.json", "stack.json.gz"):
            checks["stack"] = {
                "control": self.ref(PurePosixPath("evidence/baseline/stack.json")),
                "candidate": self.ref(path),
            }
        return checks

    def manifest(self, candidate):
        manifest = {
            "schema_version": 1,
            "policy_version": 1,
            "objective": "encoder_text_bytes",
            "contract": self.ref(PurePosixPath("fixtures.json")),
            "artifacts": {actor: {} for actor in ACTORS},
            "runs": [],
            "duplicate_controls": [],
            "checks": {},
        }
        if candidate in self.static:
            record = self.static[candidate]
            manifest["artifacts"]["mech-experiment"] = {
                target: {"sha256": record["expected"][field]}
                for target, field in (("object", "object_sha256"), ("metadata", "metadata_sha256"))
            }
            return manifest

        directories = tuple(PurePosixPath("evidence", name) for name in RUNTIME_ARCHIVES[candidate])
        runs = [self.run(directory) for directory in self.folders(directories, "complete.json")]
        pins = self.load(PurePosixPath(runs[0]["pins"]["path"]))
        baseline = PurePosixPath("evidence/baseline")
        manifest["artifacts"].update({
            "mech-control": self.mech((baseline,), pins["actors"]["mech-control"]["binary_sha256"]),
            "mech-experiment": self.mech(directories, pins["actors"]["mech-experiment"]["binary_sha256"]),
        })
        source = PurePosixPath("program.mec")
        if self.files[source] == pins.get("source_sha256"):
            for actor in ("mech-control", "mech-experiment"):
                manifest["artifacts"][actor]["source"] = self.ref(source)
        for actor in ("roc", "libdeflate"):
            peer = pins["actors"][actor]
            manifest["artifacts"][actor]["worker"] = {"sha256": peer["binary_sha256"]}
            if digest := peer.get("metadata_sha256"):
                manifest["artifacts"][actor]["metadata"] = {"sha256": digest}

        repeatability = directories
        if candidate == "width-16-owners-cold":
            repeatability += (PurePosixPath("evidence/continuation-widths/repeatability"),)
        manifest.update(runs=runs, checks=self.checks(directories),
                        duplicate_controls=self.repeatability(repeatability))
        return manifest


def main():
    root = Path(__file__).resolve().parent
    archive = Archive(root)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", nargs="?", choices=tuple(RUNTIME_ARCHIVES) + tuple(archive.static))
    parser.add_argument("--manifest", action="store_true", help="print an archived input manifest")
    parser.add_argument("--list", action="store_true", help="list archived configurations")
    args = parser.parse_args()
    if args.list:
        print("\n".join((*RUNTIME_ARCHIVES, *archive.static)))
        return
    if args.candidate is None:
        parser.error("select one archived configuration")
    manifest = archive.manifest(args.candidate)
    if args.manifest:
        print(json.dumps(manifest, indent=2))
        return
    from qualify import evaluate
    print(json.dumps(evaluate(manifest, evidence_root=root), indent=2))


if __name__ == "__main__":
    main()
