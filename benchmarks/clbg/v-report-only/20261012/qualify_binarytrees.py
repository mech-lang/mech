"""Bounded real-node audit copies; all scored V sources remain unchanged."""
import argparse
import copy
import fcntl
import json
import os
from pathlib import Path
import re

import binarytrees_trace as trace
import full_study
import prepare

HERE = Path(__file__).resolve().parent


def qualify(args):
    module = full_study.configure('binarytrees')
    destination = HERE / 'work-qualification' / args.revision / 'binarytrees'
    print('WAIT common mutex: V real-node tree qualification', flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not destination.exists(), 'Never overwrite a work qualification'
        qualified_path = HERE / 'qualification' / args.source_revision / 'binarytrees/probes.json'
        qualified = json.loads(qualified_path.read_text())
        assert qualified['status'] == 'bounded-outputs-passed'
        assert qualified['pins_unchanged_at_end'] is True
        fixture_path = HERE / 'binarytrees-trace-fixtures.json'
        fixture = json.loads(fixture_path.read_text())
        assert fixture['status'] == 'passed' and fixture['tests_run'] == 8
        bootstrap = json.loads((HERE / 'bootstrap.json').read_text())
        assert bootstrap['status'] == 'prepared-smoke-passed'
        harness = module.load_harness()
        harness.install_interrupt_handlers()
        files = [Path(__file__).resolve(), qualified_path, fixture_path,
                 HERE / 'binarytrees_trace.py', HERE / 'BINARYTREES_TRACE_PLAN.md',
                 HERE / 'prepare.py', HERE / 'full_study.py', full_study.HELPER]
        pins = prepare.merge_pins(qualified['pins'], fixture['pins'],
                                 {str(path): module.sha(path) for path in files})
        harness.PinGuard(pins).verify()
        harness.compact.wait_for_environment(harness.REQUIRED)
        destination.mkdir(parents=True)
        module.OUT, module.REPORT = destination, destination / 'report.json'
        environment = dict(os.environ, **bootstrap['runtime_environment']) | prepare.ENVIRONMENT
        report = dict(status='qualifying', created_at=module.now(), benchmark='binarytrees',
            source_revision=args.source_revision, revision=args.revision,
            report_only=True, never_chart_eligible=True, full_performance_runs=0,
            attempts=[], active=None, work_reviews=[])
        module.save(module.REPORT, report)
        try:
            assert len(qualified['profiles']) == 2
            for original in qualified['profiles']:
                source_original = Path(original['source'])
                assert module.sha(source_original) == original['source_sha256']
                directory = destination / original['id']
                directory.mkdir()
                source = directory / 'tree-work.v'
                source.write_text(trace.instrument(source_original.read_text(), original['variant']))
                binary = directory / 'tree-work'
                command = [bootstrap['compiler']['path']] + original['compiler_flags'] + [
                    '-o', str(binary), str(source)]
                prepare.command_build(module, harness, command, environment, directory, lock.fileno())
                pins = prepare.merge_pins(pins, {str(path): module.sha(path) for path in (source, binary)})
                row = copy.deepcopy(original)
                row.update(id=original['id']+'-tree-work',
                    key='binarytrees:'+original['id']+'-tree-work',
                    source=str(source), source_sha256=module.sha(source),
                    binary=str(binary), binary_sha256=module.sha(binary),
                    command=[str(binary), '{n}'], never_chart_eligible=True,
                    eligible_for_chart=False, preparation_only=True)
                for size in trace.SIZES:
                    case = 'nodes-'+str(size)
                    case_directory = module.OUT / row['id'] / case
                    # prepare.probe creates its own directory. The observer
                    # directory is separate and already exists before launch.
                    graph_directory = directory / (case+'-graphs')
                    graph_directory.mkdir()
                    stdout_file = directory / (case+'.stdout')
                    row['runtime_environment'] = dict(prepare.ENVIRONMENT,
                        MECH_V_TREE_TRACE=str(graph_directory))
                    original_validation = prepare.output_validation

                    def capture(benchmark, expected, stream, loaded_harness):
                        result = original_validation(benchmark, expected, stream, loaded_harness)
                        stream.seek(0)
                        stdout_file.write_bytes(stream.read())
                        return result

                    try:
                        prepare.output_validation = capture
                        attempt = prepare.probe(module, harness, row, case,
                            dict(pins=pins, timeout_seconds=300, maximum_rss_mib=8192),
                            report, lock.fileno(), n=size)
                    finally:
                        prepare.output_validation = original_validation
                    try:
                        review = trace.reconcile(graph_directory, stdout_file.read_bytes(),
                                                 size, original['variant'], lock.fileno())
                        assert review['status'] == 'passed'
                    except BaseException as error:
                        attempt.update(qualified=False, output_qualified_before_source_work=True,
                            source_work_error=str(error), source_work_error_type=type(error).__name__)
                        module.save(case_directory / 'work-disposition.json', attempt)
                        module.save(module.REPORT, report)
                        raise
                    review.update(case=case, source_sha256=original['source_sha256'],
                        trace_source_sha256=module.sha(source), attempt=attempt['attempt'])
                    review_path = case_directory / 'work-review.json'
                    module.save(review_path, review)
                    report['work_reviews'].append(review)
                    observed_files = [stdout_file, review_path] + list(graph_directory.iterdir())
                    pins = prepare.merge_pins(pins,
                        {str(path): module.sha(path) for path in observed_files})
                    module.save(module.REPORT, report)
                assert module.sha(source_original) == original['source_sha256']
            harness.PinGuard(pins).verify()
            report.update(status='passed-real-node-and-retention-traces', pins=pins,
                pins_unchanged_at_end=True, measured_sources_unchanged=True,
                remaining_gates=['Generated C allocation lowering review', 'Full three-sample studies'])
        except BaseException as error:
            report.update(status='stopped-unqualified', error=str(error),
                error_type=type(error).__name__, no_automatic_retry=True)
            raise
        finally:
            report.update(active=None, finished_at=module.now())
            module.save(module.REPORT, report)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-revision', required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    for value in (args.source_revision, args.revision):
        assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', value)
    qualify(args)


if __name__ == '__main__':
    main()
