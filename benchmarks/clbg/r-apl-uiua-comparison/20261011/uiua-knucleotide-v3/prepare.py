#!/usr/bin/env python3
"""Derive additive EOF-comment parser repairs; preserve all original sources."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
ORIGINALS = {
    'compact': SUITE / 'sources/uiua/knucleotide/compact.ua',
    'bounded': SUITE / 'sources/uiua/knucleotide/fast.ua',
    'native-group': SUITE / 'uiua-knucleotide-v2/sources/fast.ua',
}
OLD = '"(?m)^;[^\\n]*\\n|\\r|\\n"'
NEW = '"(?m)^;[^\\n]*(?:\\n|$)|\\r|\\n"'
OLD_RECORD = '"(?m)^>THREE[^\\n]*\\n([^>]*)"'
NEW_RECORD = '"(?m)^>THREE[^\\n]*\\n((?:[^>\\n][^\\n]*(?:\\n|$)|\\n)*)"'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    base = json.loads((SUITE / 'uiua-manifest.json').read_text())
    native = json.loads((SUITE / 'uiua-knucleotide-v2/manifest.json').read_text())
    expected = {Path(row['source']): row['source_sha256']
                for row in base['programs'] + native['programs']
                if row['benchmark'] == 'knucleotide'}
    destinations = HERE / 'sources'
    destinations.mkdir(parents=True, exist_ok=True)
    rows = []
    for name, original in ORIGINALS.items():
        assert sha(original) == expected[original], original
        text = original.read_text()
        assert text.count(OLD) == 1, original
        assert text.count(OLD_RECORD) == 1, original
        repaired = text.replace(OLD, NEW).replace(OLD_RECORD, NEW_RECORD)
        destination = destinations / (name + '.ua')
        destination.write_text(repaired)
        assert len(text.splitlines()) == len(repaired.splitlines())
        rows.append(dict(profile=name, original_source=str(original),
                         original_source_sha256=sha(original), source=str(destination),
                         source_sha256=sha(destination), changed_lines=[3,4],
                         old_regex=OLD, new_regex=NEW,
                         old_record_regex=OLD_RECORD, new_record_regex=NEW_RECORD,
                         physical_loc=sum(bool(line.strip()) for line in repaired.splitlines())))
    (HERE / 'derivation.json').write_text(json.dumps(rows, indent=2) + '\n')
    for row in rows:
        print(row['profile'], row['physical_loc'], row['source_sha256'])


if __name__ == '__main__':
    main()
