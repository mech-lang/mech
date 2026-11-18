"""Confirm bytecode on small programs and record short-input parser timings."""

import json
from statistics import median

from study_tools import checkout, q, run, sha, study, write

out = study / "small"
out.mkdir()
control = q / "ordered-vector-cursors/before/compile_profile"
combined = q / "host-opt-levels/opt3-cargo/release/compile_profile"
actors = {"control": control, "combined": combined}
orders = [["control", "combined"], ["combined", "control"]] * 3
sources = [
  checkout / "tests/fixtures/syntax-source-boundary/executable.mec",
  checkout / "tests/fixtures/syntax-source-boundary/resolver-index.mec",
  checkout / "src/syntax/tests/fixtures/document/accepted/unicode.mec",
]
results = {}
for source in sources:
    case_dir = out / source.stem
    case_dir.mkdir()

    def measure(actor, label):
        name = f"{label}-{actor}"
        folder = case_dir / name
        command = run([actors[actor], source, folder, "frontend"], case_dir / f"{name}.log")
        receipt = json.loads((folder / "compile.json").read_text())
        assert sha(folder / "program.mecb") == receipt["bytecode_sha256"]
        return {"command":command,"bytecode_sha256":receipt["bytecode_sha256"],
                "phases":receipt["phases"],"total_seconds":receipt["total_seconds"]}

    warmup = {actor:measure(actor, "warmup") for actor in actors}
    trials = [{actor:measure(actor, f"order-{index}") for actor in order}
              for index, order in enumerate(orders,1)]
    signatures = {row["bytecode_sha256"] for pair in [warmup, *trials] for row in pair.values()}
    assert len(signatures) == 1
    results[str(source.relative_to(checkout))] = {
        "source_sha256":sha(source),"bytecode_sha256":signatures.pop(),"warmup":warmup,"trials":trials,
        "median_frontend_seconds":{actor:median(t[actor]["total_seconds"] for t in trials) for actor in actors},
        "median_parse_seconds":{actor:median(t[actor]["phases"][0]["seconds"] for t in trials) for actor in actors},
    }
write(out / "report.json", {"orders":orders,"results":results})
print(json.dumps({key:row["median_parse_seconds"] for key,row in results.items()},indent=2))
