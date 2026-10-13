"""Candidate actual source-work qualification after exact C3 bounded qualification."""
import argparse
import copy
import fcntl
import json
from pathlib import Path
import re
import sys

import guarded_prepare as preparation
import references
import work_traces
import fixture_runner

HERE = preparation.HERE
GATES = {
    'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged',
}


def complete_attempt(attempt):
    assert attempt['qualified'] is True
    assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
    assert set(attempt['qualification_checks']) == GATES
    assert all(value is True for value in attempt['qualification_checks'].values())


def qualified_report(path, guard):
    report = json.loads(path.read_text())
    assert report['status'] == 'bounded-output-ownership-passed-awaiting-work-and-LOC-review'
    assert report['active'] is None and report['full_performance_runs'] == 0
    assert report['chart_admission'] is False and report['no_automatic_retry'] is True
    guard.verify(report['pins'])
    rows = {row['id']: row for row in report['profiles']}
    assert len(rows) == 20 and len(report['attempts']) == 154
    inventory = preparation.preflight_inputs(references)
    expected = {(row['id'], case) for row in rows.values() for case in inventory[row['benchmark']]}
    assert {(a['id'], a['case']) for a in report['attempts']} == expected
    for attempt in report['attempts']:
        complete_attempt(attempt)
        row = rows[attempt['id']]
        assert attempt['source_sha256'] == row['source_sha256']
        assert row['measured_architecture'] == 'arm64' and row['explicit_backend'] == 'llvm'
        assert row['explicit_target'] == 'macos-aarch64'
        assert row['compiler_version'] == '0.8.5' and row['bundled_llvm_version']
        assert preparation.sha(row['source']) == row['source_sha256']
        assert preparation.sha(row['command'][0]) == row['binary_sha256']
        assert all(preparation.sha(item['path']) == item['sha256'] for item in row['source_bundle'])
    return report


def compile_copy(guard, harness, qualification, row, source, label, output, lock_fd, pins, revision):
    directory = Path(qualification['build']) / 'source-work' / revision / label
    assert not directory.exists()
    directory.mkdir(parents=True)
    path = directory / 'observed.c3'
    path.write_text(source)
    binary = directory / 'observed'
    cache = directory / 'cache'
    sources = [path, *[Path(item['path']) for item in row['source_bundle'][1:]]]
    command = preparation.build_command(Path(qualification['compiler']), Path(qualification['stdlib']),
        Path(qualification['sdk']), sources, binary, cache)
    environment = qualification['runtime_environment']
    preparation.owned_command(guard, harness, command, environment, output / 'builds' / label,
                              lock_fd, 180, 'build')
    pins.update(preparation.merge(pins, preparation.pin_tree(directory)))
    pins.update(preparation.native_images(guard, harness, binary, environment,
        output / 'builds' / label / 'images', lock_fd,
        [Path(qualification['compiler']).parent, preparation.GMP, preparation.PCRE, preparation.CLANG.parent.parent]))
    observed = dict(row, id=row['id'] + '-work-trace', source=str(path), source_sha256=preparation.sha(path),
                    binary_sha256=preparation.sha(binary), command=[str(binary), '{n}'],
                    compiled_source_bundle=[str(p) for p in sources],
                    source_bundle=[dict(path=str(p), sha256=preparation.sha(p)) for p in sources],
                    preparation_only=True, never_chart_eligible=True)
    return observed


def probe_trace(guard, harness, row, n, data, expected, pins, output, lock_fd, report, case, reconcile):
    directory = output / row['id'] / case
    observed = copy.deepcopy(row)
    observed['runtime_environment']['CLBG_TRACE'] = str(directory / 'trace')
    attempt = preparation.probe(guard, harness, observed, n, data, expected, pins, output,
                                lock_fd, report, case, trace=reconcile)
    complete_attempt(attempt)
    return attempt


