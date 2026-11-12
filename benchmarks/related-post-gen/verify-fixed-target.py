"""Verify the managed related-post merge guard against the pinned Rust example."""
from pathlib import Path
from statistics import median
import argparse
import json

MECH = 'mech-current-managed-guarded-u16'
RUST = 'rust-preinitialized'
OUTPUT = '470263449577b9b386d47cf81a5b7ed6ec4155eabc8bb31709f55e2e3634f6fc'
FIXTURE_SHA = '500530b81cb81c5b1a5e48aa30de6a89ba0a350460ad826c4c7bb023e6e46cb8'
RUST_COMMIT = 'fc2982b0c929b1b701209d401a1dc30c773ef4ab'
MECH_SOURCE_SHA = '6a3022a0385efd1bf3dcdc9a76409607d20237260b21bf6745416d84da3d027a'
RUST_SOURCE_SHA = 'dbfbb70ae223195f9d928c9833b4bd8ea18eddbf46da10a8b7ec2080c4c1052e'


def read(path):
    return json.loads(Path(path).read_text())


def verify_cohort(path, rounds, pins, first_order):
    report = read(path)
    assert report['reference'] == RUST
    assert report['schema'] == 'mech.related-post-gen.implementations-comparison.v1'
    assert report['primary_timing_metric'] in ('post_json', 'whole_process')
    assert isinstance(report['summary'], dict)
    assert report['input']['sha256'] == pins['fixture_sha256'] and report['input']['posts'] == 5000
    workers = {row['name']: row for row in report['implementations']}
    assert set(workers) == {MECH, RUST} and len(report['implementations']) == 2
    for name in (MECH, RUST):
        assert workers[name]['executable_sha256'] == pins['binary_sha256'][name]
        assert workers[name]['source_sha256'] == pins['source_sha256'][name]
    assert workers[RUST]['calculation_boundary'] == 'preinitialized_workers'
    assert workers[MECH]['calculation_boundary'] == 'post_json_including_setup'
    groups = {index: {} for index in range(rounds)}
    for row in report['samples']:
        assert row['exit'] == 0 and row['exact_rust_output'] and row['output_sha256'] == OUTPUT
        sample, name = row['sample'], row['variant']
        assert name in workers and name not in groups[sample]
        groups[sample][name] = row
        if name == MECH:
            assert row['linked'] and row['posts'] == 5000
            assert row['work'] == 37025494493 and row['parallel_numeric_work'] == 24618013850
            assert row['parallel_batches'] == 1 and row['parallel_tasks'] == 4 and row['parallel_numeric_regions'] == 8
            assert 1 <= row['parallel_peak_workers'] <= 4
    assert len(report['samples']) == rounds * 2
    assert [list(group) for group in groups.values()] == report['execution_order']['rounds']
    assert report['execution_order']['rounds'] == [
        list(first_order if index % 2 == 0 else first_order[::-1]) for index in range(rounds)
    ]
    pair = [[group[MECH]['post_json_seconds'], group[RUST]['seconds']] for group in groups.values()]
    assert all(min(row) > 0 for row in pair)
    mech, rust = (median(column) for column in zip(*pair, strict=True))
    return {'rounds': rounds, 'exact_outputs': 2*rounds, 'mech_post_json_seconds': mech, 'rust_processing_seconds': rust,
            'mech_over_rust': mech/rust, 'paired_median': median(a/b for a, b in pair),
            'pairs_faster_than_rust': sum(a<b for a, b in pair), 'all_pairs_seconds': pair,
            'complete_process_seconds': {name: median(group[name]['wall_seconds'] for group in groups.values())
                                         for name in (MECH, RUST)}}


def replay_reports(primary, confirmation, pins):
    assert pins['fixture_sha256'] == FIXTURE_SHA
    assert pins['rust_commit'] == RUST_COMMIT and pins['source_sha256'][RUST] == RUST_SOURCE_SHA
    assert pins['source_sha256'][MECH] == MECH_SOURCE_SHA
    results = {
        'primary': verify_cohort(primary, 31, pins, (MECH, RUST)),
        'confirmation': verify_cohort(confirmation, 13, pins, (RUST, MECH)),
    }
    return {'pins': pins, 'cohorts': results,
            'passed_rust_parity': all(row['mech_over_rust'] <= 1 for row in results.values())}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Replay the fixed related-post benchmark without executing it.')
    parser.add_argument('--primary', type=Path, required=True)
    parser.add_argument('--confirmation', type=Path, required=True)
    parser.add_argument('--pins', type=Path, required=True)
    arguments = parser.parse_args()
    result = replay_reports(arguments.primary, arguments.confirmation, read(arguments.pins))
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result['passed_rust_parity'] else 1)
