"""Qualify and time newly shorter listed sources; canonical LOC stays pending.

This uses the unchanged shared supervised sampler and original source bytes.
Each benchmark is a separate immutable study with fresh Mech AOT
controls. No producer reports, visualization data or source metrics are edited.
"""
import argparse
import copy
import fcntl
import json
from pathlib import Path
import signal
import subprocess
import sys

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
VROOT = CLBG / 'v-report-only/20261012'
sys.path.insert(0, str(VROOT))
import full_study as v_sampler
import prepare as bounded

CASES = {
    'fannkuchredux': dict(id='fannkuchredux-php-8', language='php',
        source='compact/upstream/fannkuchredux-php-8.php',
        sha256='7931a305151ebb9d4b54ec3729931622806334296557c2153637c2ea9315b1c4',
        proposed_loc=71, runtime='/opt/homebrew/opt/php/bin/php',
        flags=['-n', '-d', 'memory_limit=-1', '-d', 'opcache.enable_cli=1',
               '-d', 'opcache.jit_buffer_size=64M', '-d', 'opcache.jit=tracing',
               '-d', 'short_open_tag=1'],
        sizes=(1, 2, 3, 4, 5, 6, 8),
        work='Complete n! factoradic-order permutations, actual prefix reversals and global alternating checksum.'),
    'mandelbrot': dict(id='mandelbrot-lua-2', language='lua',
        source='compact/upstream/mandelbrot-lua-2.lua',
        sha256='16eacf7585991619923c8add0aa9ae0c1327aa2b5f31764208fcf27a5d52f249',
        proposed_loc=40, runtime='/opt/homebrew/opt/lua/bin/lua', flags=[],
        sizes=(0, 1, 2, 3, 7, 8, 9, 17, 31, 65),
        work='Every requested pixel, prescribed binary64 recurrence/escape predicate and complete zero-padded binary PBM.'),
    'spectralnorm': dict(id='spectralnorm-node-7', language='javascript',
        source='compact/upstream/spectralnorm-node-7.js',
        sha256='2dea06681168ffa46990b461ef7468f38afeb3fd955d8b3078d8918b1c5afd4a',
        proposed_loc=45, runtime='/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node', flags=[],
        sizes=(1, 2, 3, 5, 9),
        work='Forty fresh implicit coefficient products; no coefficient matrix or iteration-result cache.'),
    'binarytrees': dict(id='binarytrees-ruby-2', language='ruby',
        source='compact/upstream/binarytrees-ruby-2.rb',
        sha256='d75c541ff893ccfb5b5fa399010bc60c7f14f51d1077a0f983502ffe60f05f6c',
        proposed_loc=30, runtime='/opt/homebrew/opt/ruby/bin/ruby', flags=['--yjit', '-W0'],
        sizes=(0, 4, 7, 9),
        work='Distinct newly allocated two-child arrays at every node including leaves; recursive checks, prescribed tree counts and retained long-lived root.'),
}


def configure(benchmark):
    module = v_sampler.configure(benchmark)
    module.HERE = HERE / 'pending-minima' / benchmark
    module.OUT = module.HERE / 'full-study'
    module.MANIFEST = module.OUT / 'manifest.json'
    module.REPORT = module.OUT / 'report.json'
    return module


