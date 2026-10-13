"""Qualify the repaired probe and private native images using the owned service."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
from types import SimpleNamespace

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
CONTROL = H / '.build/swi-halt-diagnostic-v1'
OWNER = C / 'owned-jobs/prolog-halt-controls-v1'
REVIEW = C / 'jobs/prolog-halt-controls-closed-review-v1/review.json'
PRIOR = H / '.build/swi-runtime-probe-v1'
LEGACY = H / 'revisions/runtime-directory-v1/bootstrap_runtime_directory_v1.py'
CONTROL_SHA = 'd43a5f0845b2b0ba7b312905265b1edf22071e3b99cbcc7a0d391af6b1bace29'
TERMINAL_SHA = '5dd615c6f9e6af4f773f1223a570c0217f3c5a5942524d6c3cf7f7626b69aee3'
OWNER_SHA = 'ebe4960eceb30abe9615384f42301468f7cb949aa5b20dfa769b671555fa0742'
PROPOSAL_SHA = '5b9186bb51e748fe69b16dcd35a632dd9ed8556abdbc30cbab45159a36a1bf88'
FILES = ('discover_runtime.py', 'tools/runtime_probe.pl', 'tools/prepare_without_main.pl',
         'tools/prepare_qlf.pl', 'support/clbg_io.pl', 'REVIEW.md')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--packet-pins', required=True, type=Path)
    parser.add_argument('--packet-pins-sha256', required=True)
    parser.add_argument('--closed-review-sha256', required=True)
    args = parser.parse_args()
    lockpath = Path('/tmp/mech-clbg-timing.lock')
    print('WAIT common mutex: repaired Prolog feature and image qualification', flush=True)
    with lockpath.open('r+') as common, (OWNER / 'owner.lock').open('r+') as lease:
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), lockpath.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        lh, ln = os.fstat(lease.fileno()), (OWNER / 'owner.lock').stat()
        assert (lh.st_dev, lh.st_ino) == (ln.st_dev, ln.st_ino) and sys.dont_write_bytecode
        output = H / '.build/swi-runtime-success-halt-v1'
        output.mkdir()
        pins, legacy, failure = {}, None, None
        report = dict(status='running', chart_admission=False, runtime_configuration_admitted=False,
            source_work_qualified=False, full_processes_authorized=0, actual_benchmark_runs=0,
            fixture_reexecutions=0, version_reexecutions=0, halt_control_reexecutions=0,
            downloads=0, extractions=0, slices=0, inherited_common_mutex=True)

        def read(path, digest):
            assert sha(path) == digest, str(path)
            assert str(path) not in pins or pins[str(path)] == digest
            pins[str(path)] = digest
            return json.loads(path.read_text())

        try:
            packet = read(args.packet_pins, args.packet_pins_sha256)
            assert set(packet['files']) == set(FILES)
            for name, digest in packet['files'].items():
                path = P / name
                assert not path.stat().st_mode & 0o222 and sha(path) == digest
                pins[str(path)] = digest
            review = read(REVIEW, args.closed_review_sha256)
            assert review['status'] == 'actual-halt-recovery-exit-replacement-confirmed'
            assert review['report_sha256'] == CONTROL_SHA and review['verified_pins'] == 13233
            prior = read(CONTROL / 'report.json', CONTROL_SHA)
            terminal = read(CONTROL / 'terminal-pin.json', TERMINAL_SHA)
            assert prior['status'] == terminal['status'] == 'halt-recovery-exit-replacement-confirmed'
            assert prior['pins'] == terminal['partial_evidence_pins'] and terminal['report_sha256'] == CONTROL_SHA
            assert [(v['mode'], v['exit_status']) for v in prior['controls']] == [('direct', 0), ('wrapped', 1)]
            assert prior['terminal_evidence_complete'] is True
            for path, digest in prior['pins'].items():
                assert path not in pins or pins[path] == digest
                assert sha(path) == digest, path
                pins[path] = digest
            done = read(OWNER / 'terminal.json', OWNER_SHA)
            assert done['status'] == 'command-exited-zero-awaiting-report-review'
            assert type(done['exit_code']) is int and done['exit_code'] == 0 and done['stop_signals'] == []
            started, worker = [read(OWNER / n, sha(OWNER / n)) for n in ('command-started.json', 'worker-started.json')]
            assert done['command_identity'] == started['command_identity'] and done['worker_identity'] == worker['worker_identity']

            origins = ('revisions/runtime-probe-v1/runtime_probe.pl', 'revisions/runtime-probe-v1/prepare_without_main.pl', 'drafts/private-v1/tools/prepare_qlf.pl')
            for name, origin in zip(FILES[1:4], origins):
                old = (H / origin).read_bytes()
                assert sha(H / origin) == pins[str(H / origin)]
                before = b'close(Out)),\n    halt(0).'
                main_before = b'main :- catch(checked, Error, (print_message(error, Error), halt(1))).'
                main_after = b'main :-\n    catch(checked, Error, (print_message(error, Error), halt(1))),\n    halt(0).'
                assert old.count(before) == old.count(main_before) == 1
                assert (P / name).read_bytes() == old.replace(before, b'close(Out)).').replace(main_before, main_after)
            old_io = H / 'drafts/private-v1/support/clbg_io.pl'
            assert sha(old_io) == pins[str(old_io)]
            expected_io = old_io.read_bytes().replace(b'    catch(run_checked(Goal), Error, report_error(Error)).', b'    catch(run_checked(Goal), Error, report_error(Error)),\n    halt(0).').replace(b'    ->  flush_output(user_output),\n        halt(0)', b'    ->  flush_output(user_output)')
            assert (P / 'support/clbg_io.pl').read_bytes() == expected_io

            spec = importlib.util.spec_from_file_location('success_halt_legacy', LEGACY)
            legacy = importlib.util.module_from_spec(spec)
            sys.modules[spec.name] = legacy
            raw = LEGACY.read_bytes()
            assert hashlib.sha256(raw).hexdigest() == pins[str(LEGACY)]
            exec(compile(raw, str(LEGACY), 'exec'), legacy.__dict__)
            preparation, sealed, pins = legacy.load_preparation(SimpleNamespace(proposal_sha256=PROPOSAL_SHA), pins)
            service, references, guard, harness, pins = legacy.reused_service(preparation, sealed, pins)
            images = legacy.fresh_load(legacy.PACKET / 'image_closure_v2.py', 'success_halt_images', pins)
            previous_receipt = read(PRIOR / 'runtime/feature-probe.json', pins[str(PRIOR / 'runtime/feature-probe.json')])
            previous_result = read(PRIOR / 'runtime/feature-result.json', pins[str(PRIOR / 'runtime/feature-result.json')])
            binary = Path(previous_receipt['command'][0])
            native = H / '.build/swi-runtime-directory-v1/arm64-app/SWI-Prolog.app'
            home = Path(previous_result['runtime']['home'])
            assert binary.is_relative_to(native) and home.is_relative_to(native) and sha(binary) == pins[str(binary)]
            environment = dict(harness.compact.ENV, SWI_HOME_DIR=str(home))
            guard.OUT, guard.REPORT = output, output / 'report.json'
            harness.install_interrupt_handlers()
            harness.compact.wait_for_environment(harness.REQUIRED)
            legacy.verify(pins)
            result_path = output / 'runtime/feature-result.json'
            command = preparation.prefix(binary) + ['-l', str(P / 'tools/runtime_probe.pl'), '-g', 'clbg_runtime_probe:main', '--', str(result_path)]
            probe = service.owned_command(guard, harness, command, environment, output / 'runtime', common.fileno(), 120, 'feature-probe')
            detail = probe['detail']
            assert type(detail['exit_status']) is type(detail['final_exit_status']) is int
            assert detail['exit_status'] == detail['final_exit_status'] == 0
            actual = json.loads(result_path.read_text())
            contracts = preparation.module(legacy.ORIGINAL_PACKET / 'contracts.py', 'success_halt_contracts', pins)
            contracts.runtime_flags(actual['runtime'], home)
            large = (1 << 220) + 12345
            assert actual['large'] == large and actual['quotient'] == large // 97 and actual['remainder'] == large % 97
            assert actual['negative_floor'] == -3 and actual['negative_modulo'] == 2 and actual['square_root'] == 2.0
            assert (actual['hash_value'], actual['hash_size']) == (5, 4) and actual['hash_capacity'] >= 8
            assert actual['regex_matches'] == 2 and actual['regex_replacement'] == 'xbx' and actual['regex_jit_requested'] is False
            foreign = [Path(row['resolved_path']).resolve(strict=True) for row in actual['foreign_libraries']]
            assert all(path.is_relative_to(native.resolve()) for path in foreign)
            allowed = {P / 'tools/runtime_probe.pl', P / 'tools/prepare_without_main.pl'}
            for name in actual['loaded_sources']:
                path = Path(name).resolve(strict=True)
                assert path.is_relative_to(native.resolve()) or path in allowed
                assert sha(path) == pins[str(path)]
            image_pins, visited = images.inspect_closure(preparation, service, guard, harness, binary, foreign, environment, output / 'images', common.fileno(), native)
            pins = legacy.merge(pins, image_pins, service.pin_tree(native))
            report.update(status='runtime-discovered-awaiting-root-source-library-review',
                binary=str(binary), binary_sha256=sha(binary), home=str(home), architecture='arm64',
                version_components=[10, 0, 2], runtime_backend='swi-native-bytecode-vm',
                allowed_library_roots=[str(native)], environment={'SWI_HOME_DIR': str(home)},
                actual_feature_probe=str(result_path), actual_feature_probe_sha256=sha(result_path),
                feature_probe_owned_receipt=legacy.actual_receipt(preparation, output / 'runtime/feature-probe.json', command),
                loaded_native_images=visited, default_gc_observed=actual['runtime']['gc'],
                bounded_observed=actual['runtime']['bounded'], pcre_version=actual['pcre_version'],
                prior_failure_relabelled_as_success=False, successful_version_reused=True)
        except BaseException as error:
            failure = error
        finally:
            if legacy is not None:
                legacy.terminal(output, report, pins, failure)
            else:
                report.update(status='stopped-before-service', error=str(failure), pins=pins)
                with (output / 'report.json').open('x') as stream:
                    json.dump(report, stream, indent=2)
        if failure is not None:
            raise failure
        print(json.dumps(dict(status=report['status'], report=str(output / 'report.json'), report_sha256=sha(output / 'report.json'))), flush=True)


if __name__ == '__main__':
    main()
