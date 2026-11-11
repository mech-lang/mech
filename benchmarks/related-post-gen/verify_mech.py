#!/usr/bin/env python3
"""Compare complete Mech and original Rust outputs, including sparse tie behavior."""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess


def execute(command, cwd, timeout):
    child = subprocess.Popen(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
    try:
        stdout, stderr = child.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGKILL)
        child.communicate()
        raise
    if child.returncode:
        raise RuntimeError(f"{command}: {stdout}\n{stderr}")
    return stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("/private/tmp/mech-related-post-gen-20261102"))
    parser.add_argument("--runner", default="runner-aot")
    parser.add_argument("--timeout", type=float, default=120)
    parser.add_argument("--output", type=Path, default=Path("mech-correctness.json"))
    args = parser.parse_args()
    root = args.root.resolve()
    work = root / "correctness"
    work.mkdir(exist_ok=True)
    (work / "rust").mkdir(exist_ok=True)
    manifest = json.loads((root / "fixtures/manifest.json").read_text())
    cases = [(row["name"], root / "fixtures" / row["input"], root / "fixtures" / row["expected"]) for row in manifest["fixtures"]]
    for name, tags in [("all-empty", []), ("hundred-tags", [f"tag-{i}-日本語" for i in range(100)])]:
        path = work / f"{name}.json"
        path.write_text(json.dumps([{"_id":f"post-{i}","title":f"Title {i}","tags":tags} for i in range(8)], ensure_ascii=False))
        cases.append((name, path, None))
    report = {"schema":"mech.related-post-gen.correctness.v1", "cases":[]}
    for name, source, expected in cases:
        shutil.copy2(source, work / "posts.json")
        execute([str(root / "upstream/rust/target/release/rust")], work / "rust", args.timeout)
        execute([str(root / "mech" / args.runner), str(root / "mech/related.mec"), str(source), str(work / "mech.json"), "1"], root / "mech", args.timeout)
        actual = json.loads((work / "mech.json").read_text())
        baseline = json.loads((work / "related_posts_rust.json").read_text())
        assert actual == baseline, name
        if expected:
            posts = json.loads(source.read_text())
            by_id = {post["_id"]:i for i,post in enumerate(posts)}
            indices = [[by_id[post["_id"]] for post in row["related"]] for row in actual]
            assert indices == json.loads(expected.read_text())["related_indices"], name
        report["cases"].append({"case":name,"posts":len(actual),"complete_output_equals_original_rust":True,"expected_indices_verified":expected is not None})
        args.output.write_text(json.dumps(report, indent=2)+"\n")
        print(json.dumps(report["cases"][-1]), flush=True)
    report["status"] = "passed"
    args.output.write_text(json.dumps(report, indent=2)+"\n")


if __name__ == "__main__":
    main()
