"""Fresh input-v2 preparation; original sources/helpers/evidence stay immutable."""
import argparse
import copy
from contextlib import contextmanager
import fcntl
import json
import os
from pathlib import Path
import re
from unittest.mock import patch

import full_study
import prepare as base
import references
import dna_oracles
from prepare import ENVIRONMENT, NUMERIC_CASES, merge_pins, pins_digest, command_build

HERE = Path(__file__).resolve().parent
DRAFT_PATH = HERE / 'revisions/input-v2/profiles.json'
PLAN_PATH = HERE / 'revisions/input-v2/REVISION.md'
SOURCE_DIRECTORY = HERE / 'revisions/input-v2/sources'
ADDITIVE_PREREQUISITES = {'work-trace-fixtures.json':22, 'dna-oracle-fixtures.json':8,
                          'input-v2-fixtures.json':8}


def read_additive_prerequisites():
    result = {}
    for filename, count in ADDITIVE_PREREQUISITES.items():
        receipt = json.loads((HERE / filename).read_text())
        assert receipt['status'] == 'passed' and receipt['tests_run'] == count, filename
        result[filename] = receipt
    return result


def select_profiles(draft, benchmark=None):
    assert draft['report_only'] is True and draft['chart_admission'] is False
    rows = draft['profiles']
    assert len(rows) == 20
    assert len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'], row['variant']) for row in rows} == {
        (bench, variant) for bench in full_study.BENCHMARKS
        for variant in ('compact','optimized')
    }
    for row in rows:
        assert row['id'].endswith('-input-v2'), 'Fresh source revision needs a fresh profile key'
        source = (HERE / row['source']).resolve()
        assert source.is_relative_to(SOURCE_DIRECTORY.resolve()), 'Outside reviewed source revision'
        assert source.is_file()
    selected = rows if benchmark is None else [row for row in rows if row['benchmark'] == benchmark]
    assert benchmark is None or len(selected) == 2
    return selected


@contextmanager
def exact_oracle(benchmark):
    if benchmark == 'knucleotide':
        with patch.object(base.references, 'knucleotide', dna_oracles.knucleotide):
            yield
    else:
        yield


def probe(module, harness, row, *args, **kwargs):
    with exact_oracle(row['benchmark']):
        return base.probe(module, harness, row, *args, **kwargs)


def input_cases(benchmark):
    cases = base.input_cases(benchmark)
    if benchmark == 'knucleotide':
        return cases + (
            ('exact-token-tab-cr', b'>THREEBAD\rGGGG\r>THREE\tactual\rACGTGGTATT\r>FOUR\rTTTT'),
            ('comment-vertical-tab', b'>ONE\n;skip\v>THREE\nGGGG\n>THREE actual\nACGT\n'),
            ('complete-multirecord-beyond-buffer', b'>ONE\n' + b'ACGT'*7000 + b'\n>THREE actual\n' + b'ACGTGGTATT'*2000 + b'\n>FOUR\nAAAA\n'),
        )
    if benchmark == 'revcomp':
        return cases + (
            ('cr-only-record-comments', b'>one\rACGT\r;ignored\r>two\rMRWSYKVHDBN\r'),
            ('complete-multirecord-beyond-buffer', b'>one\n' + b'ACGTUMRWSYKVHDBN'*2200 + b'\n>two\nACGT\n'),
        )
    return cases + (('complete-multirecord-beyond-buffer', b'>one\n' + b'agggtaaatHaN'*2600 + b'\n>two\ntttaccct\n'),)


