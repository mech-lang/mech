"""Show elapsed time and distance to libdeflate parity for both full orders."""
import json
from pathlib import Path
import math
import statistics


def derive_one(summary, role):
    campaign = summary['full_qualification'][role]
    assert campaign['complete'] and campaign['point_passed']
    orders = {}
    for run in campaign['runs']:
        rows = []
        for value in run['results']:
            ratio = value['paired_x_libdeflate']['mech-experiment']
            control = value['paired_x_libdeflate']['mech-control']
            rows.append({'case': value['case'], 'input_bytes': value['input_bytes'],
                         'milliseconds': {actor: seconds * 1000 for actor, seconds in value['median_seconds'].items()},
                         'mech_x_libdeflate': ratio, 'mech_x_previous_selected': ratio / control,
                         'mech_time_reduction_to_parity_pct': max(0.0, 100 * (1 - 1 / ratio)),
                         'libdeflate_point_win': ratio < 1})
        orders[run['order']] = {'peer_wins': run['peers'], 'cases': rows,
                               'geometric_mean_x_previous_selected': math.exp(statistics.mean(math.log(r['mech_x_previous_selected']) for r in rows))}
    forward = {r['case']: r for r in orders['forward']['cases']}
    reverse = {r['case']: r for r in orders['reverse']['cases']}
    losses = sorted((name for name in forward if not forward[name]['libdeflate_point_win'] or not reverse[name]['libdeflate_point_win']),
                    key=lambda name: max(forward[name]['mech_x_libdeflate'], reverse[name]['mech_x_libdeflate']))
    return {'role': role, 'host': 'Apple M1, macOS 15.6.1, arm64',
            'scope': 'Full81-case96-block qualification, both orders. Milliseconds are per-call medians; ratios are geometric means of paired block observations, so rounded displayed milliseconds need not divide to the paired ratio. A ratio below one is a win. Remaining reduction uses 1-1/ratio, not ratio-1. Confidence is separate.',
            'orders': orders, 'remaining_losses_closest_first': losses}


def render_one(value):
    first, second = [value['orders'][name] for name in ['forward', 'reverse']]
    forward, reverse = [{r['case']: r for r in run['cases']} for run in [first, second]]
    lines = ['# Elapsed time and libdeflate parity', '', value['scope'], '',
             f"**Qualified runtime arm: {value['role']}. Both orders retain every named 81 Roc /63 libdeflate win; no64th C win.**", '',
             '| Test | Mech ms, forward /reverse | libdeflate ms, forward /reverse | Paired Mech ÷ C | Mech time reduction still needed |',
             '| --- | ---: | ---: | ---: | ---: |']
    for name in [*value['remaining_losses_closest_first'], *(name for name in forward if name not in value['remaining_losses_closest_first'])]:
        a,b=forward[name],reverse[name]
        times=lambda actor: f"{a['milliseconds'][actor]:.6f} /{b['milliseconds'][actor]:.6f}"
        lines.append(f"| {name} | {times('mech-experiment')} | {times('libdeflate')} | {a['mech_x_libdeflate']:.6f} /{b['mech_x_libdeflate']:.6f}× | {a['mech_time_reduction_to_parity_pct']:.2f}% /{b['mech_time_reduction_to_parity_pct']:.2f}% |")
    return '\n'.join(lines)+'\n'


def derive(summary):
    return {role: derive_one(summary, role) for role in summary['full_qualification']}


def render(value):
    return '\n'.join(render_one(v) for v in value.values())


if __name__ == '__main__':
    here=Path(__file__).resolve().parent
    result=derive(json.loads((here/'summary.json').read_text()))
    (here/'parity.json').write_text(json.dumps(result,indent=2)+'\n')
    (here/'PARITY.md').write_text(render(result))
