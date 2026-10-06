#!/usr/bin/env python3
"""Reproduce the v0.4 tracked-blob census, history and declared dependency map.

Python standard library only. Physical lines include a final unterminated line;
nonblank lines include comments. Both measures describe stored text.
"""
from __future__ import annotations

import argparse
import collections
import csv
import hashlib
import json
from pathlib import Path
import platform
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
VERSION = "1.0"
BASELINE = "c4777b7015fe8ff47fdfa48d18606c49aace7d97"
HISTORY = "f7d551ca8914a72df575fd40b94dba82d1d23fe6"
LANGUAGES = {".rs": "Rust", ".py": "Python", ".js": "JavaScript", ".mjs": "JavaScript", ".ts": "TypeScript", ".sh": "Shell", ".ps1": "PowerShell", ".mec": "Mech/Mechdown", ".md": "Markdown", ".html": "HTML", ".css": "CSS", ".toml": "TOML", ".json": "JSON", ".yaml": "YAML", ".yml": "YAML", ".csv": "CSV", ".tsv": "TSV", ".svg": "SVG", ".txt": "Text", ".ebnf": "EBNF", ".mcfg": "Mech configuration", ".wgsl": "WGSL", ".c": "C", ".h": "C header", ".cpp": "C++", ".lock": "Lockfile"}
SOURCE = {"Rust", "Python", "JavaScript", "TypeScript", "Shell", "PowerShell", "Mech/Mechdown", "HTML", "CSS", "WGSL", "C", "C header", "C++"}
GENERATORS = {
    "src/syntax/src/document/parser/canonical_rules.rs": "scripts/generate-canonical-rule-registry.py",
    "src/syntax/src/document/parser/canonical/document_grammar.rs": "scripts/generate-canonical-document-grammar.py",
    "src/syntax/src/document/ast/document_core.rs": "scripts/generate-canonical-document-grammar.py",
    "docs/design/grammar-audit/document-certification.tsv": "scripts/generate-canonical-document-grammar.py",
    "docs/design/grammar-audit/document-rule-dispositions.tsv": "scripts/generate-canonical-document-grammar.py",
    "docs/design/grammar-audit/canonical-dependencies.tsv": "scripts/generate-canonical-dependencies.py",
    "docs/design/grammar-audit/grammar-component-boundaries.tsv": "scripts/generate-canonical-scc-report.py",
    "docs/design/grammar-audit/recursive-core.tsv": "scripts/generate-canonical-scc-report.py",
    **{f"tests/fixtures/resident-ekf/{n}": "scripts/generate-resident-ekf-fixture.py" for n in ("ekf-input-v1.bin", "ekf-input-v1.sha256", "ekf-v1.json")},
}


def git(*args: str) -> bytes:
    return subprocess.check_output(["git", *args], cwd=ROOT)


