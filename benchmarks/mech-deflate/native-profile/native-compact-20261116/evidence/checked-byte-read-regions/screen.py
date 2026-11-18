"""Screen one existing checked continuation family at a time for native size."""
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess
import sys

from regions import read_graph, reachable, postdominators, region

here = Path(__file__).resolve().parent
baseline = here.parent / 'phi-verifier/candidate'
ENDPOINT = 'MECH_COMPACT_CONTINUATION_WINDOWS'


def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def bound_inputs():
    receipt = read(baseline / 'rebuild/rebuild.json')
    compiler = baseline / 'compile_profile'
    assert sha(compiler) == (baseline / 'compiler.sha256').read_text().strip()
    assert receipt['full_integrated_source_identity']
    source = Path(receipt['phases']['source_to_native']['command'][1])
    assert sha(source) == '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
    return compiler, source, receipt['native_flags']


def run(variant, flags, expected_object=None):
    compiler, source, _ = bound_inputs()
    out = here / 'screen' / variant
    out.mkdir(parents=True)
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    args = [str(compiler), str(source), str(out / 'compiled'), 'compact']
    with (out / 'compile.log').open('wb') as log:
        subprocess.run(args, env=env | flags | {'PATH': '/usr/bin:/bin'}, stdout=log, stderr=log, check=True)
    program = out / 'compiled'
    output = {key: sha(program / key) for key in ('program.mecb', 'program.o', 'metadata.json')}
    assert output['program.mecb'] == 'c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3'
    if expected_object:
        assert output['program.o'] == expected_object
        assert output['metadata.json'] == sha(baseline / 'rebuild/native/metadata.json')
    root = read(program / 'metadata.json')
    subprocess.check_output([sys.executable, '-B', here.parent / 'function-compiler/root-census.py', program / 'program.o'])
    from importlib import import_module
    sys.path.insert(0, str(here.parent / 'function-compiler'))
    symbols = import_module('root-census').census(program / 'program.o')
    record = {'flags': flags, 'command': args, 'identity': output, 'root_code_bytes': root['code_bytes'],
              'root_extent_bytes': symbols['checked_root_extent_bytes'], 'stack_bytes': root['stack_bytes'],
              'encoder_text_bytes': symbols['encoder_text_bytes'], 'checked_continuations': root['checked_continuations']}
    write(out / 'result.json', record)
    print(variant, {key: record[key] for key in record if key.endswith('_bytes')}, flush=True)
    return record


def main():
    _, _, flags = bound_inputs()
    key = 'MECH_COMPACT_CONTINUATION_REGION_REPORT'
    before = here / 'screen/observe-before-sharing/blocks.tsv'
    run('observe-before-sharing', flags | {key: str(before)},
        '7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b')
    blocks = read_graph(before)
    live = reachable(blocks, next(iter(blocks)))
    _, predecessors = postdominators(blocks, live)
    original = read(here / 'regions.json')['largest_regions']
    summary = {}
    plain_flags = {key: value for key, value in flags.items() if key != 'MECH_COMPACT_ROOT_TAIL_PROFILE'}
    summary['control'] = run('control', plain_flags)
    for label, index in [('defaulted-word-read', 0), ('general-byte-read', 10)]:
        families = {}
        for site in original[index]['regions']:
            entry, exit_block = site['entry'], site['exit']
            if entry in live and exit_block in live:
                value = region(blocks, predecessors, entry, exit_block)
                if value:
                    key, member = value
                    families.setdefault(key, []).append(member)
        shapes = sorted(families.values(), key=len, reverse=True)
        sites = shapes[0] if shapes else []
        record = {'selected_instances': sites, 'original_instances': original[index]['instances']}
        if len(sites) > 1:
            family = ','.join(f"{site['entry']}:{site['exit']}:{site['origin_base']}" for site in sites)
            new_flags = plain_flags | {ENDPOINT: plain_flags[ENDPOINT] + ';' + family}
            record['output'] = run(label, new_flags)
        else:
            print(label, 'no surviving shape group at the sharing boundary', flush=True)
        summary[label] = record
    write(here / 'screen/summary.json', summary)


if __name__ == '__main__':
    main()