GMP_PRIMITIVE = r'''module clbg::gmp_abi;
import libc;
import clbg::gmp;
fn bool decimal(gmp::Mpz* value, char* expected)
{
    char[128] text;
    gmp::get_string(&text[0], 10, value);
    return libc::strcmp((ZString)&text[0], (ZString)expected) == 0;
}
fn int main()
{
    gmp::Mpz a;
    gmp::Mpz b;
    gmp::Mpz q;
    gmp::init(&a);
    gmp::init(&b);
    gmp::init(&q);
    gmp::set_ui(&a, 17);
    gmp::set_ui(&b, 5);
    gmp::divide(&q, &a, &b);
    if (gmp::Mpz::size != 16 || !decimal(&q, "3"))
    {
        return 2;
    }
    gmp::add(&q, &a, &b);
    if (!decimal(&q, "22"))
    {
        return 2;
    }
    gmp::set_ui(&q, 0);
    gmp::sub(&a, &q, &a);
    gmp::divide(&q, &a, &b);
    if (!decimal(&q, "-3"))
    {
        return 2;
    }
    if (libc::strcmp((ZString)gmp::version, "6.3.0"))
    {
        return 2;
    }
    gmp::clear(&a);
    gmp::clear(&b);
    gmp::clear(&q);
    libc::printf("GMP 6.3.0 ABI passed\n");
    return 0;
}
'''
PCRE_PRIMITIVE = r'''module clbg::pcre_abi;
import libc;
import clbg::pcre2;
fn int main()
{
    char[128] version;
    if (pcre2::config(11, &version[0]) < 0 || libc::strncmp(&version[0], "10.49", 5))
    {
        return 2;
    }
    int error = 0;
    usz error_offset = 0;
    void* code = pcre2::compile("a+", 2, 0, &error, &error_offset, null);
    if (!code)
    {
        return 2;
    }
    void* result = pcre2::match_data(code, null);
    if (!result)
    {
        return 2;
    }
    int status = pcre2::match(code, "baa", 3, 0, 0, result, null);
    usz* offsets = pcre2::offsets(result);
    if (status != 1 || offsets[0] != 1 || offsets[1] != 3)
    {
        return 2;
    }
    pcre2::free_match_data(result);
    pcre2::free_pattern(code);
    libc::printf("PCRE2 10.49 ABI passed\n");
    return 0;
}
'''


