"""Count repeated address and lane preparation in the retained four-wide scorer."""

import json

from census import digest
from prepare import HERE

ROLE = 'ord-control'


def main():
    source = HERE / ROLE / 'instruction-counts/counts.json'
    observations = json.loads(source.read_bytes())
    output = {
        'object_sha256': observations['object_sha256'],
        'counts_sha256': digest(source),
        'measurement': 'exact executing block visits. Counts exclude synthetic instrumentation.',
        'cases': {},
    }
    for name, row in observations['cases'].items():
        group, = [block for block in row['blocks']
                  if {'uzp1.4s', 'rev64.4s', 'umin.4s'}.issubset({op for op, _ in block['instructions']})]
        instructions = group['instructions']
        full = row['counts']['instructions']
        loads = [i for i, (op, dest) in enumerate(instructions) if op in ('ldr', 'ldur') and dest.startswith('q')]
        assert len(loads) == 4
        first, second, third, literal = loads
        assert [instructions[i][0] for i in loads] == ['ldr', 'ldur', 'ldur', 'ldr']
        assert '[' in instructions[first][1] and '[' not in instructions[literal][1]
        assert third == second + 1
        addresses = [i for i in range(second) if i != first]
        assert all(instructions[i][0] in ('add', 'lsl', 'sub') for i in addresses)
        assert len(addresses) in (4, 5)
        c = group['visits']
        output['cases'][name] = {'groups': c, 'scorer_instructions': full,
            'group_instructions': len(instructions) * c, 'address_preparation_instructions': c * len(addresses),
            'lane_literal_loads': c, 'group_address_instruction_indices': addresses,
            'original_group': group}
    (HERE / 'vector-census.json').write_text(json.dumps(output, indent=2) + '\n')
    print({k: {label: v[label] for label in ('groups', 'address_preparation_instructions',
          'lane_literal_loads', 'scorer_instructions')} for k, v in output['cases'].items()})


if __name__ == '__main__':
    main()
