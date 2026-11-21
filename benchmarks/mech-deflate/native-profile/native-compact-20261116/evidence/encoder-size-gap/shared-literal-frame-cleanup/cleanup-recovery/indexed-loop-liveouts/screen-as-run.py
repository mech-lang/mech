"""Compare source-built loop-use indexing against both exact M1 program artifacts."""

from hashlib import file_digest
import json
import os
from pathlib import Path
from shutil import copy2
import struct
import subprocess


HERE = Path(__file__).resolve().parent
ROOT = HERE/'loop-liveout-index'
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
    raise AssertionError('missing text')


def main():
    rebuilt = read(ROOT/'base/rebuild.json')
    assert rebuilt['full_integrated_source_identity'] and rebuilt['object_text_bytes'] == 64444
    recorded = read(HERE/'literal-frame-cleanup/receipt.json')
    control = read(HERE/'literal-frame-batched-schedule/reference/rebuild.json')
    binary = Path(rebuilt['phases']['source_to_native']['command'][0])
    assert sha(binary) == rebuilt['compiler_sha256']
    immutable = HERE/'reconsidered-size-compiler'/sha(binary)[:16]
    immutable.mkdir(exist_ok=True)
    compiler = immutable/'compile_profile'
    if not compiler.is_file():
        copy2(binary, compiler)
    assert sha(compiler) == rebuilt['compiler_sha256']
    save(immutable/'result.json', {'compiler_sha256': sha(compiler), 'source_commit': rebuilt['source_commit']})
    small = {'program.o': recorded['layout']['object_sha256'], 'metadata.json': recorded['metadata_sha256'],
             'program.mecb': recorded['bytecode_sha256']}
    large = {'program.o': control['object_sha256'], 'metadata.json': control['metadata_sha256'],
             'program.mecb': control['bytecode_sha256']}
    selections = {
        'control': (control['native_flags'], large, 64652),
        'unindexed': (rebuilt['native_flags'], small, 64444),
        'indexed': (rebuilt['native_flags'] | {'MECH_COMPACT_INDEX_LOOP_LIVE_OUTS': '1'}, small, 64444),
    }
    results = {}
    for name, (flags, expected, size) in selections.items():
        work = ROOT/name
        work.mkdir()
        output = work/'frontend'
        command = [str(compiler), rebuilt['phases']['source_to_native']['command'][1], str(output), 'compact']
        env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')} | flags
        with (work/'compile.log').open('wb') as log:
            subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        actual = {artifact: sha(output/artifact) for artifact in expected}
        result = {'source_commit': rebuilt['source_commit'], 'compiler_sha256': sha(compiler),
                  'command': command, 'flags': flags, 'text_bytes': text_bytes(output/'program.o'),
                  'object_sha256': actual['program.o'], 'metadata_sha256': actual['metadata.json'],
                  'bytecode_sha256': actual['program.mecb'], 'matches_recorded_artifacts': actual == expected}
        save(work/'receipt.json', result)
        assert actual == expected and result['text_bytes'] == size, result
        results[name] = {k: v for k, v in result.items() if k not in ['command', 'flags']}
        print(name, results[name], flush=True)
    save(ROOT/'comparison.json', results)


if __name__ == '__main__':
    main()
