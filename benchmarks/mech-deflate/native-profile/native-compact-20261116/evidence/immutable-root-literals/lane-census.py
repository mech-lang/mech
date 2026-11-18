"""Count scalar source literals in the largest checked root lane census."""

import argparse
from hashlib import sha256
import json
from pathlib import Path


def observations(cfg, emission):
    values = {}
    for row in cfg['values']:
        for slot in range(row['base'], row['base'] + row['words']):
            assert slot not in values
            values[slot] = row
    definitions = {value for site in cfg['sites'] for value in site['defs']}
    root, = [bank for bank in emission['measurements']['word_banks'] if bank['entry'] == 0]
    assert len(root['data']) == root['count']
    assert len({row['slot'] for row in root['data']}) == root['count']
    assert all(row['slot'] in values for row in root['data'])
    selected = []
    for row in root['data']:
        declared = values[row['slot']]
        if (row['origin'] == 'constant' and declared['value'] not in definitions
                and 'scalar_bytes' in declared['type']['kind'] and not declared['owners']):
            selected.append(row)
    return {'lanes': len(selected), **{key: sum(row[key] for row in selected)
                                     for key in ('reads', 'writes', 'parameter_reads')}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_cfg', type=Path)
    parser.add_argument('emission_probe', type=Path)
    args = parser.parse_args()
    cfg, emission = args.source_cfg.read_bytes(), args.emission_probe.read_bytes()
    result = {'sources': {'source_cfg': {'path': str(args.source_cfg), 'sha256': sha256(cfg).hexdigest()},
                          'emission_probe': {'path': str(args.emission_probe), 'sha256': sha256(emission).hexdigest()}},
              'scope': 'Declared scalar entry constants absent from the source definitions, matched with the exact emitted-lane census.',
              'counts': observations(json.loads(cfg), json.loads(emission))}
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
