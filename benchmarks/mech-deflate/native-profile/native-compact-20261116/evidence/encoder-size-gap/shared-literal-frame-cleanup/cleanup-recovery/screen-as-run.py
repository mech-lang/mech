"""Inspect final cleanup changes and prove resulting artifacts against the qualified worker."""

from hashlib import file_digest
import json
import os
from pathlib import Path
from shutil import copy2
import struct
import subprocess
import sys
from time import perf_counter


HERE = Path(__file__).resolve().parent
ROOT = HERE/(sys.argv[1] if len(sys.argv) > 1 else 'literal-frame-schedules')
read = lambda path: json.loads(path.read_text())
save = lambda path, value: path.write_text(json.dumps(value, indent=2)+'\n')


def sha(path):
    with Path(path).open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def text_bytes(path):
    data = Path(path).read_bytes()
    offset = 32
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        command, size = struct.unpack_from('<II', data, offset)
        if command == 0x19:
            for section in range(struct.unpack_from('<I', data, offset+64)[0]):
                at = offset+72+section*80
                if data[at:at+16].rstrip(b'\0') == b'__text':
                    return struct.unpack_from('<Q', data, at+40)[0]
        offset += size
    raise AssertionError('missing Mach-O text')


def main():
    baseline = read(ROOT/'reference/rebuild.json')
    assert baseline['full_integrated_source_identity'] and baseline['object_text_bytes'] == 64652
    original = read(HERE/'literal-frame-cleanup/receipt.json')
    binary = Path(baseline['phases']['source_to_native']['command'][0])
    assert sha(binary) == baseline['compiler_sha256']
    immutable = HERE/'reconsidered-size-compiler'/sha(binary)[:16]
    immutable.mkdir(exist_ok=True)
    compiler = immutable/'compile_profile'
    if not compiler.is_file():
        copy2(binary, compiler)
    assert sha(compiler) == baseline['compiler_sha256']
    save(immutable/'result.json', {'compiler_sha256': sha(compiler), 'source_commit': baseline['source_commit']})
    source = baseline['phases']['source_to_native']['command'][1]
    results = {}
    for schedule in sys.argv[2:] or ['1', 'ordered']:
        work = ROOT/('full' if schedule == '1' else schedule)
        work.mkdir()
        output = work/'frontend'
        flags = baseline['native_flags'] | {
            'MECH_COMPACT_LITERAL_ALLOCATION_FROM_FRAME': '1',
            'MECH_COMPACT_FINISH_CFG_CLEANUP': schedule,
        }
        report = work/'cleanup-report.json'
        diagnostic = flags | {'MECH_COMPACT_FINISH_CFG_REPORT': str(report)}
        command = [str(compiler), source, str(output), 'compact']
        env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
        with (work/'compile.log').open('wb') as log:
            start = perf_counter()
            subprocess.run(command, env=env | diagnostic, stdout=log, stderr=subprocess.STDOUT, check=True)
            elapsed = perf_counter()-start
        observed = {'program.o': sha(output/'program.o'), 'metadata.json': sha(output/'metadata.json'),
                    'program.mecb': sha(output/'program.mecb')}
        expected = {'program.o': original['layout']['object_sha256'],
                    'metadata.json': original['metadata_sha256'], 'program.mecb': original['bytecode_sha256']}
        result = {'text_bytes': text_bytes(output/'program.o'), 'object_sha256': observed['program.o'],
                  'metadata_sha256': observed['metadata.json'], 'bytecode_sha256': observed['program.mecb'],
                  'compiler_sha256': sha(compiler), 'source_commit': baseline['source_commit'],
                  'command': command, 'flags': flags, 'diagnostic_flags': diagnostic,
                  'single_untimed_diagnostic_seconds': elapsed,
                  'matches_qualified_64444_artifacts': observed == expected,
                  'cleanup': read(report)}
        if schedule == '1':
            assert observed == expected
        save(work/'receipt.json', result)
        results[schedule] = {k: v for k, v in result.items() if k not in ['flags', 'command', 'diagnostic_flags']}
        print(schedule, json.dumps(results[schedule], indent=2), flush=True)
    save(ROOT/'comparison.json', results)


if __name__ == '__main__':
    main()
