"""Rank the remaining libdeflate losses on both full orders."""
from pathlib import Path

from emit import HERE, Q, read, save, sha

ROLE = 'caller-addresses-refined'
ORDERS = ('forward', 'reverse')


def main():
    observations = []
    for order in ORDERS:
        folder = Q / f'scorer-20261117-{ROLE}-full-{order}'
        terminal = read(folder / 'complete.json')
        assert terminal['cases'] == 81 and terminal['balanced_blocks_per_case'] == 96
        observations.append({row['case']: row for row in read(folder / 'summary.json')})
    assert len(observations[0]) == 81 and observations[0].keys() == observations[1].keys()
    missing = []
    for name in observations[0]:
        values = [by_case[name]['paired_x_libdeflate']['mech-experiment'] for by_case in observations]
        if max(values) >= 1.0:
            missing.append({'case': name, 'x_libdeflate': dict(zip(ORDERS, values)),
                            'worst_order_reduction_needed': 1 - 1 / max(values)})
    missing.sort(key=lambda row: max(row['x_libdeflate'].values()))
    target = 'prefix-natural-4096'
    trace_path = HERE / 'caller-addresses/instruction-counts/counts.json'
    trace = read(trace_path)
    data = trace['cases'][target]
    blocks = sorted(data['blocks'], key=lambda block: block['visits'] * len(block['instructions']),
                    reverse=True)
    assert missing[0]['case'] == 'prefix-natural-8192' and missing[1]['case'] == target
    assert data['counts']['instructions'] == 3472745 and blocks[0]['visits'] == 70480
    output = {'floor_libdeflate': 62, 'target_libdeflate': 63, 'floor_roc': 81,
              'order_ranked_remaining_losses': missing, 'attribution_case': target,
              'trace_sha256': sha(trace_path), 'instrumented_object_sha256': trace['object_sha256'],
              'scorer_calls': data['calls'], 'scorer_instructions': data['counts']['instructions'],
              'blocks_by_dynamic_instructions': blocks[:8],
              'estimated_scope': 'Dynamic scorer instructions only. Obtain exclusive time and a separate matched candidate.'}
    save(HERE / 'next-win.json', output)
    print(missing[:3], flush=True)


if __name__ == '__main__':
    main()
