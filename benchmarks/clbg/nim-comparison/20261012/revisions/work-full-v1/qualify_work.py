"""Add live node/leaf/retention and final scored-binary allocation proofs."""
import argparse
import copy
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import signal
import sys

import tree_work

HERE = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HERE))
import guarded_prepare_v5 as preparation
import proof_reuse_v5
import references


def qualified_report(path, guard):
    report = json.loads(path.read_text())
    assert report['status'] == 'bounded-output-ownership-passed-awaiting-work-and-LOC-review'
    assert report['active'] is None and report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert report['fresh_profile_builds'] == 0 and report['reused_bounded_cases'] == 154
    assert report['reused_cases_relabelled_as_attempts'] is False
    guard.verify(report['pins'])
    reuse_path = Path(report['prior_bounded_proof_reuse'])
    assert preparation.sha(reuse_path) == report['prior_bounded_proof_reuse_sha256']
    reuse = json.loads(reuse_path.read_text())
    proof_reuse_v5.validate_prior_reports(reuse)
    guard.verify(reuse['pins'])
    assert len(report['profiles']) == 20 and {row['id'] for row in report['profiles']} == proof_reuse_v5.PROFILES
    expected = {('pure-bigint-abi', None), ('gmp-abi', None), ('pcre-abi', None), ('orc-join-parent-destruction', None)}
    expected |= {('actual-seven-map-' + variant, case) for variant in ('compact', 'optimized')
                 for case in ('empty', 'tail-comments-crlf', 'growing')}
    expected |= {('actual-pi-' + variant, 'n-' + str(n)) for variant in ('compact', 'optimized')
                 for n in (0, 1, 9, 10, 11, 27, 97)}
    assert {(item['proof'], item.get('case')) for item in report['proofs']} == expected
    assert len(report['proofs']) == len(expected)
    assert len(report['attempts']) == len(expected)
    for item in report['proofs']:
        assert item['status'] == 'passed'
        if item['proof'].startswith('actual-'):
            assert item['reconciliation']['status'] == 'passed'
        if item['proof'].startswith('actual-seven-map') or item['proof'] == 'orc-join-parent-destruction':
            assert item['asan'] is True
    for attempt in report['attempts']:
        assert attempt['qualified'] is True
        assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
        assert set(attempt['qualification_checks']) == proof_reuse_v5.old.EXPECTED_GATES
        assert all(value is True for value in attempt['qualification_checks'].values())
    for row in report['profiles']:
        assert preparation.sha(row['source']) == row['source_sha256']
        assert preparation.sha(row['command'][0]) == row['binary_sha256']
        assert preparation.sha(HERE / 'sources' / row['benchmark'] / (row['variant'] + '.nim')) == row['source_sha256']
        assert row['maximum_compute_workers'] <= 8
    return report, reuse


