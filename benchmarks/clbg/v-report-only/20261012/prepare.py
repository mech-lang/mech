"""Build and qualify immutable V source snapshots under the timing mutex.

Preparation has its own evidence directory. Full timing manifests are sealed
separately after source-work, normalization and sampler review have passed.
"""
import argparse
import copy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

import full_study
import references

HERE = Path(__file__).resolve().parent
ENVIRONMENT = dict(LC_ALL='C', OMP_NUM_THREADS='8', OPENBLAS_NUM_THREADS='1',
                   VECLIB_MAXIMUM_THREADS='1', RAYON_NUM_THREADS='8')
NUMERIC_CASES = {
    'nbody': (0, 1, 2, 10000),
    'mandelbrot': (0, 1, 2, 3, 7, 8, 9, 17, 31, 65),
    'spectralnorm': (1, 2, 3, 5, 9, 100),
    'binarytrees': (0, 4, 7, 9),
    'fannkuchredux': (1, 2, 3, 4, 5, 6, 7, 8),
    'pidigits': (0, 1, 9, 10, 11, 20, 21, 27, 30, 97),
    'fasta': (0, 1, 2, 29, 30, 31, 60, 61, 287, 574, 20000),
}


def merge_pins(*groups):
    result = {}
    for group in groups:
        for path, digest in group.items():
            assert path not in result or result[path] == digest, ('Conflicting prerequisite pin', path)
            result[path] = digest
    return result


