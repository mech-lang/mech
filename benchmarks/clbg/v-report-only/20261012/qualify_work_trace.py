"""Build and reconcile bounded unscored V map and pi traces.

Every copy is separate from its qualified application. Native compilation and
execution use the same owned-process APIs and mutex as the existing probes.
No measured source, scored record or visualization is written.
"""
import argparse
import copy
import fcntl
import json
import os
from pathlib import Path
import re
from contextlib import nullcontext
from unittest.mock import patch

import dna_oracles
import full_study
import prepare
import source_trace

HERE = Path(__file__).resolve().parent
PREREQUISITES = {'oracle-fixtures.json': 14, 'preparation-fixtures.json': 5,
                 'cleanup-fixtures.json': 8, 'work-trace-fixtures.json': 22,
                 'dna-oracle-fixtures.json': 8}


def qualify(args):
    module = full_study.configure(args.benchmark)
    destination = HERE / 'work-qualification' / args.revision / args.benchmark
    print('WAIT common mutex: bounded V work traces', args.benchmark, flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not destination.exists(), 'No overwrite or implicit trace recovery'
        harness = module.load_harness()
        harness.install_interrupt_handlers()
        qualification_path = HERE / 'qualification' / args.source_revision / args.benchmark / 'probes.json'
        qualified = json.loads(qualification_path.read_text())
        assert qualified['status'] == 'bounded-outputs-passed'
        assert qualified['benchmark'] == args.benchmark and qualified['pins_unchanged_at_end'] is True
        prerequisites = {}
        for filename, count in PREREQUISITES.items():
            receipt = json.loads((HERE / filename).read_text())
            assert receipt['status'] == 'passed' and receipt['tests_run'] == count
            prerequisites[filename] = receipt
        pins = prepare.merge_pins(qualified['pins'], *(item['pins'] for item in prerequisites.values()))
        paths = [Path(__file__).resolve(), qualification_path, HERE / 'source_trace.py',
                 HERE / 'prepare.py', HERE / 'full_study.py', HERE / 'dna_oracles.py', full_study.HELPER]
        paths += [HERE / filename for filename in PREREQUISITES]
        pins = prepare.merge_pins(pins, {str(path): module.sha(path) for path in paths})
        harness.PinGuard(pins).verify()
        bootstrap = json.loads((HERE / 'bootstrap.json').read_text())
        assert bootstrap['status'] == 'prepared-smoke-passed'
        environment = dict(os.environ, **bootstrap['runtime_environment']) | prepare.ENVIRONMENT
        destination.mkdir(parents=True)
        module.OUT, module.REPORT = destination, destination / 'report.json'
        report = dict(status='qualifying', created_at=module.now(), report_only=True,
                      never_chart_eligible=True, full_performance_runs=0, benchmark=args.benchmark,
                      source_revision=args.source_revision, revision=args.revision,
                      qualification=str(qualification_path), attempts=[], active=None, work_reviews=[])
        module.save(module.REPORT, report)
        try:
            harness.compact.wait_for_environment(harness.REQUIRED)
            assert len(qualified['profiles']) == 2
            for original in qualified['profiles']:
                assert original['variant'] in ('compact', 'optimized')
                scored_source = Path(original['source'])
                assert module.sha(scored_source) == original['source_sha256']
                directory = destination / original['id']
                directory.mkdir()
                text, edits = source_trace.instrument_text(scored_source.read_text(), args.benchmark, original['variant'])
                source = directory / 'work-trace.v'
                source.write_text(text)
                module.save(directory / 'edits.json', dict(edits, scored_source=str(scored_source),
                                                           scored_source_sha256=original['source_sha256']))
                binary = directory / 'work-trace'
                command = [bootstrap['compiler']['path']] + original['compiler_flags'] + ['-o', str(binary), str(source)]
                prepare.command_build(module, harness, command, environment, directory, lock.fileno())
                pins = prepare.merge_pins(pins, {str(path): module.sha(path) for path in (source, binary, directory / 'edits.json')})
                row = copy.deepcopy(original)
                row.update(id=original['id'] + '-work-trace', key=args.benchmark + ':' + original['id'] + '-work-trace',
                           source=str(source), source_sha256=module.sha(source), binary=str(binary),
                           binary_sha256=module.sha(binary), command=[str(binary)] + ([] if args.benchmark == 'knucleotide' else ['{n}']),
                           eligible_for_chart=False, never_chart_eligible=True, preparation_only=True)
                cases = [('digits-97', 97, None), ('zero-digits', 0, None)] if args.benchmark == 'pidigits' else [
                    (name, None, data) for name, data in prepare.input_cases('knucleotide')]
                if args.benchmark == 'knucleotide':
                    cases += [
                        ('lookalike-target', None, b'>THREEBAD\nCCCC\n>THREE target\nACGTGGTATTTTAATTTATAGT\n'),
                        ('exact-token-and-cr', None, b'>THREE\vnot-token\rCCCC\r>THREE\tvalid\rACGT\r\nGGTATT\n')]
                for name, size, data in cases:
                    case_directory = module.OUT / row['id'] / name
                    trace_file = case_directory / 'actual.trace'
                    stdout_file = case_directory / 'actual.stdout'
                    row['runtime_environment'] = dict(prepare.ENVIRONMENT, MECH_V_WORK_TRACE=str(trace_file))
                    original_validation = prepare.output_validation

                    def capture(benchmark, expected, stream, loaded_harness):
                        result = original_validation(benchmark, expected, stream, loaded_harness)
                        stream.seek(0)
                        stdout_file.write_bytes(stream.read())
                        return result

                    try:
                        prepare.output_validation = capture
                        manifest = dict(pins=pins, timeout_seconds=300, maximum_rss_mib=8192,
                                        size=harness.WORK[args.benchmark][0])
                        oracle_scope = patch.object(prepare.references, 'knucleotide', dna_oracles.knucleotide) if args.benchmark == 'knucleotide' else nullcontext()
                        with oracle_scope:
                            attempt = prepare.probe(module, harness, row, name, manifest, report,
                                                    lock.fileno(), n=size, data=data)
                    finally:
                        prepare.output_validation = original_validation
                    try:
                        assert trace_file.is_file() and stdout_file.is_file()
                        if args.benchmark == 'pidigits':
                            reconciled = source_trace.reconcile_pi(trace_file.read_bytes(), stdout_file.read_bytes(),
                                size, original['variant'], common_timing_mutex_fd=lock.fileno())
                        else:
                            reconciled = source_trace.reconcile_maps(trace_file.read_bytes(), data, stdout_file.read_bytes(),
                                original['variant'], common_timing_mutex_fd=lock.fileno())
                        assert reconciled['status'] == 'passed'
                    except BaseException as error:
                        # Keep the sampler's original output evidence intact;
                        # the separate parent work disposition is unqualified.
                        attempt.update(output_qualified_before_source_work=True,
                                       qualified=False, source_work_error=str(error),
                                       source_work_error_type=type(error).__name__)
                        module.save(case_directory / 'work-disposition.json',
                                    dict(attempt, never_chart_eligible=True))
                        module.save(module.REPORT, report)
                        raise
                    reconciled.update(case=name, scored_source_sha256=original['source_sha256'],
                                      trace_source_sha256=module.sha(source), attempt=attempt['attempt'])
                    module.save(case_directory / 'work-review.json', reconciled)
                    report['work_reviews'].append(reconciled)
                    pins = prepare.merge_pins(pins, {str(path): module.sha(path) for path in (
                        trace_file, stdout_file, case_directory / 'expected.stdout', case_directory / 'work-review.json')})
                    if data is not None:
                        input_path = case_directory / 'stdin.fasta'
                        pins = prepare.merge_pins(pins, {str(input_path): module.sha(input_path)})
                    module.save(module.REPORT, report)
                assert module.sha(scored_source) == original['source_sha256']
            harness.PinGuard(pins).verify()
            report.update(status='passed', pins=pins, pins_unchanged_at_end=True,
                          measured_source_bytes_unchanged=True, original_stdout_unchanged=True)
        except BaseException as error:
            report.update(status='stopped', error=str(error), error_type=type(error).__name__, no_automatic_retry=True)
            raise
        finally:
            report.update(finished_at=module.now(), active=None)
            module.save(module.REPORT, report)
        print('PASS bounded V work traces', args.benchmark, len(report['work_reviews']), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--benchmark', choices=('pidigits', 'knucleotide'), required=True)
    parser.add_argument('--source-revision', required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    for name in (args.source_revision, args.revision):
        assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', name)
    qualify(args)


if __name__ == '__main__':
    main()