def qualify(args):
    print('WAIT common mutex: Nim actual allocation/retention proofs', flush=True)
    with preparation.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        guard = preparation.load_guard()
        harness = guard.load_harness()
        harness.install_interrupt_handlers()
        prerequisite = HERE / 'qualification' / args.qualified_revision / 'report.json'
        qualified, reuse = qualified_report(prerequisite, guard)
        paths = [Path(__file__).resolve(), Path(tree_work.__file__).resolve(),
            Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md'), prerequisite,
            Path(__file__).with_name('test_tree_work.py'), Path(__file__).with_name('full_study.py'),
            Path(__file__).with_name('test_full_study.py')]
        pins = preparation.merge(qualified['pins'], reuse['pins'], {str(p): preparation.sha(p) for p in paths})
        output = HERE / 'work-qualification' / args.revision
        assert not output.exists(), 'No overwritten qualification, implicit resume or rerun'
        output.mkdir(parents=True)
        guard.OUT, guard.REPORT = output, output / 'report.json'
        report = dict(status='qualifying', started_at=guard.now(), attempts=[], active=None,
            prerequisite=str(prerequisite), prerequisite_sha256=preparation.sha(prerequisite),
            full_performance_runs=0, chart_admission=False, no_automatic_retry=True,
            qualification_revision=args.qualified_revision, source_reviews=[], tree_proofs=[])
        guard.save(guard.REPORT, report)
        try:
            # Run this revision's fixture module from its own directory. The
            # common owner uses an absolute path and preserves the mutex FD.
            preparation.owned_command(guard, harness,
                [sys.executable, '-B', str(Path(__file__).with_name('test_tree_work.py'))], dict(os.environ),
                output / 'fixtures', lock.fileno(), 60, 'tree-fixtures')
            preparation.owned_command(guard, harness,
                [sys.executable, '-B', str(Path(__file__).with_name('test_full_study.py'))], dict(os.environ),
                output / 'fixtures', lock.fileno(), 60, 'full-controller-fixtures')
            build = HERE / '.build' / args.qualified_revision
            compiler = build / 'nim/nim-2.2.12/bin/nim'
            toolchain = compiler.parent.parent
            bigint = build / 'bigints/bigints-3b5a106b88ec1baa904a059a1430c8335d885075'
            wrapper = build / 'nim-gmp/nim-gmp-4e9a6a05bfc274abd02a241f2d7c9162b2300191'
            proposal = json.loads((HERE / 'proposals' / (args.qualified_revision + '.json')).read_text())
            sdk = Path(proposal['sdk'])
            environment = dict(os.environ, LC_ALL='C', SDKROOT=str(sdk), DYLD_LIBRARY_PATH=preparation.LIBRARY_PATH,
                NIMBLE_DIR=str(build / 'nimble-unused'), OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
            for key in ('NIM_CONFIG_PATH', 'NIM_LIB_PREFIX', 'NIMBLE_PATH', 'NIMFLAGS', 'CFLAGS', 'CPPFLAGS', 'LDFLAGS',
                        'DYLD_INSERT_LIBRARIES', 'DYLD_FRAMEWORK_PATH', 'DYLD_FALLBACK_LIBRARY_PATH'):
                environment.pop(key, None)
            for row in qualified['profiles']:
                source = Path(row['source'])
                text = source.read_text()
                assert '\t' not in text and row['maximum_compute_workers'] <= 8
                report['source_reviews'].append(dict(profile=row['id'], status='passed',
                    scored_source_sha256=row['source_sha256'], complete_bounded_cases=len(row['prior_bounded_records']),
                    worker_cap=row['maximum_compute_workers'], review=str(Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md')),
                    counted_application_helpers='All helper definitions in the application source; generic GMP ABI adapter audited separately'))
            for original in [row for row in qualified['profiles'] if row['benchmark'] == 'binarytrees']:
                variant = original['variant']
                directory = output / original['id']
                directory.mkdir()
                source = directory / (variant + '.nim')
                text, edits = tree_work.instrument(Path(original['source']).read_text(), variant)
                source.write_text(text)
                support = directory / 'audit_support.nim'
                support.write_text(tree_work.SUPPORT)
                edits_path = directory / 'edits.json'
                preparation.new_json(edits_path, edits)
                binary = directory / 'actual-tree-work'
                cache = directory / 'nimcache'
                command = preparation.build_command(compiler, toolchain, bigint, wrapper, sdk, source, binary, cache)
                preparation.owned_command(guard, harness, command, environment, directory / 'build', lock.fileno(), 180, 'build')
                pins = preparation.merge(pins, preparation.pin_tree(cache),
                    {str(p): preparation.sha(p) for p in (source, support, binary, edits_path)})
                pins = preparation.merge(pins, preparation.native_images(guard, harness, binary, environment,
                    directory / 'images', lock.fileno(), [build / 'nim', preparation.GMP, preparation.PCRE, preparation.CLANG.parent.parent]))
                selected = reuse['profiles'][original['id']]
                scored_cache = Path(selected['binary']).parent.parent / 'cache' / original['id']
                generated = scored_cache / ('@m' + variant + '.nim.c')
                name, creator = tree_work.creator_c(generated)
                creator_fragment = directory / 'scored-creator.c'
                creator_fragment.write_text(creator)
                disassembly = preparation.owned_command(guard, harness, ['/usr/bin/otool', '-tvV', selected['binary']],
                    environment, directory / 'scored-binary', lock.fileno(), 60, 'disassembly')['stdout'].decode()
                arm = tree_work.creator_arm(disassembly, name)
                arm_path = directory / 'scored-allocation-review.json'
                preparation.new_json(arm_path, dict(arm, scored_source_sha256=original['source_sha256'],
                    scored_binary_sha256=original['binary_sha256'], generated_c=str(generated), generated_c_sha256=preparation.sha(generated),
                    unconditional_leaf_allocation_in_generated_C=True))
                pins = preparation.merge(pins, {str(p): preparation.sha(p) for p in (generated, creator_fragment, arm_path)},
                    preparation.pin_tree(directory / 'scored-binary'))
                row = copy.deepcopy(original)
                row.update(id=original['id'] + '-work-trace', key='binarytrees:' + original['id'] + '-work-trace',
                    source=str(source), source_sha256=preparation.sha(source), binary_sha256=preparation.sha(binary),
                    command=[str(binary), '{n}'], runtime_environment=dict(original['runtime_environment']))
                for n in (0, 4, 7, 9):
                    case = 'allocated-n' + str(n)
                    trace = output / row['id'] / case / 'trace'
                    assert not trace.exists(), 'Native audit startup creates its own new trace directory'
                    row['runtime_environment']['MECH_NIM_TREE_TRACE'] = str(trace)
                    attempt = preparation.probe(guard, harness, row, n, None, references.binarytrees(n), pins,
                        output, lock.fileno(), report, case, trace=lambda path, trace=trace, n=n, variant=variant:
                            tree_work.reconcile(trace, n, variant))
                    pins = preparation.merge(pins, attempt['source_work_proof']['trace_pins'])
                    report['tree_proofs'].append(dict(status='passed', profile=original['id'], case=case,
                        scored_source_sha256=original['source_sha256'], scored_binary_sha256=original['binary_sha256'],
                        trace_source_sha256=row['source_sha256'], attempt=attempt['attempt'],
                        allocation=arm, reconciliation=attempt['source_work_proof'],
                        no_scored_timing_or_GC_pressure_claim=True))
                    guard.save(guard.REPORT, report)
            assert len(report['tree_proofs']) == len(report['attempts']) == 8
            assert len(report['source_reviews']) == 20
            guard.verify(pins)
            report.update(status='source-work-allocation-passed-awaiting-full-review-and-LOC', finished_at=guard.now(),
                profiles=qualified['profiles'], pins=pins, pins_unchanged_at_end=True,
                full_manifests_sealed=0, canonical_LOC_status='pending root immutable canonical overlay')
            guard.save(guard.REPORT, report)
        except BaseException as error:
            report.update(status='stopped-unqualified', finished_at=guard.now(), active=None,
                error=str(error) or type(error).__name__, error_type=type(error).__name__)
            guard.save(guard.REPORT, report)
            raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Nim tree/work supervisor received signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    parser.add_argument('--qualified-revision', required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert args.qualified_revision == 'private-v5'
    assert args.revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.revision)
    qualify(args)


if __name__ == '__main__':
    main()
