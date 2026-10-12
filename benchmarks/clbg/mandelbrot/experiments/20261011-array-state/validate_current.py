"""Bounded canonical source validation; serialized with all CLBG CPU work."""
import argparse
import collections
import fcntl
import hashlib
import json
from pathlib import Path
import re
import subprocess
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
LOCK = Path('/tmp/mech-clbg-timing.lock')
WIDTHS = (1, 7, 8, 9, 13, 31, 63, 64, 65, 129, 200, 257)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def command(argv):
    result = subprocess.run([str(x) for x in argv], capture_output=True, timeout=120)
    return dict(command=[str(x) for x in argv], returncode=result.returncode,
                stdout=result.stdout.decode(errors='replace'),
                stderr=result.stderr.decode(errors='replace'))


def reference(n, chunk):
    output = bytearray(f'P4\n{n} {n}\n'.encode())
    scale = 2.0 / n
    for y in range(n):
        ci = y * scale - 1.0
        row = bytearray((n + 7) // 8)
        for start in range(0, n, 8):
            cr = [x * scale - 1.5 for x in range(start, start + 8)]
            zr, zi, iteration = [0.0] * 8, [0.0] * 8, 0
            while True:
                tr, ti = [z * z for z in zr], [z * z for z in zi]
                inside = [r + i <= 4.0 for r, i in zip(tr, ti)]
                if iteration >= 50 or not any(inside):
                    break
                for step in range(chunk):
                    if step:
                        tr, ti = [z * z for z in zr], [z * z for z in zi]
                    next_zr = [r - i + c for r, i, c in zip(tr, ti, cr)]
                    next_zi = [(r + r) * i + ci for r, i in zip(zr, zi)]
                    zr, zi = next_zr, next_zi
                iteration += chunk
            for lane, live in enumerate(inside):
                x = start + lane
                if x < n and live:
                    row[x // 8] |= 128 >> (x % 8)
        output.extend(row)
    return bytes(output)


def inspect_object(path):
    result = command(['/opt/homebrew/opt/llvm@22/bin/llvm-objdump', '-d', path])
    if result['returncode']:
        return result
    path.with_suffix('.asm').write_text(result['stdout'])
    functions = {}
    current = None
    for line in result['stdout'].splitlines():
        found = re.match(r'^[0-9a-f]+ <([^>]+)>:', line)
        if found:
            current = found[1]
            functions[current] = []
        found = re.match(r'^\s*([0-9a-f]+):\s+[0-9a-f]+\s+(\S+)(?:\s+(.*))?$', line)
        if current and found:
            functions[current].append(dict(address=int(found[1], 16),
                                           mnemonic=found[2], operands=found[3] or ''))
    return {name: dict(instructions=len(instructions),
                       mnemonics=dict(sorted(collections.Counter(x['mnemonic'] for x in instructions).items())),
                       memory_instructions=[x for x in instructions if re.match(r'^(ld|st)', x['mnemonic'])])
            for name, instructions in functions.items()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--driver', type=Path, required=True)
    parser.add_argument('--driver-sha256', required=True)
    parser.add_argument('--driver-manifest', type=Path, required=True)
    parser.add_argument('--driver-manifest-sha256', required=True)
    args = parser.parse_args()
    target = HERE / 'validation-current'
    target.mkdir(exist_ok=True)
    clang = Path('/opt/homebrew/opt/llvm@22/bin/clang-22')
    with LOCK.open('a+') as lock:
        print('Waiting for shared timing lock for current canonical validation.', flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Acquired shared timing lock.', flush=True)
        assert sha(args.driver) == args.driver_sha256, 'Current compiler binary changed'
        assert sha(args.driver_manifest) == args.driver_manifest_sha256, 'Current compiler manifest changed'
        sdk = subprocess.check_output(['xcrun', '--show-sdk-path'], text=True).strip()
        pins = {str(p): sha(p) for p in [args.driver, args.driver_manifest, clang, Path(__file__),
            ROOT / 'src/engine/src/native_sequence/application.rs',
            ROOT / 'src/engine/src/native_sequence/grid.rs',
            ROOT / 'hosts/gpu/src/fused_f64/mod.rs',
            ROOT / 'hosts/gpu/src/fused_f64/native.rs',
            ROOT / 'hosts/gpu/src/fused_f64/map_runner.rs',
            ROOT / 'hosts/gpu/src/fused_f64/grid_runner.c',
            ROOT / 'benchmarks/clbg/mandelbrot/mech-driver/Cargo.toml',
            ROOT / 'benchmarks/clbg/mandelbrot/mech-driver/Cargo.lock',
            ROOT / 'benchmarks/clbg/mandelbrot/upstream/mandelbrot-200.pbm']}
        settings = Path(sdk).resolve() / 'SDKSettings.json'
        if settings.exists():
            pins[str(settings)] = sha(settings)
        profiles = [('scalar11', 'scalar-canonical', False, False),
                    ('lanes12', 'lanes-canonical', True, False),
                    ('lanes12-rows', 'lanes-rows-canonical', True, False),
                    ('five16', 'five-canonical', True, False),
                    ('five16-rows', 'five-rows-canonical', True, False),
                    ('five16-loop-fma', 'five-canonical', True, True),
                    ('five16-rows-loop-fma', 'five-rows-canonical', True, True),
                    ('frozen20', None, False, False),
                    ('frozen29', None, True, False),
                    ('frozen29-loop-fma', None, True, True)]
        report = dict(created_at=datetime.now(timezone.utc).isoformat(),
                      purpose='Small semantic validation only; no scored timing',
                      driver=str(args.driver), driver_sha256=sha(args.driver),
                      driver_manifest=str(args.driver_manifest), driver_manifest_sha256=sha(args.driver_manifest),
                      clang=str(clang), clang_version=command([clang, '--version']),
                      sdk=sdk, sdk_realpath=str(Path(sdk).resolve()),
                      source_metric='LOC only; one authored binding/statement per line',
                      lock=str(LOCK), pins=pins, profiles=[], images=[], differences=[])
        for identifier, suffix, simd, fma in profiles:
            source = HERE / ('mandelbrot-array-state-' + suffix + '.mec') if suffix else ROOT / 'benchmarks/clbg/mandelbrot' / ('mandelbrot-compact-program.mec' if identifier == 'frozen20' else 'mandelbrot-program.mec')
            pins[str(source)] = sha(source)
            c, obj, executable = (target / (identifier + extension) for extension in ('.c', '.o', ''))
            row = dict(id=identifier, source=str(source), source_sha256=sha(source),
                       loc=sum(bool(line.strip()) and not line.lstrip().startswith('--') for line in source.read_text().splitlines()),
                       simd=simd, loop_fma=fma, executable=str(executable))
            argv = [args.driver, source, 'emit-cranelift-simd' if simd else 'emit-cranelift', c, obj, '--source-program']
            if fma:
                argv.append('--fp-contract-loop')
            row['emission'] = command(argv)
            if row['emission']['returncode'] == 0:
                row['link'] = command([clang, '-O3', '-mcpu=native', '-ffp-contract=off', '-isysroot', sdk, c, obj, '-lm', '-pthread', '-o', executable])
                row.update(generated_c_sha256=sha(c), native_object_sha256=sha(obj), object_inspection=inspect_object(obj))
                pins[str(c)] = row['generated_c_sha256']
                pins[str(obj)] = row['native_object_sha256']
                if row['link']['returncode'] == 0:
                    row['binary_sha256'] = sha(executable)
                    pins[str(executable)] = row['binary_sha256']
            report['profiles'].append(row)
            save(target / 'report.json', report)
            print(identifier + ': ' + ('compiled' if row.get('link', {}).get('returncode') == 0 else 'FAILED'), flush=True)
        for n in WIDTHS:
            per_step, five_step = reference(n, 1), reference(n, 5)
            (target / f'reference-step-{n}.pbm').write_bytes(per_step)
            (target / f'reference-five-{n}.pbm').write_bytes(five_step)
            if per_step != five_step:
                report['differences'].append(dict(size=n, type='reference-disagreement'))
            if n == 200:
                official = (ROOT / 'benchmarks/clbg/mandelbrot/upstream/mandelbrot-200.pbm').read_bytes()
                report['official_200'] = dict(sha256=hashlib.sha256(official).hexdigest(), per_step_exact_match=official == per_step, five_step_exact_match=official == five_step)
            for profile in report['profiles']:
                if profile.get('link', {}).get('returncode') != 0:
                    continue
                for workers in (1, 3):
                    result = subprocess.run([profile['executable'], str(n), str(workers)], capture_output=True, timeout=60)
                    filename = f"{profile['id']}-{n}-workers-{workers}.pbm"
                    (target / filename).write_bytes(result.stdout)
                    item = dict(id=profile['id'], size=n, workers=workers, returncode=result.returncode,
                                stderr=result.stderr.decode(errors='replace'), path=filename, bytes=len(result.stdout),
                                sha256=hashlib.sha256(result.stdout).hexdigest(), per_step_exact_match=result.stdout == per_step,
                                five_step_exact_match=result.stdout == five_step)
                    report['images'].append(item)
                    if result.returncode or result.stdout != per_step or result.stdout != five_step:
                        report['differences'].append(item)
            save(target / 'report.json', report)
            print(f'N={n}: complete images, tails and worker state reset checked.', flush=True)
        report['pins_unchanged_at_end'] = all(sha(path) == digest for path, digest in pins.items())
        failed = [p['id'] for p in report['profiles'] if p.get('link', {}).get('returncode') != 0]
        report['status'] = 'passed' if not failed and not report['differences'] and report['pins_unchanged_at_end'] else 'failed'
        report['compile_failures'] = failed
        save(target / 'report.json', report)
        print(f"{report['status']}: {len(report['images'])} complete image checks, {len(report['differences'])} differences; pins unchanged={report['pins_unchanged_at_end']}", flush=True)
        if report['status'] != 'passed':
            raise SystemExit(1)


if __name__ == '__main__':
    main()
