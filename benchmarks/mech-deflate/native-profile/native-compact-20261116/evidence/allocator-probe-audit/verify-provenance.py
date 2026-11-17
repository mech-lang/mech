#!/usr/bin/env python3
"""Exercise both historical hash loops without a Git checkout for the fixture."""
import ast
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parent
repo = '/private/tmp/mech-shared-cold-epilogues.2L6obP'
before = 'd5bd13ddd3f5dfd97ee6118171c8674a511c04d0'
publication = '95c9a1f454681e2c4f6813b277d3e23277607d2c'
relative = Path('benchmarks/related-post-gen')
published = root / relative
probe = 'check-measurement.rs'
verifier = 'verify-review.py'
probe_path = str(relative / probe)
archived = Path('review/recorded-sources')
candidate_probe = (
    Path('/private/tmp/mech-allocator-audit.mo3o692k') / relative / probe
).read_bytes()

def show(ref, path):
    return subprocess.check_output(['git', '-C', repo, 'show', f'{ref}:{path}'])

def sha(data):
    return hashlib.sha256(data).hexdigest()

def assignment(node, name):
    return (
        isinstance(node, ast.Assign)
        and len(node.targets) == 1
        and isinstance(node.targets[0], ast.Name)
        and node.targets[0].id == name
    )

def compile_section(nodes):
    return compile(ast.Module(body=nodes, type_ignores=[]), str(relative / verifier), 'exec')

tree = ast.parse((published / verifier).read_text())
packet_loop = next(index for index, node in enumerate(tree.body) if isinstance(node, ast.For))
revision = next(index for index, node in enumerate(tree.body) if assignment(node, 'revision'))
integration = next(index for index, node in enumerate(tree.body) if assignment(node, 'integration'))
assert isinstance(tree.body[integration + 2], ast.If)
packet_code = compile_section(tree.body[:revision])
integration_code = compile_section(tree.body[:packet_loop] + tree.body[integration:integration + 3])

with tempfile.TemporaryDirectory(dir=root) as scratch:
    fixture_root = Path(scratch)
    packet = fixture_root / relative
    review = packet / 'review'
    review.mkdir(parents=True)
    (packet / 'integration').mkdir()
    shutil.copytree(published / archived, packet / archived)
    (packet / verifier).write_bytes((published / verifier).read_bytes())
    (packet / probe).write_bytes(candidate_probe)
    assert not (fixture_root / '.git').exists()
    manifest = json.loads(show(before, relative / 'review/manifest.json'))
    inventory = json.loads(show(before, relative / 'integration/source-inventory.json'))
    manifest['files'] = {name: manifest['files'][name] for name in (probe, verifier, 'README.md')}
    inventory['files'] = {name: inventory['files'][name] for name in (probe_path, 'build.rs')}
    (review / 'manifest.json').write_text(json.dumps(manifest))
    (packet / 'integration/source-inventory.json').write_text(json.dumps(inventory))
    (packet / 'README.md').write_bytes(show(publication, relative / 'README.md'))
    (fixture_root / 'build.rs').write_bytes(show(publication, 'build.rs'))
    assert sha(candidate_probe) != manifest['files'][probe]['sha256']
    assert sha((packet / verifier).read_bytes()) != manifest['files'][verifier]['sha256']
    for name in (probe, verifier):
        contents = gzip.decompress((packet / archived / f'{name}.gz').read_bytes())
        assert contents == show(publication, relative / name)
        assert sha(contents) == manifest['files'][name]['sha256']
        assert len(contents) == manifest['files'][name]['bytes']
    assert manifest['files'][probe]['sha256'] == inventory['files'][probe_path]

    def run(code):
        env = {'__file__': str(packet / verifier)}
        exec(code, env)
        return env

    def expect_failure(code, diagnostic, exc=SystemExit):
        try:
            run(code)
        except exc as error:
            assert diagnostic in str(error), (diagnostic, error)
        else:
            raise AssertionError(f'Expected {diagnostic}')

    run(packet_code)
    assert run(integration_code)['current_files'] == 2
    print('Both original receipt hashes verify with the edited probe and verifier present, without Git.')

    archive = packet / archived / f'{probe}.gz'
    contents = archive.read_bytes()
    archive.write_bytes(gzip.compress(b'wrong source', mtime=0))
    for code in (packet_code, integration_code):
        expect_failure(code, 'Hash mismatch: ' + str(archived / archive.name))
    archive.write_bytes(b'not a gzip file')
    for code in (packet_code, integration_code):
        expect_failure(code, 'Not a gzipped file', gzip.BadGzipFile)
    archive.unlink()
    for code in (packet_code, integration_code):
        expect_failure(code, 'Missing published file: ' + str(archived / archive.name))
    outside = fixture_root / 'outside-probe.gz'
    outside.write_bytes(contents)
    archive.symlink_to(outside)
    for code in (packet_code, integration_code):
        expect_failure(code, 'Invalid manifest path: ' + str(archived / archive.name))
    archive.unlink()
    archive.write_bytes(contents)
    print('Both hash loops reject missing, wrong, corrupt, and out-of-packet probe captures.')

    archived_verifier = packet / archived / f'{verifier}.gz'
    archived_verifier.write_bytes(gzip.compress(b'wrong verifier', mtime=0))
    expect_failure(packet_code, 'Hash mismatch: ' + str(archived / archived_verifier.name))
    shutil.copyfile(published / archived / archived_verifier.name, archived_verifier)
    (packet / 'README.md').write_bytes(b'changed ordinary packet member')
    expect_failure(packet_code, 'Hash mismatch: README.md')
    (fixture_root / 'build.rs').write_bytes(b'changed ordinary inventory member')
    expect_failure(integration_code, 'Hash mismatch: build.rs')
    print('The historical verifier pin and unrelated packet and inventory pins remain enforced.')
