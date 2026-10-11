#!/usr/bin/env python3
"""Vendor all unstarred CLBG n-body entries in the selected language cohort."""
import concurrent.futures
import argparse
import hashlib
import json
import re
import urllib.request
from datetime import datetime, timezone
from html.parser import HTMLParser
from pathlib import Path

HERE = Path(__file__).resolve().parent
BASE = "https://benchmarksgame-team.pages.debian.net/benchmarksgame/"
EXTENSIONS = {"chapel": "chpl", "julia": "jl", "rust": "rs", "go": "go", "graalvmaot": "java", "gcc": "c", "gpp": "cpp"}

class Preformatted(HTMLParser):
    def __init__(self):
        super().__init__()
        self.active = False
        self.blocks = []
        self.all_text = []

    def handle_starttag(self, tag, attrs):
        if tag == "pre":
            self.active = True
            self.blocks.append("")

    def handle_endtag(self, tag):
        if tag == "pre":
            self.active = False

    def handle_data(self, data):
        self.all_text.append(data)
        if self.active:
            self.blocks[-1] += data

def download(url):
    return urllib.request.urlopen(url, timeout=60).read().decode("utf-8")

def fetch(item):
    language, variant, published_secs, published_gz, starred = item
    identifier = f"nbody-{language}-{variant}"
    url = f"{BASE}program/{identifier}.html"
    parser = Preformatted()
    parser.feed(download(url))
    source = parser.blocks[0].strip() + "\n"
    path = f"upstream/{identifier}.{EXTENSIONS[language]}"
    (HERE / path).write_text(source)
    return {
        "id": identifier, "language": {"graalvmaot": "java", "gcc": "c", "gpp": "cpp"}.get(language, language),
        "variant": variant, "path": path, "url": url,
        "sha256": hashlib.sha256(source.encode()).hexdigest(),
        "published_secs": published_secs, "published_gz": published_gz,
        "upstream_starred": starred,
        "upstream_execution": "GraalVM native-image AOT" if language == "graalvmaot" else language,
        "upstream_notes": parser.blocks[1].strip(),
    }

def main():
    args_parser = argparse.ArgumentParser(description=__doc__)
    args_parser.add_argument("--extend-native", action="store_true", help="append C/C++ and starred Rust without refreshing existing pinned sources")
    args = args_parser.parse_args()
    existing = json.loads((HERE / "manifest.json").read_text()) if (HERE / "manifest.json").exists() else None
    if existing and not args.extend_native:
        raise SystemExit("Manifest already pinned; remove it explicitly to refresh upstream sources.")
    (HERE / "upstream").mkdir(exist_ok=True)
    table = download(BASE + "performance/nbody.html")
    items = []
    # The upstream HTML omits optional </tr> and </td> closing tags.
    for row in re.split(r"<tr\b[^>]*>", table)[1:]:
        match = re.search(r'program/nbody-(chapel|julia|rust|go|graalvmaot|gcc|gpp)-(\d+)\.html', row)
        starred = "*" in row
        if match and (args.extend_native or not starred):
            language, variant = match.groups()
            identifier = f"nbody-{language}-{variant}"
            if existing and any(program["id"] == identifier for program in existing["programs"]):
                continue
            cells = re.split(r"<td\b[^>]*>", row)[1:]
            plain = [re.sub(r"<[^>]*>", "", cell).strip() for cell in cells]
            items.append((language, int(variant), float(plain[2]), int(plain[4].replace(",", "")), starred))
    if not items:
        raise RuntimeError("No candidate rows found")
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        programs = list(pool.map(fetch, items))
    parser = Preformatted()
    parser.feed(download(BASE + "license.html"))
    license_text = "\n".join(x.strip() for x in parser.all_text if x.strip()) + "\n"
    (HERE / "upstream/LICENSE.txt").write_text(license_text[license_text.index("Revised BSD license"):])
    report = existing or {
        "schema_version": 1, "retrieved_at": datetime.now(timezone.utc).isoformat(),
        "selection": "All unstarred n-body entries in CLBG 25.03 for Chapel, Julia, Rust, Go, and Java native AOT. Java sources are also run locally as separately labelled OpenJDK JIT programs.",
        "table_url": BASE + "performance/nbody.html", "programs": [],
    }
    report["programs"].extend(programs)
    if args.extend_native:
        report["extended_at"] = datetime.now(timezone.utc).isoformat()
        report["selection"] = "All listed n-body entries in CLBG 25.03 for Chapel, Julia, Rust, Go, Java native AOT, C gcc, and C++ g++; starred entries retained and architecture exclusions reported explicitly. Java runs locally as OpenJDK JIT. Existing source pins preserved."
    (HERE / "manifest.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"Pinned {len(programs)} programs, author comments, build notes, and license.")

if __name__ == "__main__":
    main()