def prepare(args):
    module = full_study.configure(args.benchmark)
    directory = HERE / 'qualification' / args.revision / args.benchmark
    print('WAIT LOCK: V preparation', args.revision, args.benchmark, flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not directory.exists(), 'Existing preparation revision must be inspected, never overwritten'
        cohort_path = Path(args.expected_cohort).resolve()
        assert cohort_path == (HERE / 'qualification' / args.revision / 'cohort.json').resolve()
        cohort = json.loads(cohort_path.read_text())
        assert cohort['status'] == 'preparing'
        assert pins_digest(cohort['pins']) == args.cohort_pins_sha256, 'Reviewed cohort inputs changed'
        cohort_pins = cohort['pins']
        module.load_harness().PinGuard(cohort_pins).verify()
        bootstrap_path = HERE / 'bootstrap.json'
        fixture_path = HERE / 'oracle-fixtures.json'
        preparation_fixture_path = HERE / 'preparation-fixtures.json'
        cleanup_fixture_path = HERE / 'cleanup-fixtures.json'
        additive_paths = [HERE / name for name in ADDITIVE_PREREQUISITES]
        additive = read_additive_prerequisites()
        bootstrap = json.loads(bootstrap_path.read_text())
        fixtures = json.loads(fixture_path.read_text())
        preparation_fixtures = json.loads(preparation_fixture_path.read_text())
        cleanup_fixtures = json.loads(cleanup_fixture_path.read_text())
        assert bootstrap['status'] == 'prepared-smoke-passed'
        assert fixtures['status'] == 'passed' and fixtures['tests_run'] == 14
        assert preparation_fixtures['status'] == 'passed' and preparation_fixtures['tests_run'] == 5
        assert cleanup_fixtures['status'] == 'passed' and cleanup_fixtures['tests_run'] == 8
        draft_path = DRAFT_PATH
        draft = json.loads(draft_path.read_text())
        assert draft['report_only'] is True and draft['chart_admission'] is False
        originals = select_profiles(draft, args.benchmark)
        assert len(originals) == 2 and {row['variant'] for row in originals} == {'compact', 'optimized'}
        directory.mkdir(parents=True)
        module.OUT = directory
        module.REPORT = directory / 'probes.json'
        harness = module.load_harness()
        harness.install_interrupt_handlers()
        harness.compact.wait_for_environment(harness.REQUIRED)
        pins = merge_pins(bootstrap['pins'], fixtures['pins'], preparation_fixtures['pins'], cleanup_fixtures['pins'], cohort_pins, *(receipt['pins'] for receipt in additive.values()))
        paths = (bootstrap_path, fixture_path, preparation_fixture_path, cleanup_fixture_path, draft_path, PLAN_PATH,
                 Path(__file__).resolve(), full_study.SUITE / 'prepare.py',
                 full_study.CLBG / 'compact/run.py', full_study.CLBG / 'nbody/run.py',
                 full_study.CLBG / 'array-comparison/all-20261009/run.py',
                 HERE / 'prepare.py', HERE / 'dna_oracles.py',
                 HERE / 'test_input_v2.py') + tuple(additive_paths)
        pins = merge_pins(pins, {str(path): module.sha(path) for path in paths})
        harness.PinGuard(pins).verify()
        environment = dict(os.environ, **bootstrap['runtime_environment']) | ENVIRONMENT
        compiler = bootstrap['compiler']['path']
        profiles = []
        report = dict(status='preparing', created_at=module.now(), report_only=True,
                      never_chart_eligible=True, benchmark=args.benchmark,
                      revision=args.revision, attempts=[], active=None, profiles=[])
        module.save(module.REPORT, report)
        try:
            for original in originals:
                row = copy.deepcopy(original)
                assert row['algorithm']['strict'] is True
                assert row['algorithm']['native_benchmark_kernel'] is False
                profile_directory = directory / row['id']
                profile_directory.mkdir()
                source = (HERE / row['source']).resolve()
                assert source.is_relative_to(SOURCE_DIRECTORY.resolve())
                snapshot = profile_directory / 'program.v'
                snapshot.write_bytes(source.read_bytes())
                assert module.sha(snapshot) == module.sha(source)
                binary = profile_directory / 'program'
                command = [compiler] + row['compiler_flags'] + ['-o', str(binary), str(snapshot)]
                assert '-prod' in command and '-ffast-math' not in ' '.join(command)
                print('BUILD', row['id'], flush=True)
                command_build(module, harness, command, environment, profile_directory, lock.fileno())
                pins = merge_pins(pins, {str(path): module.sha(path) for path in (source, snapshot, binary)})
                row.update(
                    source=str(snapshot), original_source=str(source),
                    source_sha256=module.sha(snapshot), binary=str(binary),
                    binary_sha256=module.sha(binary), language='v',
                    key=args.benchmark + ':' + row['id'], cwd=str(HERE),
                    command=[str(binary)] + ([] if args.benchmark in harness.INPUTS else ['{n}']),
                    runtime_environment=ENVIRONMENT,
                    runtime=bootstrap['compiler'], build_command=command,
                    eligible_for_chart=False, never_chart_eligible=True,
                )
                profiles.append(row)
            manifest = dict(pins=pins, timeout_seconds=300, maximum_rss_mib=8192,
                            size=harness.WORK[args.benchmark][0])
            for row in profiles:
                checks = [probe(module, harness, row, 'canonical-small', manifest, report, lock.fileno())]
                cases = ((f'size-{n}', n, None) for n in NUMERIC_CASES[args.benchmark]) if args.benchmark in NUMERIC_CASES else ((name, None, data) for name, data in input_cases(args.benchmark))
                for name, n, data in cases:
                    print('PROBE', row['id'], name, flush=True)
                    checks.append(probe(module, harness, row, name, manifest, report, lock.fileno(), n=n, data=data))
                row.update(small_validation=checks[0]['validation'],
                           adversarial_validation=checks[1:],
                           build_status='passed', qualification_status='bounded-outputs-passed',
                           required_work_review=dict(status='pending independent work tracing/review'),
                           loc=None, loc_status='pending canonical vlang normalization')
            harness.PinGuard(pins).verify()
            report.update(status='bounded-outputs-passed', profiles=profiles,
                          pins=pins, pins_unchanged_at_end=True, full_performance_runs=0)
        except BaseException as error:
            report.update(status='stopped', error=str(error), error_type=type(error).__name__)
            raise
        finally:
            report.update(finished_at=module.now(), active=None)
            module.save(module.REPORT, report)
        print('PASS BOUNDED OUTPUTS', args.benchmark, len(report['attempts']), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--benchmark', choices=full_study.BENCHMARKS, required=True)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--expected-cohort', required=True)
    parser.add_argument('--cohort-pins-sha256', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision), 'Unsafe preparation revision name'
    assert re.fullmatch(r'[0-9a-f]{64}', args.cohort_pins_sha256)
    prepare(args)


if __name__ == '__main__':
    main()
