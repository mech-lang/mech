#!/usr/bin/env python3
"""Reproduce the v0.4 stdlib relocation experiment without changing the checkout.

Prepare: python3 scripts/audit-v04-extraction.py /private/tmp/mech-v04-extraction
The directory must not already exist. Builds are separate explicit commands.
"""
from pathlib import Path
import argparse, difflib, io, json, os, re, shutil, subprocess, tarfile

BASELINE = 'c4777b7015fe8ff47fdfa48d18606c49aace7d97'
ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / 'audit/v04/evidence/extraction'

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', type=Path)
    parser.add_argument('--finalize', action='store_true', help='Apply the verified source-profile repair, re-home Mech integration tests, and refresh the root lock offline.')
    args = parser.parse_args()
    destination = args.destination.resolve()
    destination.mkdir()
    core = destination / 'mech'
    core.mkdir()
    archive = subprocess.check_output(['git', 'archive', BASELINE], cwd=ROOT)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(core, filter='data')
    external = destination / 'external'
    external.mkdir()
    moves = [('src/stdlib', 'stdlib'), ('machines', 'machines')]
    for old, new in moves:
        shutil.move(str(core / old), external / new)

    def relocated(path):
        for old, new in moves:
            prefix = core / old
            if path == prefix or prefix in path.parents:
                return external / new / path.relative_to(prefix)
        return path

    changes = []
    manifests = sorted(core.rglob('Cargo.toml')) + sorted(external.rglob('Cargo.toml'))
    for manifest in manifests:
        original = manifest.read_text()
        old_manifest = manifest
        if external in manifest.parents:
            rel = manifest.relative_to(external)
            old_manifest = core / ('src/stdlib' if rel.parts[0] == 'stdlib' else 'machines') / Path(*rel.parts[1:])
        def rewrite(match):
            value = match.group(1)
            old_target = (old_manifest.parent / value).resolve()
            new_target = relocated(old_target)
            if old_target == new_target and old_manifest == manifest:
                return match.group(0)
            return 'path = "' + os.path.relpath(new_target, manifest.parent) + '"'
        updated = re.sub(r'path\s*=\s*"([^"]+)"', rewrite, original)
        if manifest == core / 'Cargo.toml':
            updated = updated.replace('  "src/stdlib",\n', '')
        if manifest == external / 'stdlib/Cargo.toml':
            updated += '\n[workspace]\n\n[patch.crates-io]\n'
            for machine in sorted((external / 'machines').iterdir()):
                if (machine / 'Cargo.toml').exists():
                    updated += f'mech-{machine.name} = {{ path = "../machines/{machine.name}" }}\n'
        if updated != original:
            manifest.write_text(updated)
            changes.append(dict(path=str(manifest.relative_to(destination)), old=original, new=updated))
    assert not (core / 'src/stdlib').exists()
    assert not (core / 'machines').exists()
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    patch = ''.join(''.join(difflib.unified_diff(c['old'].splitlines(True), c['new'].splitlines(True), fromfile='a/'+c['path'], tofile='b/'+c['path'])) for c in changes)
    (EVIDENCE / 'extraction-manifests.patch').write_text(patch)
    (EVIDENCE / 'preparation.json').write_text(json.dumps(dict(baseline=BASELINE, destination=str(destination), moves=moves, changed_manifests=[c['path'] for c in changes], old_paths_absent=True, production_rust_changes=0, added_rust_lines=0, deleted_rust_lines=0, fixture_portability='stdlib integration tests still reference root fixtures; preserved as a known independent-test packaging failure'), indent=2)+'\n')
    if args.finalize:
        subprocess.run(['git', 'apply', str(EVIDENCE / 'source-only-cfg-fix.patch')], cwd=core, check=True)
        subprocess.run(['git', 'apply', '-p3', str(EVIDENCE / 'source-profile-feature-fix.patch')], cwd=external / 'stdlib', check=True)
        subprocess.run(['python3', str(EVIDENCE / 'rehome-tests.py'), str(destination)], check=True)
        command = ['cargo', '+nightly-2026-03-03', 'metadata', '--offline', '--format-version', '1', '--no-default-features']
        resolved = subprocess.run(command, cwd=core, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        (EVIDENCE / 'finalization-lock-resolution.log').write_text(resolved.stderr)
        if resolved.returncode:
            raise SystemExit(resolved.returncode)
        (EVIDENCE / 'finalization.json').write_text(json.dumps(dict(destination=str(destination), baseline=BASELINE, source_patches=['source-only-cfg-fix.patch', 'source-profile-feature-fix.patch'], retained_test_manifest_lines=len((core/'tests/stdlib-integration/Cargo.toml').read_text().splitlines()), lock_resolution_command=command, lock_resolution_exit_code=resolved.returncode, old_paths_absent=not (core/'src/stdlib').exists() and not (core/'machines').exists()), indent=2)+'\n')
    print(json.dumps(dict(destination=str(destination), changed_manifests=len(changes), old_paths_absent=True, finalized=args.finalize)))

if __name__ == '__main__':
    main()
