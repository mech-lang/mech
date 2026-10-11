#!/usr/bin/env python3
"""Freeze and compile general codegen policies without overwriting evidence."""
import argparse, hashlib, json, shutil, subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
NBODY = HERE.parents[1]

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--tag', required=True)
    ap.add_argument('--form', choices=['compact', 'pairwise'], required=True)
    ap.add_argument('--flags', default='')
    ap.add_argument('--manifest', type=Path, required=True)
    args = ap.parse_args()
    rows = json.loads(args.manifest.read_text()) if args.manifest.exists() else [
        row for row in json.loads((HERE / 'source-programs.json').read_text())
        if not row['id'].startswith('source-shared')
    ]
    ident = args.form + '-' + args.tag
    assert not any(row['id'] == ident for row in rows)
    driver = NBODY / 'mech-driver/target/release/mech-clbg-nbody-compiled'
    frozen = HERE / ('driver-' + args.tag)
    if not frozen.exists():
        shutil.copy2(driver, frozen)
    assert sha(frozen) == sha(driver)
    source = NBODY / ('nbody-matrix-short.mec' if args.form == 'compact' else 'nbody-matrix-pairwise.mec')
    obj, c, binary = [HERE / (ident + suffix) for suffix in ('.o', '.c', '')]
    assert not any(path.exists() for path in (obj, c, binary))
    flags = ['--fp-contract', '--algebraic-powers']
    if args.form == 'pairwise':
        flags.append('--pack-state-tails')
    flags.extend(args.flags.split())
    generation = [str(frozen), '0', str(source), 'emit-cranelift-simd', str(c), str(obj), *flags]
    subprocess.run(generation, check=True, capture_output=True)
    sdk = subprocess.check_output(['xcrun', '--show-sdk-path'], text=True).strip()
    build = ['/opt/homebrew/opt/llvm@22/bin/clang-22', '-O3', '-isysroot', sdk,
             str(c), str(obj), '-lm', '-o', str(binary)]
    subprocess.run(build, check=True, capture_output=True)
    rows.append(dict(id=ident, language='mech', source=str(source), source_sha256=sha(source),
        loc=20 if args.form == 'compact' else 25, generation_command=generation,
        compile_command=build, driver_sha256=sha(frozen), object_sha256=sha(obj),
        binary_sha256=sha(binary), command=[str(binary), '{n}'],
        state_control='compact-regions8' if args.form == 'compact' else 'pairwise-before',
        preserves_bits='--share-scaled-products' not in flags))
    args.manifest.write_text(json.dumps(rows, indent=2) + '\n')

if __name__ == '__main__':
    main()
