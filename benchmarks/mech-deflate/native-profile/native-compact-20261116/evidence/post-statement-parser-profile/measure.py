"""Acquire serial balanced probe comparisons and a separate work census."""
from itertools import chain
import json
from statistics import median

from driver import digest, here, pin, write
import sample as previous

previous.work_fields += ('summary_applied', 'raised_peak_with_resource_envelope')


def main():
    binaries = {}
    for role in ('plain', 'time', 'census'):
        out = here / role
        binary = out / 'parser'
        record = json.loads((out / 'build.json').read_bytes())
        assert digest(binary) == record['binary']
        binaries[role] = binary
    assert len(set(map(digest, binaries.values()))) == 3
    orders = [('plain', 'time'), ('time', 'plain'), ('time', 'plain'), ('plain', 'time')]
    directory = here / 'observed'
    directory.mkdir()
    write(directory / 'plan.json', {'source_pin': pin, 'orders': orders, 'census_after_orders': True})
    pairs = [{role: previous.sample(binaries[role], directory, f'{index:02d}-{role}')
              for role in row} for index, row in enumerate(orders)]
    assert all('grammar' not in pair['plain'] and 'root_work' not in pair['plain'] for pair in pairs)
    census = previous.sample(binaries['census'], directory, 'census')
    families = census['counted_replays']
    covered = []
    replay = []
    for pair in pairs:
        timed = pair['time']
        assert set(timed['timed_replays']) == set(families)
        rows = list(timed['timed_replays'].values())
        exclusive = sum(row['exclusive_ns'] for row in rows)
        assert sum(row['outermost_ns'] for row in rows) == exclusive
        assert all(row['inclusive_union_ns'] >= max(row['exclusive_ns'], row['outermost_ns']) for row in rows)
        accounted = timed['grammar']['outside_ns'] + exclusive
        assert 0 < accounted <= timed['grammar']['window_ns']
        replay.append(exclusive)
        covered.append(accounted)
    for owner, counts in census['counted_trials'].items():
        assert sum(v['replays'] for k, v in families.items() if k.split('/')[0] == owner) == counts['with_replay']
    for counts in families.values():
        assert 0 <= counts['summary_applied'] <= counts['matched'] <= counts['replays']
    assert families['mech-code-alt/function-define']['summary_applied'] == 369
    assert families['statement/variable-define']['summary_applied'] > 0
    assert families['mech-code-alt/statement']['summary_applied'] == 117
    assert census['root_work']['fuel_remaining'] + census['observable']['parser_steps'] == 4_000_000
    cells = {
        name: {**counts, **{metric: median(pair['time']['timed_replays'][name][metric] for pair in pairs)
                           for metric in pairs[0]['time']['timed_replays'][name]}}
        for name, counts in families.items()
    }
    report = {
        'source_pin': pin,
        'median_plain_s': median(pair['plain']['observable']['parse_seconds'] for pair in pairs),
        'median_diagnostic_s': median(pair['time']['observable']['parse_seconds'] for pair in pairs),
        'paired_diagnostic_ratio': median(pair['time']['observable']['parse_seconds'] /
                                           pair['plain']['observable']['parse_seconds'] for pair in pairs),
        'median_grammar_ns': median(pair['time']['grammar']['window_ns'] for pair in pairs),
        'median_accounted_grammar_ns': median(covered), 'median_replay_ns': median(replay),
        'families': dict(sorted(cells.items(), key=lambda item: item[1]['exclusive_ns'], reverse=True)),
    }
    stage_names = {
        'root': ('initialize', 'grammar', 'finish_events', 'finish_snapshot'),
        'finish': ('sink', 'diagnostics', 'tree_index', 'restart_index'),
    }
    stages = []
    for log in sorted(directory.glob('*-time.log')):
        spans = {}
        for line in log.read_text().splitlines():
            if not line.startswith('post-statement-profile'):
                continue
            _, phase, *ns = line.split('\t')
            spans.update(zip(stage_names[phase], map(int, ns), strict=True))
        assert len(spans) == 8, log
        stages.append(spans)
    assert len(stages) == 4
    report['phase_median_ms'] = {phase: median(row[phase] for row in stages) / 1e6 for phase in stages[0]}
    write(directory / 'exact-runs.json', {'orders': orders, 'pairs': pairs, 'census': census, 'stage_timings_ns': stages})
    write(here / 'report.json', report)
    print({name: value for name, value in report.items() if name != 'families'})
    for name, row in report['families'].items():
        print(name, f"self={row['exclusive_ns'] / 1e6:.2f} ms", f"union={row['inclusive_union_ns'] / 1e6:.2f} ms",
              f"observed={row['replays']} applied={row['summary_applied']} shapes={row['provisional_candidates']}")


if __name__ == '__main__':
    main()
