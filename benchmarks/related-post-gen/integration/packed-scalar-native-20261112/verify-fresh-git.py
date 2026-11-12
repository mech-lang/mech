#!/usr/bin/env python3
"""Verify with isolated real published Git objects, without temporary evidence paths."""
from pathlib import Path
import gzip
import hashlib
import json
import os
import subprocess
import tempfile
import zipfile
import zlib

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
manifest = json.loads((D / 'records.json').read_bytes())
records = {row['source']: row for row in manifest['records']}
archive = zipfile.ZipFile(D / manifest['container']['path'])


def read(name):
    return json.loads(gzip.decompress(archive.read(records[name]['path'])))


report = json.loads((D / 'report.json').read_bytes())
requirements = {}
for base in report['source_closure']['bases'].values():
    requirements[base['commit']] = set(read(base['inventory_record'])['files'])
base338 = report['source_closure']['vendor_base_commit']
requirements[base338].update(read(report['source_closure']['literal_inventory'])['literal_includes'])
requirements[base338].update(read(report['source_closure']['vendor_inventory']))
base340 = next(base['commit'] for name, base in report['source_closure']['bases'].items() if name == 'dense-worker-compiler-v2')
requirements[base340].update(name.removeprefix('published-base/') for name in records if name.startswith('published-base/'))
objects = set()
for commit, names in requirements.items():
    objects.add(commit)
    raw_commit = subprocess.run(['git', 'cat-file', 'commit', commit], cwd=ROOT, check=True, capture_output=True).stdout
    objects.add(raw_commit.splitlines()[0].split()[1].decode())
    trees = subprocess.run(['git', 'ls-tree', '-r', '-t', '--full-tree', commit], cwd=ROOT, check=True, capture_output=True).stdout
    for line in trees.splitlines():
        fields = line.split(b'\t', 1)[0].split()
        if fields[1] == b'tree':
            objects.add(fields[2].decode())
    specs = [commit + ':' + name for name in sorted(names)]
    found = subprocess.run(['git', 'cat-file', '--batch-check'], cwd=ROOT, check=True,
                           input=''.join(spec + '\n' for spec in specs).encode(), capture_output=True).stdout
    for line in found.splitlines():
        fields = line.split()
        if fields[-1] == b'missing':
            continue  # New vendor inputs are supplied by the packet overrides.
        assert fields[1] == b'blob'
        objects.add(fields[0].decode())

ordered = sorted(objects)
raw = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT, check=True,
                     input=''.join(oid + '\n' for oid in ordered).encode(), capture_output=True).stdout
with tempfile.TemporaryDirectory(prefix='packed-scalar-fresh-git-', dir='/private/tmp') as temp:
    repo = Path(temp) / 'objects.git'
    subprocess.run(['git', 'init', '--bare', str(repo)], check=True, capture_output=True)
    cursor = 0
    kinds = {}
    for oid in ordered:
        end = raw.index(b'\n', cursor)
        header = raw[cursor:end].split()
        assert header[0].decode() == oid and len(header) == 3
        kind, size = header[1], int(header[2])
        cursor = end + 1
        data = raw[cursor:cursor + size]
        cursor += size
        assert raw[cursor:cursor + 1] == b'\n'
        cursor += 1
        encoded = kind + b' ' + str(size).encode() + b'\0' + data
        assert hashlib.sha1(encoded).hexdigest() == oid
        target = repo / 'objects' / oid[:2] / oid[2:]
        target.parent.mkdir(exist_ok=True)
        target.write_bytes(zlib.compress(encoded))
        kinds[kind.decode()] = kinds.get(kind.decode(), 0) + 1
    assert cursor == len(raw)
    for index, commit in enumerate(sorted(requirements)):
        target = repo / 'refs' / 'heads' / ('published-base-' + str(index))
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(commit + '\n')
    env = os.environ.copy()
    for name in ('GIT_OBJECT_DIRECTORY', 'GIT_ALTERNATE_OBJECT_DIRECTORIES', 'GIT_WORK_TREE', 'GIT_COMMON_DIR'):
        env.pop(name, None)
    env['GIT_DIR'] = str(repo)
    result = subprocess.run(['python3', str(D / 'verify.py')], cwd=ROOT, env=env,
                            capture_output=True, text=True)
    if result.returncode:
        print(result.stderr, end='')
        raise SystemExit(result.returncode)
    verification = json.loads(result.stdout)
    assert verification['optional_published_IR349_alias_checked'] is False
    print(json.dumps(dict(status='passed', isolated_real_Git_objects=True,
                         synthetic_commit_aliases=False, required_published_commits=sorted(requirements),
                         imported_objects=len(objects), imported_object_types=kinds,
                         optional_IR349_commit_intentionally_absent=True,
                         native_executions=0, verification=verification), indent=2))