def run(args):
    print('WAIT common mutex: C3 actual source-work qualification', flush=True)
    with (HERE / 'source-work-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with preparation.LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            guard = preparation.load_guard()
            harness = guard.load_harness()
            harness.install_interrupt_handlers()
            prerequisite = HERE / 'qualification' / args.qualified_revision / 'report.json'
            qualification = qualified_report(prerequisite, guard)
            output = HERE / 'work-qualification' / args.revision
            assert not output.exists()
            output.mkdir(parents=True)
            guard.OUT = output
            guard.REPORT = output / 'report.json'
            pins = preparation.merge(qualification['pins'],
                {str(prerequisite): preparation.sha(prerequisite)},
                {str(p): preparation.sha(p) for p in (Path(__file__).resolve(), Path(work_traces.__file__).resolve(),
                                                    Path(__file__).with_name('test_work_traces.py'),
                                                    Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md'))})
            report = dict(status='preparing-source-work', prerequisite=str(prerequisite),
                prerequisite_sha256=preparation.sha(prerequisite), profiles=qualification['profiles'],
                qualification_revision=args.qualified_revision, work_revision=args.revision,
                attempts=[], active=None, full_performance_runs=0, chart_admission=False,
                no_automatic_retry=True, full_manifests_sealed=0, proofs=[], source_reviews=[])
            guard.save(guard.REPORT, report)
            try:
                fixture_environment = dict(qualification['runtime_environment'],
                                           PYTHONPATH=str(Path(__file__).resolve().parent))
                preparation.owned_command(guard, harness,
                    [sys.executable, '-B', str(Path(__file__).with_name('fixture_runner.py')),
                     '--set', 'work', '--receipt', str(output / 'fixtures/actual-inventory.json')], fixture_environment,
                    output / 'fixtures', lock.fileno(), 30, 'source-work-corruption-fixtures')
                fixture_receipt = output / 'fixtures/actual-inventory.json'
                fixture_runner.validate_receipt(json.loads(fixture_receipt.read_text()), 'work')
                pins.update(preparation.merge(pins, preparation.pin_tree(output / 'fixtures')))
                report['actual_mock_fixtures'] = dict(receipt=str(fixture_receipt),
                    sha256=preparation.sha(fixture_receipt), methods=17, zero_skips=True)
                for row in qualification['profiles']:
                    source = Path(row['source']).read_text()
                    benchmark = row['benchmark']
                    optimized = row['variant'] == 'optimized'
                    report['source_reviews'].append(dict(profile=row['id'], status='passed',
                        scored_source_sha256=row['source_sha256'], scored_binary_sha256=row['binary_sha256'],
                        compiled_source_bundle=row['source_bundle'], worker_cap=row['maximum_compute_workers'],
                        complete_bounded_cases=len([a for a in qualification['attempts'] if a['id'] == row['id']]),
                        review=str(Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md')))
                    if benchmark not in ('binarytrees', 'pidigits', 'knucleotide'):
                        continue
                    if benchmark == 'binarytrees':
                        llvm = work_traces.allocation_llvm(list((Path(row['cache']) / 'llvm').rglob('*.ll')))
                        native = preparation.owned_command(guard, harness,
                            ['/usr/bin/otool', '-tvV', row['command'][0]], qualification['runtime_environment'],
                            output / 'scored-allocation' / row['id'], lock.fileno(), 30, 'final-scored-arm')
                        allocation = work_traces.allocation_arm(native['stdout'].decode(), llvm['name'])
                        observed = compile_copy(guard, harness, qualification, row,
                            work_traces.tree_source(source, optimized), row['id'], output, lock.fileno(), pins, args.revision)
                        for n in (0, 4, 7, 9):
                            case = 'allocated-n' + str(n)
                            attempt = probe_trace(guard, harness, observed, n, None, references.binarytrees(n),
                                pins, output, lock.fileno(), report, case,
                                lambda directory, n=n: work_traces.tree_trace(directory, n, optimized))
                            report['proofs'].append(dict(kind='tree', profile=row['id'], case=case,
                                status='passed', attempt=attempt['attempt'], scored_source_sha256=row['source_sha256'],
                                scored_binary_sha256=row['binary_sha256'], trace_source_sha256=observed['source_sha256'],
                                allocation=allocation, llvm=llvm, reconciliation=attempt['source_work_proof']))
                    elif benchmark == 'pidigits':
                        observed = compile_copy(guard, harness, qualification, row,
                            work_traces.pi_source(source), row['id'], output, lock.fileno(), pins, args.revision)
                        for n in (0, 1, 9, 10, 11, 27, 97):
                            case = 'real-divisions-n' + str(n)
                            attempt = probe_trace(guard, harness, observed, n, None, references.pidigits(n),
                                pins, output, lock.fileno(), report, case,
                                lambda directory, n=n: work_traces.pi_trace(directory, n))
                            report['proofs'].append(dict(kind='pi', profile=row['id'], case=case,
                                status='passed', attempt=attempt['attempt'], scored_source_sha256=row['source_sha256'],
                                scored_binary_sha256=row['binary_sha256'], trace_source_sha256=observed['source_sha256'],
                                reconciliation=attempt['source_work_proof']))
                    else:
                        observed = compile_copy(guard, harness, qualification, row,
                            work_traces.map_source(source, optimized), row['id'], output, lock.fileno(), pins, args.revision)
                        for name, data in preparation.inputs('knucleotide', references):
                            case = 'complete-maps-' + name
                            attempt = probe_trace(guard, harness, observed, harness.WORK['knucleotide'][0], data,
                                references.knucleotide(data), pins, output, lock.fileno(), report, case,
                                lambda directory, data=data, name=name: work_traces.map_trace(
                                    directory, data, optimized, references, require_growth=name == 'arbitrary-widths'))
                            report['proofs'].append(dict(kind='maps', profile=row['id'], case=case,
                                status='passed', attempt=attempt['attempt'], scored_source_sha256=row['source_sha256'],
                                scored_binary_sha256=row['binary_sha256'], trace_source_sha256=observed['source_sha256'],
                                reconciliation=attempt['source_work_proof'],
                                reviewed_source_loop=dict(status='passed', source=row['source'],
                                    source_sha256=row['source_sha256'],
                                    method='Exact compact nested start/width loops' if not optimized else
                                           'Exact rolling index loop with width warmup; no per-window event trace')))
                for benchmark, source, expected in (
                    ('pidigits', GMP_PRIMITIVE, b'GMP 6.3.0 ABI passed\n'),
                    ('regexredux', PCRE_PRIMITIVE, b'PCRE2 10.49 ABI passed\n')):
                    base = next(row for row in qualification['profiles'] if row['benchmark'] == benchmark and row['variant'] == 'compact')
                    primitive = compile_copy(guard, harness, qualification, base, source,
                        benchmark + '-primitive-abi', output, lock.fileno(), pins, args.revision)
                    case = 'generic-abi'
                    attempt = preparation.probe(guard, harness, primitive, 0, None, expected,
                                               pins, output, lock.fileno(), report, case)
                    complete_attempt(attempt)
                    report['proofs'].append(dict(kind='primitive', profile=base['id'], case=case,
                        status='passed', attempt=attempt['attempt'], scored_source_sha256=base['source_sha256'],
                        scored_binary_sha256=base['binary_sha256'], trace_source_sha256=primitive['source_sha256']))
                assert len(report['proofs']) == len(report['attempts']) == 48
                assert len(report['source_reviews']) == 20
                # Native children, samplers and monitors have finished. Bind
                # every actual trace/stdout/raw/supervisor/image/LLVM/ARM proof
                # byte before terminal reporting; exclude only the mutable
                # top report itself to avoid a circular self-digest.
                evidence = {path: digest for path, digest in preparation.pin_tree(output).items()
                            if Path(path) != guard.REPORT}
                pins = preparation.merge(pins, evidence)
                guard.verify(pins)
                report.update(status='source-work-allocation-passed-awaiting-full-review-and-LOC',
                              finished_at=guard.now(), pins=pins, pins_unchanged_at_end=True)
                guard.save(guard.REPORT, report)
            except BaseException as error:
                report.update(status='stopped-unqualified', finished_at=guard.now(),
                              error=str(error) or type(error).__name__, error_type=type(error).__name__)
                guard.save(guard.REPORT, report)
                raise


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--qualified-revision', required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert all(value and value.replace('-', '').isalnum() for value in (args.qualified_revision, args.revision))
    run(args)


if __name__ == '__main__':
    main()