def pins_digest(pins):
    return hashlib.sha256(json.dumps(pins, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def input_cases(benchmark):
    if benchmark == 'knucleotide':
        return references.dna_fixtures() + (
            ('empty-comments', b'>THREE\n;\nACGT\n;\r\nGGTATT\n;'),
            ('no-final-newline', b'>THREE last\nggtattttaatttatagtacgt'),
            ('unterminated-target-header', b'>THREE'),
        )
    if benchmark == 'revcomp':
        return (
            ('all-iupac-empty', b'>one\nACGTUMRWSYKVHDBNacgtumrwsykvhdbn\n>empty\n>two\nAACCGGTT\n'),
            ('crlf-comment-eof', b'>one\r\naCgTu\r\n;>ignored\r\nMrWsYkVhDbN\r\n;ignored'),
            ('missing-final-newline', b'>one\nACGTUMRWSYKVHDBN'),
            ('long-odd-wrap', b'>one\n' + b'ACGTUMRWSYKVHDBN' * 13 + b'\n'),
            ('header-eof', b'>one'), ('empty-input', b''),
        )
    assert benchmark == 'regexredux'
    return (
        ('case-and-all-replacements', b'>one\nagggtaaatttaccct\nAGGGTAAA\ntHaNtaNDcaNHaDWaSaNaSaNBY<x>|hello|\n'),
        ('multiple-records', b'>one\nagggtaaa\n>two\ntttaccct\n'),
        ('unterminated-header', b'>one\nagggtaaa\n>unterminated'),
        ('replacement-boundaries', b'>one\ntHaNtHatHaTtaNDcaNHaDHaSWaSaNaSaNBY<<x>>|a||b|\n'),
        ('empty-input', b''),
    )


def command_build(module, harness, command, environment, directory, lock_fd):
    process = None
    detail = dict(command=command, started_at=module.now(), inherited_common_mutex=True)
    cleanup_error = None
    try:
        process = subprocess.Popen(
            command, cwd=HERE, env=environment, stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
            pass_fds=(lock_fd,),
        )
        detail.update(pid=process.pid)
        detail['identity'] = module.identity(process.pid)
        stdout, stderr = process.communicate(timeout=180)
        detail.update(returncode=process.returncode, finished_at=module.now())
        (directory / 'build.stdout').write_bytes(stdout)
        (directory / 'build.stderr').write_bytes(stderr)
        if harness.group_members(process.pid):
            raise RuntimeError('V compiler leader exited with a live child process')
        assert process.returncode == 0, ('V build failed; diagnostics retained', directory)
    except BaseException as error:
        detail.update(error=str(error), error_type=type(error).__name__)
        raise
    finally:
        if process is not None:
            try:
                harness.kill_group(process.pid)
            except BaseException as error:
                cleanup_error = error
                detail.update(cleanup_error=str(error), cleanup_error_type=type(error).__name__)
            finally:
                process.wait()
        module.save(directory / 'build.json', detail)
        if cleanup_error is not None:
            raise RuntimeError('V compiler cleanup failed; retained diagnostics') from cleanup_error


def output_validation(benchmark, expected, stream, harness):
    stream.seek(0)
    actual = stream.read()
    if benchmark == 'nbody':
        expected_values = [float(line) for line in expected.splitlines()]
        harness.compact.common.validate(actual.decode(), expected_values)
    else:
        assert actual == expected, ('V bounded output differs', benchmark,
                                   actual[:200], expected[:200])
    import hashlib
    return dict(bytes=len(actual), sha256=hashlib.sha256(actual).hexdigest(),
                reference_match=True, bounded_independent_oracle=True,
                tolerance=1e-8 if benchmark == 'nbody' else None)


def probe(module, harness, row, case, manifest, report, lock_fd, n=None, data=None):
    benchmark = row['benchmark']
    original_execute, original_validate = harness.execute, harness.validate
    original_work = harness.WORK[benchmark]
    original_inputs = harness.INPUTS.get(benchmark)
    n = original_work[0] if n is None else n
    local_manifest = dict(manifest, size=n)
    directory = module.OUT / row['id'] / case
    directory.mkdir(parents=True)
    data_path = None
    try:
        if data is not None:
            assert original_inputs is not None, 'Unexpected stdin benchmark'
            data_path = directory / 'stdin.fasta'
            data_path.write_bytes(data)
            harness.INPUTS[benchmark] = (data_path, original_inputs[1])
        harness.WORK[benchmark] = (n, original_work[1])
        expected = None if case == 'canonical-small' else getattr(references, benchmark)(data if data is not None else n)
        if expected is not None:
            (directory / 'expected.stdout').write_bytes(expected)

        def validate(bench, full, stream):
            assert bench == benchmark and full is False
            if expected is None:
                return original_validate(bench, False, stream)
            return output_validation(bench, expected, stream, harness)

        def execute(probe_row, full, timeout, maximum_rss_mib):
            assert probe_row['id'] == row['id']
            return original_execute(probe_row, False, timeout, maximum_rss_mib)

        harness.validate, harness.execute = validate, execute
        attempt = module.one_attempt(harness, row, local_manifest, report, 0, lock_fd)
        attempt.update(case=case, preparation_only=True, never_chart_eligible=True,
                       full_performance_run=False,
                       input_file=str(data_path) if data_path else None)
        report['attempts'].append(attempt)
        report['active'] = None
        module.save(module.REPORT, report)
        assert attempt['qualified'], ('V bounded probe failed; retained evidence', row['id'], case)
        assert attempt['full_workload'] is False
        assert attempt['supervised_native_group']['inherited_common_mutex'] is True
        return attempt
    finally:
        harness.execute, harness.validate = original_execute, original_validate
        harness.WORK[benchmark] = original_work
        if original_inputs is not None:
            harness.INPUTS[benchmark] = original_inputs


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
        bootstrap = json.loads(bootstrap_path.read_text())
        fixtures = json.loads(fixture_path.read_text())
        preparation_fixtures = json.loads(preparation_fixture_path.read_text())
        cleanup_fixtures = json.loads(cleanup_fixture_path.read_text())
        assert bootstrap['status'] == 'prepared-smoke-passed'
        assert fixtures['status'] == 'passed' and fixtures['tests_run'] == 14
        assert preparation_fixtures['status'] == 'passed' and preparation_fixtures['tests_run'] == 5
        assert cleanup_fixtures['status'] == 'passed' and cleanup_fixtures['tests_run'] == 8
        draft_path = HERE / 'profiles.json'
        draft = json.loads(draft_path.read_text())
        assert draft['report_only'] is True and draft['chart_admission'] is False
        originals = [row for row in draft['profiles'] if row['benchmark'] == args.benchmark and row['variant'] in ('compact', 'optimized')]
        assert len(originals) == 2 and {row['variant'] for row in originals} == {'compact', 'optimized'}
        directory.mkdir(parents=True)
        module.OUT = directory
        module.REPORT = directory / 'probes.json'
        harness = module.load_harness()
        harness.install_interrupt_handlers()
        harness.compact.wait_for_environment(harness.REQUIRED)
        pins = merge_pins(bootstrap['pins'], fixtures['pins'], preparation_fixtures['pins'], cleanup_fixtures['pins'], cohort_pins)
        paths = (bootstrap_path, fixture_path, preparation_fixture_path, cleanup_fixture_path, draft_path, HERE / 'SOURCE_PLAN.md',
                 Path(__file__).resolve(), full_study.SUITE / 'prepare.py',
                 full_study.CLBG / 'compact/run.py', full_study.CLBG / 'nbody/run.py',
                 full_study.CLBG / 'array-comparison/all-20261009/run.py')
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
                assert source.is_relative_to((HERE / 'sources').resolve())
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
