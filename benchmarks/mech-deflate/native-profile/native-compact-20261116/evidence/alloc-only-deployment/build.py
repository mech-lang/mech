"""Build and link the already-prepared Mech experiment, recording each command."""
from argparse import ArgumentParser
from hashlib import file_digest
import json
from pathlib import Path
from shutil import copy2
import shlex
import subprocess

HERE = Path(__file__).resolve().parent


def sha(path):
    with Path(path).open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def execute(work, label, arguments):
    result = subprocess.run(arguments, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (work / f'{label}.log').write_bytes(result.stdout)
    if result.returncode:
        raise RuntimeError(f'{label} exit {result.returncode}: {result.stdout.decode(errors="replace")}')
    print(label, flush=True)
    return result.stdout.decode(errors='replace')


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument('--trial', type=Path, default=HERE/'trial')
    cli.add_argument('--mode', choices=('stock','abort-core'), default='stock')
    args = cli.parse_args()
    work = args.trial.resolve()
    plan = json.loads((work / 'plan.json').read_text())
    for path, digest in plan['fixed_input_sha256'].items():
        assert sha(path) == digest, path
    for name, fixed in plan['sources'].items():
        assert sha(work / name) == fixed['experiment_sha256'], name

    commands = {}
    stock = args.mode == 'stock'
    if stock:
        commands['compile-runtime'] = plan['runtime_rust_crate']
        execute(work, 'compile-runtime', commands['compile-runtime'])
        commands['compile-bridge'] = plan['bridge_rust_staticlib']
    else:
        commands['compile-bridge'] = plan['rust_src_fallback']
    log = execute(work, 'compile-bridge-' + args.mode, commands['compile-bridge'])
    native = [shlex.split(line.partition('native-static-libs: ')[2])
              for line in log.splitlines() if 'native-static-libs: ' in line]
    assert len(native) <= 1
    libraries = native[0] if native else []

    link = list(plan['clang_link_without_rust_native_libraries'])
    if not stock:
        link = [plan['fallback_bridge_archive'] if part.endswith('/libmech_deploy.a') else part for part in link]
    out_at = link.index('-o')
    link[out_at:out_at] = libraries
    commands['link'] = link
    execute(work, 'link-' + args.mode, link)
    executable = work / 'mech'
    stripped = work / 'mech-stripped'
    copy2(executable,stripped)
    commands['strip'] = plan['stripping']
    execute(work, 'strip-' + args.mode, commands['strip'])
    receipt = {'mode': args.mode, 'commands': commands, 'native_libraries': libraries, 'executable': {}}
    for name in ('mech', 'mech-stripped'):
        binary = work / name
        command = ['/usr/bin/size', '-m', str(binary)]
        receipt['executable'][name] = {'bytes': binary.stat().st_size, 'sha256': sha(binary),
                                       'sections': execute(work, 'sections-'+name, command)}
    (work/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print({k:v['bytes'] for k,v in receipt['executable'].items()}, flush=True)


if __name__ == '__main__':
    main()
