"""Derive both register profiles from the archived capture; no private paths needed."""
import argparse
import bisect
import copy
import struct
from hashlib import sha256
import json
from pathlib import Path
import re



def kernel_bytes(path, atom):
    data = path.read_bytes()
    assert struct.unpack_from('<I',data)[0] == 0xFEEDFACF
    at = 32
    for _ in range(struct.unpack_from('<I',data,16)[0]):
        kind,size = struct.unpack_from('<II',data,at)
        if kind==0x19:
            for index in range(struct.unpack_from('<I',data,at+64)[0]):
                section = at+72+80*index
                if data[section:section+16].rstrip(b'\0')==b'__text':
                    address,extent,offset = struct.unpack_from('<QQI',data,section+32)
                    begin = atom['start']-address
                    assert 0<=begin and begin+atom['extent']<=extent
                    code = data[offset+begin:offset+begin+atom['extent']]
                    assert sha256(code).hexdigest()==atom['sha256']
                    return code
        at += size
    raise AssertionError('missing native text')


def verify_binding(root, counts_path, receipt, counts):
    read = lambda path:json.loads(path.read_text())
    digest = lambda path:sha256(path.read_bytes()).hexdigest()
    proof = read(counts_path.with_name('source-binding.json'))
    def relative(value):
        path = Path(value)
        assert not path.is_absolute() and '..' not in path.parts
        return root/path
    source,canonical = relative(proof['source_object']),relative(proof['canonical_object'])
    assert digest(source)==proof['source_object_sha256']==receipt['artifacts']['program.o']
    assert digest(canonical)==proof['canonical_object_sha256']
    assert digest(counts_path)==proof['counts_sha256']
    assert counts['object_sha256']==proof['historical_counts_alias']
    original_path = counts_path.with_name('canonical-counts.json')
    assert digest(original_path)==proof['canonical_counts_sha256']==counts['canonical_counts_sha256']
    original = read(original_path)
    assert original['object_sha256']==proof['canonical_object_sha256']
    assert original['symbol']==counts['symbol']
    assert proof['source_atom']==receipt['layout']['symbols'][counts['symbol']]
    assert kernel_bytes(source,proof['source_atom'])==kernel_bytes(canonical,proof['canonical_atom'])
    assert proof['source_atom']['relocations']==proof['canonical_atom']['relocations']
    assert digest(relative(proof['source_worker']))==digest(relative(proof['canonical_worker']))==proof['worker_sha256']
    delta = proof['source_atom']['start']-proof['canonical_atom']['start']
    assert delta==proof['address_translation']==counts['address_translation']
    expected = copy.deepcopy(original['cases'])
    for sample in expected.values():
        for block in sample['blocks']:
            block['address']=hex(int(block['address'],16)+delta)
    assert expected==counts['cases']


def derive(root):
    read = lambda path:json.loads(path.read_text())
    digest = lambda path:sha256(path.read_bytes()).hexdigest()
    receipt = read(root/'receipt.json')
    symbol = 'mech_compact_numeric_475_vector'
    capture = root/(symbol+'.vcode')
    text = capture.read_text()
    expected = text.split(';; REGALLOC_INPUT_BEGIN\n',1)[1].split(';; REGALLOC_INPUT_END\n',1)[0]
    blocks = [(int(i),int(at)) for i,at in re.findall(r'^;; REGALLOC_BLOCK (\d+) offset=(\d+) ',text,re.M)]
    assert [i for i,_ in blocks] == list(range(len(blocks)))
    counts_path = root/'block-counts-475/counts.json'
    counts = read(counts_path)
    assert digest(root/'frontend/program.o') == receipt['artifacts']['program.o']
    if counts['object_sha256'] != receipt['artifacts']['program.o']:
        verify_binding(root,counts_path,receipt,counts)
    atom = receipt['layout']['symbols']['_'+symbol]
    assert not atom['relocations']
    weights = [0.0]*len(blocks)
    for case,sample in counts['cases'].items():
        visits = {int(b['address'],16)-atom['start']:b['visits'] for b in sample['blocks']}
        heads = sorted(visits)
        assert heads[0] == 0 and all(0 <= at < atom['extent'] for at in heads)
        mapped = [heads[bisect.bisect_right(heads,offset)-1] for _,offset in blocks]
        observed = [visits[at] for at in mapped]
        assert observed[0] == sample['calls']
        total = sum(visits.values())
        assert total > 0
        for index,value in enumerate(observed):
            weights[index] += value/total/len(counts['cases'])
    peak = max(weights)
    result = {}
    for mode,power in [('linear',1.0),('sqrt',0.5)]:
        result[mode] = {
            'expected_vcode':expected,
            'block_frequencies':[max(1,min(1<<20,round((value/peak)**power*4096))) for value in weights],
            'scope':'Equal case share of mapped kernel block visits across nine mixed cases, normalized to maximum4096, floor1. All virtual-code instructions, register types/constraints/clobbers must match exactly. '+mode,
            'power':power,
            'cases':list(counts['cases']),
            'training_object_sha256':receipt['artifacts']['program.o'],
            'training_vcode_sha256':digest(capture),
            'block_counts_sha256':digest(counts_path),
        }
    return result


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('capture',type=Path,help='Extracted study/training-native-v5 directory')
    parser.add_argument('output',type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=False)
    for name,profile in derive(args.capture).items():
        (args.output/(name+'.json')).write_text(json.dumps(profile,indent=2)+'\n')
    print('Derived linear and sqrt weights; shipping compilation validates full VCode identity.')


if __name__ == '__main__':
    main()
