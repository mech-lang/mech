#!/usr/bin/env python3
"""Measure serial paired final links and in-process calls of four deployment adapters."""
from argparse import ArgumentParser
from contextlib import ExitStack
from hashlib import file_digest, sha256
from itertools import permutations
from pathlib import Path
from statistics import median
from time import perf_counter_ns
import importlib.util
import json
import platform
import subprocess

ACTORS = ('mech-std', 'mech-alloc', 'roc', 'libdeflate')
CASE_BATCHES = {'empty': 32768, 'prefix-natural-4096': 1024, 'prefix-random-4096': 512}


def digest(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def timings_summary(rows, key, units):
    values = {actor: [row['samples'][actor][key] for row in rows] for actor in ACTORS}
    pairs = (
        ('mech-alloc', 'mech-std'), ('mech-std', 'roc'), ('mech-alloc', 'roc'),
        ('mech-std', 'libdeflate'), ('mech-alloc', 'libdeflate'),
    )
    return {
        'units': units,
        'median': {actor: median(samples) for actor, samples in values.items()},
        'paired_ratios': {
            f'{num}/{den}': {
                'median': median(a / b for a, b in zip(values[num], values[den], strict=True)),
                'minimum': min(a / b for a, b in zip(values[num], values[den], strict=True)),
                'maximum': max(a / b for a, b in zip(values[num], values[den], strict=True)),
            }
            for num, den in pairs
        },
    }


def linked_command(template, output, main):
    command = [
        f'-Wl,-map,{output}.map' if arg.startswith('-Wl,-map,') else arg
        for arg in template
    ]
    command[command.index('-o') + 1] = str(output)
    if main is not None:
        matches = [index for index, arg in enumerate(command) if Path(arg).name == 'main.o']
        assert len(matches) == 1, command
        command[matches[0]] = str(main)
    return command


def link_round(order, templates, directory, label, main=None):
    result = {'label': label, 'order': list(order), 'samples': {}}
    for actor in order:
        output = directory / actor
        command = linked_command(templates[actor], output, main)
        before = perf_counter_ns()
        run = subprocess.run(command, capture_output=True, check=False, timeout=30)
        elapsed = perf_counter_ns() - before
        (directory / f'{actor}.log').write_bytes(run.stdout + run.stderr)
        run.check_returncode()
        result['samples'][actor] = {
            'elapsed_ns': elapsed, 'sha256': digest(output), 'file_bytes': output.stat().st_size,
            'command': command, 'log': (directory / f'{actor}.log').name,
        }
    return result


def write_report(path, report):
    path.write_text(json.dumps(report, indent=2) + '\n')


def link_only(templates, out, trial, verified):
    link_root = out / 'final-links'
    link_root.mkdir()
    reference = {actor: digest(path) for actor, path in zip(
        ACTORS, [verified / 'mech', trial / 'mech', verified / 'roc', verified / 'libdeflate'],
        strict=True,
    )}
    warmup = link_round(ACTORS, templates, link_root, 'warmup')
    rows = []
    for block, rotation in enumerate((0, 1, 3, 2) * 2, start=1):
        order = ACTORS[rotation:] + ACTORS[:rotation]
        directory = link_root / f'{block:02d}'
        directory.mkdir()
        rows.append(link_round(order, templates, directory, block))
    summary = timings_summary(rows, 'elapsed_ns', 'ns/clang invocation')
    return {
        'scope': 'clang invocation, linking prebuilt files and writing executable plus map',
        'warmup': warmup, 'blocks': rows, 'summary': summary,
        'exact_reference_binary_sha256': reference,
        'all_links_match_reference_binary_sha256': all(
            sample['sha256'] == reference[actor]
            for row in [warmup, *rows] for actor, sample in row['samples'].items()
        ),
    }


def timer_command(source, out):
    loop = Path(__file__).resolve().with_name('loop.c')
    result = out / 'loop.o'
    return ['/usr/bin/clang', '-arch', 'arm64', '-mmacosx-version-min=15.0',
            '-O2', '-std=gnu11', '-Wall', '-Wextra', '-Werror',
            '-I', str(source), '-c', str(loop), '-o', str(result)]


def read_batch(process, actor, count, expected_bytes):
    process.stdin.write(f'{count}\n')
    process.stdin.flush()
    response = process.stdout.readline().split()
    assert len(response) == 2, (actor, response, process.poll())
    ticks, output_bytes = map(int, response)
    assert ticks > 0 and output_bytes == count * expected_bytes, (actor, count, output_bytes)
    return {
        'iterations': count, 'ticks': ticks, 'output_bytes': output_bytes,
        'ns_per_call': ticks * process.numer / process.denom / count,
    }


def start_timer(executable, fixture, stack):
    process = stack.enter_context(subprocess.Popen(
        [str(executable), str(fixture)], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, text=True, bufsize=1,
    ))
    # Kill before Popen.__exit__ on exceptional paths to avoid waiting for stdin.
    stack.callback(process.kill)
    readiness = process.stdout.readline().split()
    assert len(readiness) == 3 and readiness[0] == 'READY', (executable, readiness)
    process.numer, process.denom = map(int, readiness[1:])
    assert process.numer > 0 and process.denom > 0
    return process


def timing_case(index, verifier, binaries, fixture_root, case, c_oracle, count):
    name = case['name']
    input_file = fixture_root / case['input']
    input_bytes = verifier.pinned(input_file, case['input_sha256'])
    expected = verifier.pinned(fixture_root / case['expected'], case['expected_sha256'])
    prefix_path = fixture_root / case['prefix']
    prefix = verifier.pinned(prefix_path, prefix_path.name)
    assert expected.startswith(prefix) and len(prefix) == case['prefix_bytes']
    raw = expected[len(prefix):]
    assert len(input_bytes) == case['input_bytes'] and len(raw) == case['raw_bytes']
    expected_bytes = dict.fromkeys(ACTORS, len(raw))
    expected_bytes['libdeflate'] = c_oracle['raw_bytes']
    exact_checks = {}
    for actor, exe in binaries.items():
        command = [str(exe), str(input_file), 'check']
        output = subprocess.check_output(command, timeout=30)
        sha = sha256(output).hexdigest()
        assert len(output) == expected_bytes[actor]
        if actor == 'libdeflate':
            assert sha == c_oracle['output_sha256']
        else:
            assert output == raw
        assert verifier.raw_inflates_to(output, input_bytes), (actor, name)
        exact_checks[actor] = {'command': command, 'raw_bytes': len(output), 'raw_sha256': sha,
                               'raw_exact': True, 'independent_full_stream_inflate': True}

    rounds = list(permutations(ACTORS))
    rounds = rounds[index * 7:] + rounds[:index * 7]
    rows = []
    with ExitStack() as stack:
        processes = {actor: start_timer(binaries[actor], input_file, stack) for actor in ACTORS}
        warmups = [
            {'order': list(order), 'samples': {
                actor: read_batch(processes[actor], actor, count, expected_bytes[actor])
                for actor in order
            }}
            for order in (ACTORS, ACTORS[::-1])
        ]
        for order in rounds:
            rows.append({'order': list(order), 'samples': {
                actor: read_batch(processes[actor], actor, count, expected_bytes[actor])
                for actor in order
            }})
        clock = {actor: {'numer': process.numer, 'denom': process.denom}
                 for actor, process in processes.items()}
        for process in processes.values():
            process.stdin.write('0\n')
            process.stdin.flush()
            assert process.wait(timeout=10) == 0
    assert digest(input_file) == case['input_sha256']
    summary = timings_summary(rows, 'ns_per_call', 'ns/encode and release')
    print(name, json.dumps(summary), flush=True)
    return {
        'case': name, 'input_sha256': case['input_sha256'],
        'input_bytes': case['input_bytes'], 'protocol_commands': {
            actor: [str(binary), str(input_file)] for actor, binary in binaries.items()
        },
        'iterations_per_batch': count,
        'exact_output_checks': exact_checks, 'timebase': clock, 'warmups': warmups,
        'blocks': rows, 'summary': summary,
    }


def runtime(templates, out, source, inventory, oracle_path):
    dest = out / 'in-process'
    dest.mkdir()
    compile_command = timer_command(source, dest)
    subprocess.run(compile_command, timeout=30, check=True)
    link_results = link_round(ACTORS, templates, dest, 'compile driver', dest / 'loop.o')
    binaries = {actor: dest / actor for actor in ACTORS}
    spec = importlib.util.spec_from_file_location('oracle', source / 'verify.py')
    verify = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(verify)
    packet = inventory.parent
    cases = {row['name']: row for row in json.loads(verify.pinned(inventory, verify.INVENTORY_SHA256))}
    oracle = {row['case']: row for row in json.loads(verify.pinned(oracle_path, verify.C_ORACLE_SHA256))}
    report = {
        'scope': 'in-process raw no-prefix encode plus buffer cleanup, input preloaded',
        'system': platform.platform(),
        'clang_version': subprocess.check_output(['/usr/bin/clang','--version'], text=True),
        'driver': {'command': compile_command, 'source_sha256': digest(Path(__file__).with_name('loop.c')),
                   'object_sha256': digest(dest / 'loop.o')},
        'links': link_results,
        'fixture_inventory_sha256': verify.INVENTORY_SHA256,
        'libdeflate_oracle_sha256': verify.C_ORACLE_SHA256,
        'cases': [],
    }
    rounds = out / 'runtime.json'
    for index, (name, count) in enumerate(CASE_BATCHES.items()):
        report['cases'].append(timing_case(index, verify, binaries, packet, cases[name], oracle[name], count))
        write_report(rounds, report)


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('trial', type=Path)
    parser.add_argument('out', type=Path)
    here = Path(__file__).resolve().parent
    parser.add_argument('--deployment', type=Path, default=here.parent/'minimal-deployment')
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--inventory', type=Path, default=here.parents[1]/'fixtures.json')
    parser.add_argument('--skip-final-link', action='store_true')
    parser.add_argument('--runtime', action='store_true')
    args = parser.parse_args()
    trial, out = args.trial.resolve(), args.out.resolve()
    out.mkdir(exist_ok=True)
    deployment = args.deployment.resolve()
    source = deployment / 'source'
    verified = args.baseline.resolve()
    commands = json.loads((verified / 'build.json').read_text())['commands']
    templates = {
        'mech-std': commands['link-mech'],
        'mech-alloc': json.loads((trial / 'receipt.json').read_text())['commands']['link'],
        'roc': commands['link-roc'],
        'libdeflate': commands['link-libdeflate'],
    }
    receipt = out / 'link.json'
    if not args.skip_final_link:
        assert not receipt.exists(), receipt
        report = link_only(templates, out, trial, verified)
        write_report(receipt, report)
        print(json.dumps(report['summary'], indent=2), flush=True)
        print('Exact links:', report['all_links_match_reference_binary_sha256'], flush=True)
    if args.runtime:
        runtime(templates, out, source, args.inventory.resolve(), deployment/'libdeflate-output-oracle.json')


if __name__ == '__main__':
    main()