def dump(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def tree(revision: str) -> tuple[list[tuple[str, str, str]], dict[str, bytes]]:
    entries = []
    for item in git("ls-tree", "-rz", revision).split(b"\0"):
        if not item:
            continue
        info, name = item.split(b"\t", 1)
        mode, kind, blob = info.decode().split()
        if kind == "blob":
            entries.append((name.decode(), blob, mode))
    ids = list(dict.fromkeys(e[1] for e in entries))
    result = subprocess.run(["git", "cat-file", "--batch"], cwd=ROOT, input=("\n".join(ids) + "\n").encode(), stdout=subprocess.PIPE, check=True).stdout
    blobs, offset = {}, 0
    for blob in ids:
        end = result.index(b"\n", offset)
        _, kind, size = result[offset:end].split()
        assert kind == b"blob"
        offset = end + 1
        blobs[blob] = result[offset:offset + int(size)]
        offset += int(size) + 1
    assert offset == len(result)
    return entries, blobs


def classify(path: str, raw: bytes) -> dict:
    p = Path(path)
    parts = p.parts
    language = LANGUAGES.get(p.suffix, "Other text")
    try:
        content = raw.decode("utf-8")
        if b"\0" in raw:
            raise UnicodeError()
        lines = content.split("\n")
        if lines[-1] == "":
            lines.pop()
    except UnicodeError:
        content, lines, language = "", [], "Binary"
    subsystem = "/".join(parts[:2]) if parts[0] in {"src", "hosts", "machines"} and len(parts) > 2 else parts[0] if len(parts) > 1 else "repository-root"
    responsibility = subsystem
    if parts[:2] == ("src", "runtime") and len(parts) > 4 and parts[2] == "src":
        responsibility = "runtime/" + parts[3]
    elif parts[:2] == ("src", "syntax") and len(parts) > 4 and parts[2] == "src":
        responsibility = "syntax/" + parts[3]
    elif parts[:2] == ("src", "engine") and len(parts) > 4 and parts[2] == "src":
        responsibility = "engine/" + parts[3]
    elif parts[:2] == ("src", "core") and len(parts) > 4 and parts[2] == "src":
        responsibility = "core/" + parts[3]
    role = "unresolved"
    if any(x in parts for x in ("vendor", "third_party", "third-party")) or p.name.endswith(".min.js"):
        role = "third-party"
    elif "fixtures" in parts or p.suffix in {".stderr", ".stdout"}:
        role = "fixture"
    elif "tests" in parts or p.name.startswith("test_"):
        role = "test"
    elif any(x in parts for x in ("benchmarks", "benches", "benchmark")):
        role = "benchmark"
    elif "examples" in parts:
        role = "example"
    elif parts[0] in {"docs", "mika"} or p.suffix == ".md":
        role = "documentation"
    elif parts[0] in {"scripts", ".github", ".cargo", "installer"} or p.name in {"build.rs", "Dockerfile"}:
        role = "build-tooling"
    elif p.name in {"Cargo.toml", "Cargo.lock", "package.json", "package-lock.json"} or p.suffix in {".toml", ".yml", ".yaml", ".mcfg", ".lock"}:
        role = "configuration"
    elif parts[0] in {"src", "machines", "hosts", "assets", "pkg", "include"} and language in SOURCE:
        role = "production"
    elif p.name.startswith(("LICENSE", "NOTICE", "COPYING")):
        role = "license"
    elif p.name.startswith("."):
        role = "configuration"
    elif language in {"Binary", "SVG"}:
        role = "asset"
    if role == "production" and re.search(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", content):
        role = "production-mixed-tests"
    header = "\n".join(lines[:8])
    generated = path in GENERATORS or bool(re.search(r"(?im)^\s*(?://|#|/\*|\*)?\s*(?:@generated\b|auto[- ]generated\b|generated (?:by|from|file)\b)", header))
    notes = []
    if role == "production-mixed-tests":
        notes.append("Embedded cfg(test): whole-file mixed attribution; excluded from pure-production subtotal.")
    if generated:
        notes.append("Generated output; see generator field or first-eight-line source marker.")
    if role == "unresolved":
        notes.append("Role attribution requires manual investigation.")
    return dict(subsystem=subsystem, responsibility=responsibility, role="generated" if generated else role, underlying_role=role, language=language, bytes=len(raw), physical_lines=len(lines), nonblank_lines=sum(bool(x.strip()) for x in lines), source_file=language in SOURCE, generated_marker=generated, generator=GENERATORS.get(path), attribution="deterministic path rules", notes=notes)


def census(revision: str) -> tuple[list[dict], dict[str, bytes]]:
    entries, blobs = tree(revision)
    return [dict(path=p, blob=b, mode=m, **classify(p, blobs[b])) for p, b, m in entries], blobs


def totals(records: list[dict]) -> dict:
    return {"files": len(records), **{k: sum(r[k] for r in records) for k in ("bytes", "physical_lines", "nonblank_lines")}}


def groups(records: list[dict], field: str) -> list[dict]:
    by = collections.defaultdict(list)
    for row in records:
        by[row[field]].append(row)
    return [dict(name=key, **totals(value)) for key, value in sorted(by.items())]


def dependencies(records: list[dict], blobs: dict[str, bytes]) -> dict:
    nodes, edges, errors = [], [], []
    for row in records:
        if Path(row["path"]).name != "Cargo.toml":
            continue
        try:
            manifest = tomllib.loads(blobs[row["blob"]].decode())
        except tomllib.TOMLDecodeError as error:
            errors.append(dict(path=row["path"], error=str(error), role=row["role"]))
            continue
        name = manifest.get("package", {}).get("name")
        if not name:
            continue
        nodes.append(dict(id=name, path=row["path"], features=manifest.get("features", {}), workspace_member=not row["path"].startswith("tests/fixtures/")))
        sections = [("all", manifest)] + list(manifest.get("target", {}).items())
        for target, section in sections:
            for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                for alias, declaration in section.get(kind, {}).items():
                    item = declaration if isinstance(declaration, dict) else {"version": declaration}
                    edges.append(dict(source=name, target=item.get("package", alias), alias=alias, kind=kind, platform=target, optional=item.get("optional", False), path=item.get("path"), version=item.get("version"), default_features=item.get("default-features", True), features=item.get("features", []), manifest=row["path"]))
    return dict(meaning="Declared Cargo dependencies, including optional, build, dev and target-specific edges. Resolved build dependencies and runtime interactions have separate records. Invalid/template fixture manifests remain in parse_errors.", nodes=nodes, edges=edges, parse_errors=errors)


def history(old: list[dict], current: list[dict], start: str, end: str) -> dict:
    oldmap, newmap = {r["path"]: r for r in old}, {r["path"]: r for r in current}
    raw = git("diff", "--name-status", "-z", "--find-renames=50%", start, end).decode().split("\0")
    rows, index = [], 0
    while index < len(raw) and raw[index]:
        status = raw[index]
        index += 1
        before = after = raw[index]
        index += 1
        if status.startswith(("R", "C")):
            after = raw[index]
            index += 1
        a, b = oldmap.get(before), newmap.get(after)
        rows.append(dict(status=status, before=before if a else None, after=after if b else None, old_blob=a["blob"] if a else None, new_blob=b["blob"] if b else None, old_lines=a["physical_lines"] if a else 0, new_lines=b["physical_lines"] if b else 0, delta_lines=(b["physical_lines"] if b else 0) - (a["physical_lines"] if a else 0), subsystem=(b or a)["subsystem"]))
    assert sum(r["delta_lines"] for r in rows) == totals(current)["physical_lines"] - totals(old)["physical_lines"]
    return dict(comparison_commit=start, comparison_label="v0.3.5-beta release tag (ancestor); release-to-integration comparison", baseline_commit=end, rename_detection="git --find-renames=50%; capability history is classified separately", totals_before=totals(old), totals_after=totals(current), changes=rows)


def environment() -> dict:
    commands = [["rustc", "-Vv"], ["cargo", "-V"], ["rustup", "target", "list", "--installed"], ["wasm-pack", "--version"], ["git", "--version"], ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "--version"], ["sysctl", "-n", "machdep.cpu.brand_string"], ["system_profiler", "SPDisplaysDataType"]]
    output = {}
    for command in commands:
        try:
            r = subprocess.run(command, capture_output=True, text=True, timeout=20)
            output[" ".join(command)] = dict(exit_code=r.returncode, stdout=r.stdout, stderr=r.stderr)
        except (OSError, subprocess.TimeoutExpired) as error:
            output[" ".join(command)] = dict(error=str(error))
    return dict(platform=platform.platform(), python=sys.version, commands=output)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", default=BASELINE)
    parser.add_argument("--comparison", default=HISTORY)
    parser.add_argument("--output", type=Path, default=ROOT / "audit/v04/data")
    parser.add_argument("--capture-environment", action="store_true")
    args = parser.parse_args()
    current, blobs = census(args.baseline)
    old, _ = census(args.comparison)
    meta = dict(schema_version=1, counter_version=VERSION, baseline_commit=args.baseline, comparison_commit=args.comparison, scope="All tracked regular/symlink blob entries at baseline. Worktree edits, ignored build outputs, expanded macros and external dependency source excluded.", units="bytes, physical text lines, nonblank text lines (comments included); binary line count is zero", mixed_policy="Files with cfg(test) are reported as production-mixed-tests. Pure-production and mixed-file totals remain separate.", generated_policy="Reviewed generator/output map plus first-eight-line markers; unmarked outputs may remain unrecognized. Macro expansions are excluded.", classification_policy="Deterministic path-based attribution; unresolved records remain visible. Responsibility is independently recorded from role.", totals=totals(current), by_subsystem=groups(current, "subsystem"), by_role=groups(current, "role"), by_language=groups(current, "language"), unresolved=[r["path"] for r in current if r["role"] == "unresolved"], lockfile_sha256=hashlib.sha256(blobs[next(r["blob"] for r in current if r["path"] == "Cargo.lock")]).hexdigest())
    assert sum(r["physical_lines"] for r in meta["by_role"]) == meta["totals"]["physical_lines"]
    dump(args.output / "census.json", dict(metadata=meta, files=current))
    with (args.output / "census.csv").open("w", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=list(current[0]))
        writer.writeheader()
        writer.writerows(current)
    dump(args.output / "history.json", history(old, current, args.comparison, args.baseline))
    dump(args.output / "dependencies.json", dependencies(current, blobs))
    if args.capture_environment:
        dump(args.output / "environment.json", environment())
    additions = []
    names = set(git("ls-files", "--others", "--exclude-standard").decode().splitlines()) | set(git("diff", "--name-only", args.baseline).decode().splitlines())
    for name in sorted(names):
        path = ROOT / name
        reproducible_evidence_source = name.startswith("audit/v04/evidence/") and path.suffix in {".rs", ".py", ".sh", ".toml", ".mec", ".mcfg", ".md"}
        if not path.is_file() or name.startswith(("audit/v04/data/", "audit/v04/site/pkg/")) or (name.startswith("audit/v04/evidence/") and not reproducible_evidence_source) or name.endswith((".wasm", ".png")):
            continue
        raw = path.read_bytes()
        category = ("supplied assignment" if name == "audit/v04/assignment.txt" else
                    "reproduction sources retained with evidence" if reproducible_evidence_source else
                    "optional WASM inspection" if name.startswith("src/wasm/") else
                    "absorbed R-stack proof" if name.startswith(("examples/r_stack", "examples/r-stack", "scripts/demo-r-stack")) else
                    "audit and demo" if name.startswith(("audit/", "scripts/audit-v04")) else
                    "production corrections and integration")
        additions.append(dict(path=name, change_category=category, sha256=hashlib.sha256(raw).hexdigest(), baseline_bytes=next((r["bytes"] for r in current if r["path"] == name), 0), baseline_physical_lines=next((r["physical_lines"] for r in current if r["path"] == name), 0), **classify(name, raw)))
    categories = [{"category": category, "files": len(rows), "net_physical_lines": sum(r["physical_lines"]-r["baseline_physical_lines"] for r in rows), "net_bytes":sum(r["bytes"]-r["baseline_bytes"] for r in rows)} for category in sorted({r["change_category"] for r in additions}) for rows in [[r for r in additions if r["change_category"]==category]]]
    dump(args.output / "audit-additions.json", dict(scope="Changed source, reports, fixtures and reproduction code, including code retained under evidence. Generated datasets, transcripts, patches, images and compiled package contents have separate byte totals. Baseline and current source sizes are separate.", files=additions, by_change_category=categories, current_totals=totals(additions), net_physical_lines=sum(r["physical_lines"]-r["baseline_physical_lines"] for r in additions), net_bytes=sum(r["bytes"]-r["baseline_bytes"] for r in additions), evidence_bytes=sum(p.stat().st_size for p in (ROOT / "audit/v04/evidence").rglob("*") if p.is_file()), data_bytes=sum(p.stat().st_size for p in args.output.glob("*") if p.is_file())))
    print(json.dumps(dict(baseline=args.baseline, totals=meta["totals"], roles=meta["by_role"], unresolved=meta["unresolved"]), indent=2))


if __name__ == "__main__":
    main()
