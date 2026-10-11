"""Build an additive same-driver-package control under the shared timing mutex.

Never changes frozen evidence or production sources. Only canonical small
fannkuch is executed; static code counts are not dynamic profiles or causation.
"""
from pathlib import Path
import collections
import datetime
import fcntl
import hashlib
import json
import os
import re
import shutil
import struct
import subprocess

HERE = Path(__file__).resolve().parent
C = HERE.parents[1]
ROOT = C.parents[1]
TARGET = Path('/tmp/mech-interpreter-integration-target')
MANIFEST = C / 'execution-modes/20261010/driver/Cargo.toml'
OLD_TARGET = Path('/tmp/mech-execution-modes-target')
CARGO = Path('/Users/cim310/.cargo/bin/cargo')
NM = Path('/opt/homebrew/opt/llvm@22/bin/llvm-nm')
OBJDUMP = Path('/opt/homebrew/opt/llvm@22/bin/llvm-objdump')
SYMBOL = '<mech_gpu::control::resident::Machine>::call'
BUILD_ENV = dict(CARGO_TARGET_DIR=str(TARGET), C_INCLUDE_PATH='/opt/homebrew/include',
                 LIBRARY_PATH='/opt/homebrew/lib')


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def sha(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def save(value):
    path = HERE / 'package-control.json'
    temporary = path.with_suffix('.json.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def archives():
    return {str(path): sha(path) for path in sorted((TARGET / 'release/deps').glob('lib*.rlib'))}


def function_bytes(binary, address, count):
    data = binary.read_bytes()
    assert struct.unpack_from('<I', data)[0] == 0xfeedfacf
    commands = struct.unpack_from('<I', data, 16)[0]
    offset = 32
    for _ in range(commands):
        command, size = struct.unpack_from('<II', data, offset)
        if command == 0x19:
            _, _, _, vmaddr, vmsize, fileoff, filesize, _, _, _, _ = struct.unpack_from('<II16sQQQQiiII', data, offset)
            if vmaddr <= address < vmaddr + vmsize:
                position = fileoff + address - vmaddr
                assert position + count <= fileoff + filesize
                return data[position:position + count]
        offset += size
    raise AssertionError('Function virtual address is not in a file-backed segment')


def inspect(binary, label):
    command = [str(NM), '-C', '-n', str(binary)]
    output = subprocess.check_output(command, text=True)
    text = []
    for line in output.splitlines():
        match = re.match(r'^([0-9a-f]+) [tT] (.+)$', line)
        if match:
            text.append((int(match[1], 16), match[2]))
    address = next(address for address, name in text if name == SYMBOL)
    end = next(other for other, _ in text if other > address)
    command = [str(OBJDUMP), '--disassemble', '--demangle', '--no-show-raw-insn',
               '--start-address=' + str(address), '--stop-address=' + str(end), str(binary)]
    disassembly = subprocess.check_output(command, text=True)
    destination = HERE / f'codegen-package-control-{label}-machine-call.txt'
    destination.write_text(disassembly)
    instructions = []
    for line in disassembly.splitlines():
        match = re.match(r'^[0-9a-f]+:\s+(.*)$', line)
        if match:
            instructions.append(match[1].split(';')[0].strip())
    stack = [instruction for instruction in instructions
             if re.search(r'\[(?:sp|x29)(?:,|\])', instruction)]
    loads = {'ldr', 'ldur', 'ldp', 'ldrb', 'ldrh', 'ldrsb', 'ldrsh', 'ldrsw',
             'ldurb', 'ldurh', 'ldursb', 'ldursh', 'ldursw'}
    stores = {'str', 'stur', 'stp', 'strb', 'strh', 'sturb', 'sturh'}
    body = function_bytes(binary, address, end - address)
    # Ignore PC-relative target addresses for a supplemental textual comparison.
    # Absolute data/GOT offsets are deliberately retained; raw archive identity
    # supplies the stronger compiled-library control.
    normalized = []
    for instruction in instructions:
        instruction = re.sub(r'0x[0-9a-f]+(?=\s*<)', 'ADDR', instruction)
        instruction = re.sub(r'<dyld_stub_binder\+0x[0-9a-f]+>', '<DYLD-STUB>', instruction)
        normalized.append(instruction)
    return dict(binary=str(binary), binary_sha256=sha(binary), symbol=SYMBOL,
        address=hex(address), cache_line_offset=address % 64,
        next_text_symbol_address=hex(end), size_to_next_text_symbol=end - address,
        instruction_lines=len(instructions),
        static_stack_load_instructions=sum(x.split()[0] in loads for x in stack),
        static_stack_store_instructions=sum(x.split()[0] in stores for x in stack),
        stack_address_bases=['sp', 'x29'],
        opcode_counts=dict(collections.Counter(x.split()[0] for x in instructions)),
        function_bytes_sha256=hashlib.sha256(body).hexdigest(),
        branch_address_normalized_text_sha256=hashlib.sha256('\n'.join(normalized).encode()).hexdigest(),
        disassembly_file=str(destination), disassembly_sha256=sha(destination), command=command)


def main():
    binary = HERE / '.build/package-control'
    report_path = HERE / 'package-control.json'
    assert not binary.exists() and not report_path.exists(), 'Do not overwrite an existing control'
    for name in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTUP_TOOLCHAIN',
                 'CARGO_PROFILE_RELEASE_LTO', 'CARGO_PROFILE_RELEASE_CODEGEN_UNITS',
                 'CARGO_PROFILE_RELEASE_OPT_LEVEL']:
        assert name not in os.environ, ('Unexpected build override', name)
    baseline = json.loads((HERE / 'baseline.json').read_text())
    pins = {str(HERE / 'baseline.json'): sha(HERE / 'baseline.json'),
            str(HERE / 'timings.json'): sha(HERE / 'timings.json'),
            baseline['binary']: baseline['binary_sha256'],
            baseline['after_binary']: baseline['after_binary_sha256'],
            str(MANIFEST): sha(MANIFEST),
            str(MANIFEST.with_name('Cargo.lock')): sha(MANIFEST.with_name('Cargo.lock')),
            str(ROOT / '.cargo/config.toml'): sha(ROOT / '.cargo/config.toml'),
            str(Path(__file__).resolve()): sha(__file__)}
    pins.update({str(ROOT / path): digest for path, digest in baseline['source_files'].items()})
    for path in (ROOT / 'hosts/gpu/src').rglob('*.rs'):
        pins[str(path)] = sha(path)
    before_library = OLD_TARGET / 'release/deps/libmech_gpu-816c26dd95eb7c0e.rlib'
    before_library_pin = sha(before_library)
    report = dict(status='waiting-for-shared-mutex', created_at=now(), scope=__doc__,
        pins=pins, original_driver_manifest=str(MANIFEST),
        build_environment=BUILD_ENV, ambient_build_overrides='All checked RUSTFLAGS/compiler/release-profile overrides absent.',
        source_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        shared_timing_lock='/tmp/mech-clbg-timing.lock',
        old_target_library=dict(path=str(before_library), sha256=before_library_pin),
        tools={str(tool): sha(tool.resolve()) for tool in [CARGO, NM, OBJDUMP]})
    save(report)
    print('Waiting for shared timing mutex', flush=True)
    with open('/tmp/mech-clbg-timing.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Acquired mutex; recording archive pins and building driver only', flush=True)
        for path, digest in pins.items():
            assert sha(path) == digest, path
        report['library_archives_before'] = archives()
        env = os.environ | BUILD_ENV
        report['rustc_version'] = subprocess.check_output(['rustc', '-vV'], cwd=ROOT, env=env, text=True)
        report['cargo_version'] = subprocess.check_output([str(CARGO), '-vV'], cwd=ROOT, env=env, text=True)
        report['build_command'] = [str(CARGO), 'build', '--release', '--locked', '--offline',
                                   '--verbose', '--manifest-path', str(MANIFEST),
                                   '--bin', 'mech-execution-modes-compiled']
        report.update(status='building', started_at=now())
        save(report)
        log = HERE / 'package-control-build.log'
        with log.open('w') as output:
            build = subprocess.run(report['build_command'], cwd=ROOT, env=env,
                                   stdout=output, stderr=subprocess.STDOUT)
        report.update(build_returncode=build.returncode, build_log=str(log), build_log_sha256=sha(log))
        report['library_archives_after'] = archives()
        old, new = report['library_archives_before'], report['library_archives_after']
        report['archive_comparison'] = dict(unchanged=old == new,
            added=sorted(new.keys() - old.keys()), removed=sorted(old.keys() - new.keys()),
            changed=[path for path in sorted(old.keys() & new.keys()) if old[path] != new[path]])
        for path, digest in pins.items():
            assert sha(path) == digest, path
        assert sha(before_library) == before_library_pin, 'Before target was changed'
        report['old_target_library']['unchanged'] = True
        if build.returncode != 0:
            report['status'] = 'build-failed'
            save(report)
            raise SystemExit(build.returncode)
        built = TARGET / 'release/mech-execution-modes-compiled'
        shutil.copy2(built, binary)
        report['control_binary'] = dict(path=str(binary), sha256=sha(binary), size=binary.stat().st_size)
        report['fresh_mech_gpu_log_evidence'] = [line for line in log.read_text().splitlines()
                                               if 'Fresh mech-gpu' in line or 'Compiling mech-gpu' in line]
        fingerprints = []
        for path in (TARGET / 'release/.fingerprint').glob('mech-execution-modes-*/bin-mech-execution-modes-compiled.json'):
            fingerprints.append(dict(path=str(path), sha256=sha(path), metadata=json.loads(path.read_text())))
        report['driver_fingerprints'] = fingerprints
        library_fp = TARGET / 'release/.fingerprint/mech-gpu-816c26dd95eb7c0e/lib-mech_gpu.json'
        report['library_fingerprint'] = dict(path=str(library_fp), sha256=sha(library_fp),
                                            metadata=json.loads(library_fp.read_text()))
        source = next(row for row in baseline['programs']
                      if row['source_key'] == 'fannkuchredux:mech-fannkuch-default')
        command = [str(binary)] + [arg.replace('{n}', '7') for arg in source['command'][1:]]
        process = subprocess.run(command, cwd=ROOT, env=env | source['runtime_environment'],
                                 capture_output=True, timeout=90)
        expected = b'228\nPfannkuchen(7) = 16\n'
        stdout = HERE / 'package-control-small.stdout'
        stderr = HERE / 'package-control-small.stderr'
        stdout.write_bytes(process.stdout)
        stderr.write_bytes(process.stderr)
        report['small_validation'] = dict(n=7, command=command, returncode=process.returncode,
            bytes=len(process.stdout), sha256=sha(stdout), expected_sha256=hashlib.sha256(expected).hexdigest(),
            reference_match=process.returncode == 0 and process.stdout == expected,
            stdout=str(stdout), stderr=str(stderr), stderr_sha256=sha(stderr))
        report['codegen'] = dict(control=inspect(binary, 'control'),
                                after=inspect(Path(baseline['after_binary']), 'after-reference'))
        control, after = report['codegen']['control'], report['codegen']['after']
        metrics = ['size_to_next_text_symbol', 'instruction_lines', 'static_stack_load_instructions',
                   'static_stack_store_instructions', 'opcode_counts']
        report['static_comparison'] = dict(same_metrics=all(control[key] == after[key] for key in metrics),
            function_bytes_identical=control['function_bytes_sha256'] == after['function_bytes_sha256'],
            branch_address_normalized_text_identical=control['branch_address_normalized_text_sha256'] == after['branch_address_normalized_text_sha256'])
        for path, digest in pins.items():
            assert sha(path) == digest, path
        assert sha(before_library) == before_library_pin
        report.update(status='complete', finished_at=now(),
            limitation='No full workload measurement, dynamic profile, or causal attribution. '
                       'This control isolates the original driver package/bin identity while using '
                       'the current after source and cache. Static stack instructions include cold paths. '
                       'An unchanged archive proves dependent library codegen reused; it does not prove '
                       'equal cache behavior or full-workload performance.')
        save(report)
        assert report['small_validation']['reference_match']
    print(json.dumps(dict(report=str(report_path), control=report['control_binary'],
        archives=report['archive_comparison'], small_validation=report['small_validation']['reference_match'],
        codegen_metrics={phase: {key: value[key] for key in metrics[:-1]} for phase, value in report['codegen'].items()},
        comparison=report['static_comparison']), indent=2), flush=True)


if __name__ == '__main__':
    main()
