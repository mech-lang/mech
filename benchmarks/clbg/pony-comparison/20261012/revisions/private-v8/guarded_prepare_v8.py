"""Unexecuted additive directory-init repair; preserve all154 workload cases."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import sys
from types import SimpleNamespace

PACKET = Path(__file__).resolve().parent
COHORT = PACKET.parents[1]
sys.path.insert(0, str(COHORT))
import guarded_prepare_v6 as base
LOCK = base.LOCK
FROZEN_V7 = COHORT / 'revisions/private-v7'
V7_PROPOSAL = COHORT / 'proposals/private-v7.json'
V7_SHA = 'd6d318a54b4277527ceb9a2ecd0ab35c46c86ddfb55228e453df5117ac111512'


def dependencies(guard, pins):
    import importlib
    guard.verify(pins)
    sys.path.insert(0, str(FROZEN_V7))
    loaded = []
    for name, directory in (('owned_service_v7', FROZEN_V7), ('proof_reuse_v7', FROZEN_V7), ('directory_init_v8', PACKET)):
        assert name not in sys.modules, 'Require fresh exact pinned module identity'
        module = importlib.import_module(name)
        assert Path(module.__file__).resolve() == directory / (name + '.py')
        loaded.append(module)
    guard.verify(pins)
    return loaded


def failed_v7(repair, old):
    directory = COHORT / 'qualification/private-v7'
    report_path = directory / 'report.json'
    report = json.loads(report_path.read_text()); repair.validate_failed_v7(report, COHORT)
    assert not (directory / 'source-proofs').exists()
    build = COHORT / '.build/private-v7'
    assert build.is_dir() and list(build.iterdir()) == []
    source = directory / 'source-snapshot/support/standard_primitives/main.pony'
    assert base.sha(source) == old['pins'][old['corrected_primitive_source']]
    return dict(status='preserved-v7-directory-failure-before-any-native-build-or-run',
        report=str(report_path), report_sha256=base.sha(report_path), failed_build_root=str(build),
        attempts=0, profiles=0, native_cases_reexecuted=0, pins=base.pin_tree(directory))


def packet_paths():
    return sorted(set(PACKET.glob('*.py')) | set(PACKET.glob('*.md')) |
                  set((PACKET / 'support').rglob('*.pony')))


def toolchain():
    build = COHORT / '.build/private-v6'
    return dict(compiler=str(build / 'release/0.74.0-95ad180/bin/ponyc'),
        release_root=str(build / 'release'), regex_root=str(next((build / 'regex').glob('*/regex/regex.pony')).parents[1]),
        gmp_root=str(next((build / 'gmp4pony').glob('*/gmp/mpz.pony')).parents[1]),
        sdk=json.loads((COHORT / 'proposals/private-v6.json').read_text())['sdk'],
        original_qualification='private-v6')


def seal(proposal_file):
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not proposal_file.exists()
        guard = base.load_guard(); harness = guard.load_harness()
        harness.install_interrupt_handlers()
        packet_inventory = packet_paths()
        packet_pins = {str(path): base.sha(path) for path in packet_inventory}
        assert base.sha(V7_PROPOSAL) == V7_SHA
        old = json.loads(V7_PROPOSAL.read_text())
        inherited_pins = base.merge(old['pins'], {str(V7_PROPOSAL): V7_SHA}, packet_pins)
        service, proof_reuse_v7, repair = dependencies(guard, inherited_pins)
        failure = failed_v7(repair, old)
        # Reuse the actual successful6+3 fixture receipts; no repetition.
        for suite, count in (('proof-reuse', 6), ('owned-service', 3)):
            old_receipt = COHORT / 'proposal-seals/private-v7/fixtures' / suite / (suite + '-fixture-receipt.json')
            receipt = json.loads(old_receipt.read_text())
            assert receipt['status'] == 'passed' and receipt['tests_run'] == receipt['expected_count'] == count
            assert receipt['actual_common_mutex_held'] is receipt['inherited_common_mutex'] is True
            assert all(receipt[name] == 0 for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'))
            assert len(receipt['started_ids']) == count and sorted(receipt['expected_ids']) == sorted(receipt['discovered_ids']) == sorted(receipt['started_ids'])
            assert base.sha(old_receipt) == inherited_pins[str(old_receipt)]
            assert base.sha(receipt['raw_log']) == receipt['raw_log_sha256'] == inherited_pins[receipt['raw_log']]
        output = COHORT / 'proposal-seals/private-v8'
        assert not output.exists(); output.mkdir(parents=True)
        for suite, count in (('directory-init', 1),):
            fixture_output = output / 'fixtures' / suite
            service.owned_command(base, guard, harness,
                [sys.executable, '-B', str(PACKET / 'run_fixtures_v8.py'), '--suite', suite,
                 '--output', str(fixture_output), '--lock-fd', str(lock.fileno())],
                dict(os.environ), fixture_output, lock.fileno(), 30, 'actual-fixtures')
            receipt = json.loads((fixture_output / (suite + '-fixture-receipt.json')).read_text())
            assert receipt['status'] == 'passed' and receipt['expected_count'] == receipt['tests_run'] == count
            assert receipt['actual_common_mutex_held'] is True and receipt['inherited_common_mutex'] is True
            assert receipt['failures'] == receipt['errors'] == receipt['skipped'] == 0
            assert receipt['expected_failures'] == receipt['unexpected_successes'] == 0
            assert sorted(receipt['expected_ids']) == sorted(receipt['discovered_ids']) == sorted(receipt['started_ids'])
            assert len(receipt['started_ids']) == count
            assert base.sha(receipt['raw_log']) == receipt['raw_log_sha256']
        assert packet_paths() == packet_inventory
        guard.verify(packet_pins)
        reuse = json.loads(Path(old['prior_proof_reuse']).read_text())
        assert base.sha(old['prior_proof_reuse']) == old['prior_proof_reuse_sha256']
        proof_reuse_v7.validate_prior_reports(reuse)
        reuse_path = output / 'proof-reuse.json'; base.new_json(reuse_path, reuse)
        pins = base.merge(reuse['pins'], {str(reuse_path): base.sha(reuse_path)},
            inherited_pins, failure['pins'], base.pin_tree(output / 'fixtures'))
        compiler = toolchain(); compiler['compiler_sha256'] = base.sha(compiler['compiler'])
        guard.verify(pins)
        proposal = dict(status='sealed-primitive-correction-awaiting-root-native-review',
            pins=pins, packet=str(PACKET), prior_proof_reuse=str(reuse_path),
            prior_proof_reuse_sha256=base.sha(reuse_path), reused_profiles=20, reused_bounded_cases=154,
            new_workload_builds=0, new_workload_native_cases=0, fresh_unscored_primitive_builds=1,
            fresh_unscored_primitive_cases=1, toolchain=compiler,
            corrected_primitive_source=old['corrected_primitive_source'],
            actual_fixture_suites={'directory-init': 1}, reused_fixture_suites={'proof-reuse': 6, 'owned-service': 3},
            preserved_failed_v7=failure, prior_sealed_proposal=str(V7_PROPOSAL), prior_sealed_proposal_sha256=V7_SHA,
            preserved_failed_primitive=reuse['preserved_failed_primitive'],
            no_automatic_retry=True, chart_admission=False, full_performance_runs=0)
        base.new_json(proposal_file, proposal)
        print(json.dumps(dict(proposal=str(proposal_file), sha256=base.sha(proposal_file),
            reused_bounded_cases=154, new_workload_native_cases=0, fresh_unscored_primitive_cases=1)))


def run(proposal_file, digest):
    with (PACKET / 'native-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert base.sha(proposal_file) == digest
            proposal = json.loads(proposal_file.read_text())
            guard = base.load_guard(); guard.verify(proposal['pins'])
            harness = guard.load_harness(); harness.install_interrupt_handlers()
            service, proof_reuse_v7, repair = dependencies(guard, proposal['pins'])
            import source_proofs
            assert Path(source_proofs.__file__).resolve() == COHORT / 'source_proofs.py'
            old = json.loads(V7_PROPOSAL.read_text())
            assert base.sha(V7_PROPOSAL) == V7_SHA
            current_failure = failed_v7(repair, old)
            assert current_failure == proposal['preserved_failed_v7']
            reuse = json.loads(Path(proposal['prior_proof_reuse']).read_text())
            assert base.sha(proposal['prior_proof_reuse']) == proposal['prior_proof_reuse_sha256']
            proof_reuse_v7.validate_prior_reports(reuse)
            proof_reuse_v7.failed_primitive_evidence(base)
            guard.verify(reuse['pins'])
            output = COHORT / 'qualification/private-v8'
            assert not output.exists(); output.mkdir()
            guard.OUT = output; guard.REPORT = output / 'report.json'
            report = dict(status='preparing-primitive-only', proposal_sha256=digest, attempts=[], active=None,
                full_performance_runs=0, chart_admission=False, no_automatic_retry=True,
                reused_bounded_cases=154, reused_cases_relabelled_as_attempts=False)
            guard.save(guard.REPORT, report)
            try:
                snapshot = output / 'source-snapshot'; snapshot.mkdir()
                shutil.copytree(COHORT / 'support-v2', snapshot / 'support-v2')
                corrected = Path(proposal['corrected_primitive_source'])
                target = snapshot / 'support/standard_primitives/main.pony'
                target.parent.mkdir(parents=True); shutil.copyfile(corrected, target)
                assert base.sha(target) == proposal['pins'][str(corrected)]
                build = COHORT / '.build/private-v8'
                repair.initialize_build_root(build)
                c = proposal['toolchain']
                environment = dict(os.environ, SDKROOT=c['sdk'], PATH='/usr/bin:/bin',
                    PONY_MAX_THREADS='8', LD_LIBRARY_PATH=base.LIBRARY_PATH, DYLD_LIBRARY_PATH=base.LIBRARY_PATH)
                for key in ('PONY_PATH','PONYPATH','PONYC_OPTIONS','CFLAGS','CPPFLAGS','LDFLAGS',
                            'DYLD_INSERT_LIBRARIES','DYLD_FRAMEWORK_PATH','DYLD_FALLBACK_LIBRARY_PATH'):
                    environment.pop(key, None)
                pins = base.merge(proposal['pins'], {str(proposal_file): digest}, base.pin_tree(snapshot))
                primitive_preparation = SimpleNamespace(**vars(base))
                primitive_preparation.owned_command = lambda g, h, cmd, env, out, fd, timeout, name: service.owned_command(
                    base, g, h, cmd, env, out, fd, timeout, name)
                primitive_preparation.native_images = lambda g, h, b, e, o, fd, roots: service.native_images(
                    base, g, h, b, e, o, fd, roots + [Path(c['release_root'])])
                proofs = source_proofs.run(primitive_preparation, guard, harness, proposal, snapshot, build, Path(c['compiler']),
                    Path(c['regex_root']), Path(c['gmp_root']), Path(c['sdk']), environment,
                    dict(PONY_MAX_THREADS='8', LD_LIBRARY_PATH=base.LIBRARY_PATH, DYLD_LIBRARY_PATH=base.LIBRARY_PATH),
                    pins, lock.fileno(), report)
                rows = []
                for prior in reuse['profiles'].values():
                    row = proof_reuse_v7.profile_row(prior)
                    origin = Path(prior['binary']).parents[2].name
                    row['production_build_receipt'] = str(COHORT / 'qualification' / origin / 'builds' / row['id'] / 'build.json')
                    rows.append(row)
                assert len(rows) == 20 and len(report['attempts']) == 1
                pins = base.merge(pins, base.pin_tree(output / 'source-proofs'))
                guard.verify(pins)
                report.update(status='bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review',
                    profiles=rows, proofs=proofs, toolchain=c, pins=pins, fresh_profile_builds=0,
                    fresh_unscored_primitive_builds=1, fresh_unscored_primitive_cases=1,
                    prior_proof_reuse=proposal['prior_proof_reuse'])
            except BaseException as error:
                report.update(status='stopped-unqualified', error=str(error), error_type=type(error).__name__)
                raise
            finally:
                guard.save(guard.REPORT, report)


def main():
    parser = argparse.ArgumentParser(); modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--seal', action='store_true'); modes.add_argument('--run', action='store_true')
    parser.add_argument('--proposal-file', type=Path, default=COHORT / 'proposals/private-v8.json')
    parser.add_argument('--proposal-sha256'); args = parser.parse_args()
    proposal = args.proposal_file.resolve()
    assert proposal.is_relative_to(COHORT) and proposal.suffix == '.json'
    if args.seal: seal(proposal)
    else: assert args.proposal_sha256; run(proposal, args.proposal_sha256)


if __name__ == '__main__': main()