def qualify(benchmark):
    module = configure(benchmark)
    case = CASES[benchmark]
    directory = module.HERE / 'qualification-v1'
    print('WAIT common mutex: listed compact candidate', case['id'], flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not directory.exists() and not module.MANIFEST.exists(), 'No overwrite or implicit resume'
        harness = module.load_harness()
        harness.install_interrupt_handlers()
        fixtures = {}
        for name, count in (('oracle-fixtures.json', 14), ('preparation-fixtures.json', 5), ('cleanup-fixtures.json', 8)):
            receipt = json.loads((VROOT / name).read_text())
            assert receipt['status'] == 'passed' and receipt['tests_run'] == count
            fixtures[name] = receipt
        launch_fixture_path = HERE / 'minimum-launch-fixtures.json'
        launch_fixtures = json.loads(launch_fixture_path.read_text())
        assert launch_fixtures['status'] == 'passed' and launch_fixtures['tests_run'] == 4
        fixtures['minimum-launch-fixtures.json'] = launch_fixtures
        pins = bounded.merge_pins(*(receipt['pins'] for receipt in fixtures.values()))
        harness.PinGuard(pins).verify()
        source = CLBG / case['source']
        assert module.sha(source) == case['sha256']
        original_manifest = CLBG / 'compact/manifest.json'
        listed = next(row for row in json.loads(original_manifest.read_text())['programs'] if row['id'] == case['id'])
        assert listed['sha256'] == case['sha256'] and source == CLBG / 'compact' / listed['path']
        control_path = v_sampler.SUITE / 'source-audit-r.json'
        controls = json.loads(control_path.read_text())
        assert controls['status'] == 'reviewed'
        control = copy.deepcopy(next(row for row in controls['controls'] if row['benchmark'] == benchmark))
        control_files = [control_path]
        if benchmark == 'mandelbrot':
            control_path = CLBG / 'mandelbrot/experiments/20261011-array-state/source-modes-study/full-study/manifest.json'
            control_manifest = json.loads(control_path.read_text())
            control = copy.deepcopy(next(row for row in control_manifest['profiles'] if row['id'] == 'row16-aot-control'))
            control_files.append(control_path)
        assert module.sha(control['source']) == control['source_sha256']
        assert module.sha(control['binary']) == control['binary_sha256']
        control.update(id='mech-aot-control', key=benchmark + ':loc-minimum-study:mech-control',
                       role='Fresh current Mech AOT fastest-source control', maximum_workers=8,
                       producer_loc=control['loc'], loc=None, loc_status='pending canonical count overlay')
        runtime = Path(case['runtime']).resolve(strict=True)
        directory.mkdir(parents=True)
        snapshot = directory / source.name
        snapshot.write_bytes(source.read_bytes())
        assert module.sha(snapshot) == case['sha256']
        row = dict(id=case['id'], key=benchmark + ':' + case['id'], benchmark=benchmark,
            language=case['language'], source=str(snapshot), source_sha256=case['sha256'],
            original_source=str(source), command=[str(runtime)] + case['flags'] + [str(snapshot), '{n}'],
            cwd=str(CLBG), runtime_environment=bounded.ENVIRONMENT, maximum_workers=1,
            loc=None, proposed_canonical_loc=case['proposed_loc'], loc_status='pending final pool/source review',
            role='New listed compact candidate after canonical LOC reselection',
            algorithm=dict(strict=True, native_benchmark_kernel=False, reason=case['work']),
            publication_status='pending qualification, all three full samples and count review')
        files = [Path(__file__).resolve(), source, snapshot, runtime, original_manifest,
                 VROOT / 'full_study.py', VROOT / 'prepare.py', v_sampler.HELPER,
                 v_sampler.SUITE / 'run.py', v_sampler.SUITE / 'prepare.py',
                 CLBG / 'compact/run.py', CLBG / 'nbody/run.py', CLBG / 'array-comparison/all-20261009/run.py']
        files += [VROOT / name for name in fixtures if name != 'minimum-launch-fixtures.json']
        files.append(launch_fixture_path)
        files += control_files + [Path(control['source']), Path(control['binary'])]
        pins = bounded.merge_pins(pins, {str(path): module.sha(path) for path in files})
        harness.PinGuard(pins).verify()
        module.OUT, module.REPORT = directory, directory / 'probes.json'
        report = dict(status='qualifying', benchmark=benchmark, created_at=module.now(),
                      attempts=[], active=None, full_performance_runs=0, chart_admitted=False)
        module.save(module.REPORT, report)
        try:
            harness.compact.wait_for_environment(harness.REQUIRED)
            version_command = [str(runtime)] + (['--version'] if case['language'] != 'lua' else ['-v'])
            versions = directory / 'runtime-version'
            versions.mkdir()
            bounded.command_build(module, harness, version_command, harness.compact.ENV, versions, lock.fileno())
            row['runtime'] = dict(path=str(runtime), sha256=module.sha(runtime),
                version=(versions / 'build.stdout').read_text().strip() or (versions / 'build.stderr').read_text().strip())
            checks = [bounded.probe(module, harness, row, 'canonical-small',
                dict(pins=pins, timeout_seconds=300, maximum_rss_mib=8192), report, lock.fileno())]
            for n in case['sizes']:
                checks.append(bounded.probe(module, harness, row, 'size-' + str(n),
                    dict(pins=pins, timeout_seconds=300, maximum_rss_mib=8192), report, lock.fileno(), n=n))
            row.update(small_validation=checks[0]['validation'], adversarial_validation=checks[1:],
                       required_work_review=dict(status='passed', method='Root source review plus canonical and independent bounded outputs',
                           source_sha256=case['sha256'], contract=case['work']))
            harness.PinGuard(pins).verify()
            report.update(status='bounded-outputs-passed', profile=row, pins=pins, pins_unchanged_at_end=True)
        except BaseException as error:
            report.update(status='stopped', error=str(error), error_type=type(error).__name__, no_automatic_retry=True)
            raise
        finally:
            report.update(finished_at=module.now(), active=None)
            module.save(module.REPORT, report)
        qualification_path = module.REPORT
        module = configure(benchmark)
        profiles = [row, control]
        ids = [profile['id'] for profile in profiles]
        pins = bounded.merge_pins(pins, {str(qualification_path): module.sha(qualification_path)})
        manifest = dict(status='prepared-awaiting-root-review', benchmark=benchmark,
            created_at=module.now(), samples=3, size=harness.WORK[benchmark][1], profiles=profiles,
            orders=[dict(round=i + 1, profiles=ids[i % 2:] + ids[:i % 2]) for i in range(3)],
            pins=pins, shared_timing_lock=str(module.LOCK), required_power_profile=harness.REQUIRED,
            maximum_rss_mib=8192, timeout_seconds=14400, maximum_compute_workers=8, environment_retry_limit=3,
            qualification=str(qualification_path), chart_admitted=False, loc_status='pending final canonical audit',
            timing_boundary='Fresh process startup, all prescribed work, full output, and shutdown; preparation excluded.',
            retry_policy='Only bounded environment-only retries; output/work/resource/pin/ownership failures stop and retain evidence.')
        harness.PinGuard(pins).verify()
        module.save(module.MANIFEST, manifest)
        print(json.dumps(dict(manifest=str(module.MANIFEST), sha256=module.sha(module.MANIFEST),
                              status=manifest['status'], loc_status=manifest['loc_status'])), flush=True)


def launch_controller(module, command, manifest_sha256, loc_status):
    """Caller holds the common mutex until durable ownership transfer/cleanup.

    The child must acquire that same mutex before any sampler or native trial.
    A failed launch therefore cannot interrupt a valid timed trial.
    """
    child, transferred = None, False
    receipt = dict(command=command, launched_at=module.now(),
                   manifest_sha256=manifest_sha256, loc_status=loc_status,
                   chart_admitted=False)
    try:
        with (module.OUT / 'supervisor.log').open('xb') as log:
            child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=log,
                                     stderr=subprocess.STDOUT, start_new_session=True)
        receipt['pid'] = child.pid
        receipt['identity'] = module.identity(child.pid)
        if receipt['identity'] is None and child.poll() is None:
            raise RuntimeError('Detached supervisor identity missing while live')
        module.save(module.OUT / 'launch.json', receipt)
        transferred = True
        return receipt
    except BaseException as error:
        receipt.update(error=str(error), error_type=type(error).__name__,
                       ownership_transferred=False)
        cleanup_error = None
        try:
            if child is not None and not transferred:
                try:
                    module.load_harness().kill_group(child.pid)
                except BaseException as failure:
                    cleanup_error = failure
                    receipt.update(cleanup_error=str(failure),
                                   cleanup_error_type=type(failure).__name__)
                    if child.poll() is None:
                        # This is our direct Python controller, still blocked
                        # before sampler startup by the parent's common mutex.
                        child.kill()
                        receipt['fallback_owned_controller_kill'] = True
                finally:
                    child.wait(timeout=10)
                    remaining = module.load_harness().group_members(child.pid)
                    receipt.update(cleanup_confirmed=not remaining,
                                   returncode=child.returncode,
                                   remaining_owned_group_members=remaining)
                    if remaining:
                        raise RuntimeError('Owned launch group remains; inspect retained failure receipt')
        finally:
            module.save(module.OUT / 'launch-failure.json', receipt)
        if cleanup_error is not None:
            raise RuntimeError('Detached controller cleanup failed; retained receipt') from cleanup_error
        raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Compact minimum preparation interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    parser.add_argument('--benchmark', choices=CASES, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare', action='store_true')
    mode.add_argument('--run', action='store_true')
    mode.add_argument('--detach', action='store_true')
    parser.add_argument('--manifest-sha256')
    args = parser.parse_args()
    if args.prepare:
        qualify(args.benchmark)
        return
    module = configure(args.benchmark)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert args.manifest_sha256 and module.sha(module.MANIFEST) == args.manifest_sha256
        manifest = json.loads(module.MANIFEST.read_text())
        assert manifest['status'] == 'prepared-awaiting-root-review' and manifest['samples'] == 3
        assert len(manifest['profiles']) == 2 and manifest['benchmark'] == args.benchmark
        module.load_harness().PinGuard(manifest['pins']).verify()
        if args.detach:
            with (module.OUT / 'launch-owner.lock').open('a+') as owner:
                fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
                assert not (module.OUT / 'launch.json').exists() and not module.REPORT.exists()
                command = [sys.executable, '-u', str(Path(__file__).resolve()), '--run',
                           '--benchmark', args.benchmark, '--manifest-sha256', args.manifest_sha256]
                receipt = launch_controller(module, command, args.manifest_sha256,
                                            manifest['loc_status'])
                print(json.dumps(receipt, indent=2), flush=True)
                return
    module.run(args.manifest_sha256)


if __name__ == '__main__':
    main()
