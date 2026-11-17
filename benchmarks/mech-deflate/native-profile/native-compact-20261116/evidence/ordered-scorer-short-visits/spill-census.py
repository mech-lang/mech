"""Trace the repeated scorer reload to its input packet field."""

import json
from pathlib import Path
from hashlib import sha256

HERE = Path(__file__).resolve().parent
CASE = 'prefix-natural-8192'
FILES = {
    'latch-control': HERE / 'short-visit-latch-control/instruction-counts/counts.json',
    'vector-12': HERE / 'vector-12-layout/instruction-counts/counts.json',
    'paired-scalar-unplaced': HERE / 'paired-scalar/instruction-counts/counts.json',
}


def main():
    summary = {}
    for role, path in FILES.items():
        data = path.read_bytes()
        raw = json.loads(data)
        case = raw['cases'][CASE]
        summary[role] = {'counts_sha256': sha256(data).hexdigest(), 'object_sha256': raw['object_sha256'],
                         'calls': case['calls'], 'stack_loads': case['counts']['stack_loads']}
        if role == 'paired-scalar-unplaced':
            continue
        store = 'x8, [sp, #0x70]'
        accesses = [block for block in case['blocks'] if any(args == store for _,args in block['instructions'])]
        reload, = [block for block in accesses if ['ldur', store] in block['instructions']]
        stores = [block for block in accesses if ['stur', store] in block['instructions']]
        assert reload['visits'] == 43020
        assert len(stores) == 2
        for block in stores:
            instructions = block['instructions']
            load = instructions.index(['ldr', 'x8, [x1, #0x58]'])
            spill = instructions.index(['stur', store])
            assert load < spill
        assert ['cmp', 'x7, x8'] in reload['instructions']
        summary[role].update(reload=reload, stores=stores)
    (HERE / 'spill-census.json').write_text(json.dumps({'case': CASE, 'workers': summary}, indent=2) + '\n')
    print({k:{x:y for x,y in v.items() if x not in ('reload','stores')} for k,v in summary.items()})


if __name__ == '__main__':
    main()
