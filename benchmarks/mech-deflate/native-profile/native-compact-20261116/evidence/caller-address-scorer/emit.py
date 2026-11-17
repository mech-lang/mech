"""Build and bind scorer experiments with the measured PR compiler."""
from hashlib import sha256
from importlib import import_module
import os
from pathlib import Path
from struct import unpack_from
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
ROOT = Q.parent
CHECKOUT = Q / 'native-review-pr/checkout'
PIN = '176a80bde45bdc0c137ddd797e5961c42f23d56c'
PLAN = ROADMAP / 'flowgraph-verification/production/matched-repeat/plan.json'
TEMPLATE = ROADMAP / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
HISTORY = Q / 'cranelift-horizontal-minimum-native/final-candidate'
LAYOUT = Q / 'cranelift-machine-outline-horizontal-minimum-parser-balanced-fallthroughs-native/final-candidate/report.json'
SCORER = '_mech_compact_numeric_3009_no_growth_composed'
HORIZONTAL = {'MECH_ORDERED_HORIZONTAL_MINIMUM': '1'}
REVERSE = {'MECH_ORDERED_REVERSE_LOW_WORDS': '1'}

sys.path.insert(0, str(Q))
macho = import_module('reachability-census')
sys.path.insert(0, str(ROADMAP / 'checked-root-census'))
link = import_module('recheck-link')
read, save = link.read, link.write


def sha(path):
    return link.sha(Path(path))


def symbols(path):
    image, sections, names = macho.load(path)
    named = macho.text_symbols(sections, names)
    _, base, size, offset, reloc_offset, reloc_count = sections[macho.TEXT]
    relocs = [
        (base + address, flags >> 24,
         names[flags & 0xFFFFFF][0] if flags >> 27 & 1 else flags & 0xFFFFFF)
        for address, flags in (unpack_from('<II', image, reloc_offset + i * 8) for i in range(reloc_count))
    ]
    atoms = {}
    offsets = sorted(set(named.values()) | {base + size})
    bounds = dict(zip(offsets, offsets[1:]))
    for name, begin in named.items():
        end = bounds[begin]
        data = image[offset + begin - base:offset + end - base]
        atoms[name] = {'extent': end - begin, 'start': begin, 'sha256': sha256(data).hexdigest(),
                       'relocations': [[at - begin, flags, target] for at, flags, target in relocs
                                       if begin <= at < end]}
    return {'text_bytes': size, 'object_sha256': sha(path), 'symbols': atoms}


def profile():
    original = HERE / 'horizontal/frontend/program.o'
    historical = HISTORY / 'program.o'
    report = read(LAYOUT)
    assert report['old_sha256'] == sha(historical)
    reference = symbols(historical)['symbols'][SCORER]
    candidate = symbols(original)['symbols'][SCORER]
    match = ('extent', 'sha256', 'relocations')
    if any(candidate[field] != reference[field] for field in match):
        save(HERE / 'historical-layout-rejected.json', {'historical': reference, 'candidate': candidate,
             'reason': 'Exact unlaid scorer bytes or relocations differ.'})
        print('archived scorer layout needs a new same-code profile', candidate, reference, flush=True)
        return None
    image, sections, _ = macho.load(original)
    _, base, _, offset, _, _ = sections[macho.TEXT]
    start = offset + candidate['start'] - base
    literals = []
    for pc in range(0, max(report['order']) + 4, 4):
        word, = unpack_from('<I', image, start + pc)
        if word & 0x3B00_0000 == 0x1800_0000:
            displacement = (word >> 5) & 0x7FFFF
            if displacement & 0x40000:
                displacement -= 0x80000
            literals.append(pc + 4 * displacement)
    assert literals
    pool = min(literals)
    assert pool % 16 == 0 and max(report['order']) < pool <= max(literals) < candidate['extent']
    path = HERE / 'scorer-block-profile.json'
    save(path, {'order': report['order'], 'literal_offset': pool})
    save(HERE / 'layout-proof.json', {'unlaid_scorer': candidate, 'archived_scorer': reference,
         'historical_report_sha256': sha(LAYOUT), 'profile_sha256': sha(path), 'constant_pool_targets': literals})
    return path


def emit(label, extra, compiler=None, profile_path=None, drop_flags=()):
    base = read(PLAN)['actors']['mech-verified-flowgraph']
    pins = base['artifact_sha256']
    assert sha(base['compile'][0]) == base['compiler_sha256']
    current = subprocess.check_output(['git','rev-parse','refs/heads/codex/native-compact-source-handoff'], cwd=ROOT)
    assert current.decode().strip() == PIN
    assert sha(base['compile'][1]) == '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
    path = HERE / label
    path.mkdir()
    flags = base['flags'].copy() | extra
    for key in drop_flags:
        flags.pop(key)
    if profile_path:
        key = 'MECH_COMPACT_PHASE_BLOCK_LAYOUT'
        flags[key] += f';{SCORER[1:]}@{profile_path}'
    assert all('LLVM' not in field for field in flags)
    source = list(base['compile'])
    if compiler:
        source[0] = str(compiler)
    source[2] = str(path / 'frontend')
    env = {key:value for key,value in os.environ.items() if not key.startswith('MECH_')}
    with (path / 'compile.log').open('wb') as log:
        subprocess.run(source, env=env | flags, stdout=log, stderr=subprocess.STDOUT, check=True)
    output = path / 'frontend'
    assert sha(output / 'program.mecb') == pins['bytecode']
    result = symbols(output / 'program.o')
    default = symbols(TEMPLATE / 'program.o')
    assert default['object_sha256'] == pins['encoder']
    updated = {name: info['extent'] for name, info in result['symbols'].items()
               if default['symbols'].get(name, {}).get('sha256') != info['sha256']}
    recorded = link.link_worker(TEMPLATE, output, path / 'bound')
    assert recorded['object_sha256'] == result['object_sha256']
    recorded.update(emit=source, flags=flags, emitter_sha256=sha(source[0]))
    save(path / 'bound/runtime.json', recorded)
    link.check_worker(path / 'bound', 'scorer-runtime-' + label)
    text = result['text_bytes']
    proof = {'source_pin': PIN, 'flags': flags, 'compile': source, 'compiler_sha256': sha(source[0]),
             'metadata_sha256': recorded['metadata_sha256'], 'worker_sha256': recorded['worker_sha256'],
             'bytecode_sha256': pins['bytecode'], 'text_bytes': text, 'exact_checks': 162,
             'text_change': text - default['text_bytes'], 'changed_symbols': updated, 'layout': result}
    save(path / 'receipt.json', proof)
    print(label, text, proof['text_change'], result['symbols'][SCORER]['extent'], updated, flush=True)


def main():
    for arm in sys.argv[1:]:
        if arm == 'horizontal':
            emit(arm, HORIZONTAL)
        elif arm == 'reverse-low-words':
            emit(arm, REVERSE)
        elif arm == 'horizontal-reverse':
            emit(arm, HORIZONTAL | REVERSE)
        elif arm == 'horizontal-layout-bound':
            path = profile()
            if path is not None:
                emit(arm, HORIZONTAL, profile_path=path)
        else:
            raise ValueError(arm)


if __name__ == '__main__':
    main()
