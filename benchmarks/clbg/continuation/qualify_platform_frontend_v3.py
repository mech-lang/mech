"""Run the existing platform selection assertions from a frozen mirrored tree."""
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
NODE = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node')
PREPARATION = CLBG / 'c3-comparison/20261012/revisions/private-v3/guarded_prepare.py'
PREREQUISITE = HERE / 'jobs/native-platform-v1-fixtures/fixture-receipt.json'
PREVIOUS = HERE / 'jobs/platform-frontend-v2/fixture-receipt.json'
OUTPUT = HERE / 'jobs/platform-frontend-v3'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def main():
    print('WAIT common mutex: platform frontend selection assertions', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        assert not OUTPUT.exists(), 'Preserve every actual terminal result'
        OUTPUT.mkdir()
        pins = {str(Path(__file__).resolve()): sha(__file__)}
        receipt = dict(status='failed', common_timing_mutex=str(LOCK),
            exclusive_common_timing_mutex_held=True, actual_benchmark_runs=0,
            chart_admission=False, dataset_mutated=False, automatic_retry=False,
            started_at=datetime.now(timezone.utc).isoformat())
        try:
            prior = json.loads(PREREQUISITE.read_text())
            terminal = json.loads(PREREQUISITE.with_name('terminal-pins.json').read_text())
            assert terminal['terminal_status'] == prior['status'] == 'passed'
            assert sha(PREREQUISITE) == terminal['receipt_sha256']
            assert prior['tests_run'] == prior['expected_fixture_count'] == 4
            assert prior['pins_unchanged_at_end'] is prior['exclusive_common_timing_mutex_held'] is True
            assert all(prior[name] == [] for name in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
            assert set(prior['actual_started_fixture_ids']) == set(prior['required_fixture_ids'])
            pins[str(PREREQUISITE)] = sha(PREREQUISITE)
            pins[str(PREREQUISITE.with_name('terminal-pins.json'))] = sha(PREREQUISITE.with_name('terminal-pins.json'))
            for path, value in prior['source_pins'].items():
                assert sha(path) == value
                pins[path] = value
            previous = json.loads(PREVIOUS.read_text())
            previous_terminal = json.loads(PREVIOUS.with_name('terminal-pins.json').read_text())
            assert previous['status'] == previous_terminal['terminal_status'] == 'passed'
            assert sha(PREVIOUS) == previous_terminal['pins'][str(PREVIOUS)]
            assert previous['pins_unchanged_at_end'] is True
            for path, value in previous['source_pins'].items():
                assert sha(path) == value
                pins[path] = value
            pins[str(PREVIOUS)] = sha(PREVIOUS)
            pins[str(PREVIOUS.with_name('terminal-pins.json'))] = sha(PREVIOUS.with_name('terminal-pins.json'))
            snapshot = OUTPUT / 'snapshot'
            sources = [HERE / 'platform-filter-proposal-v3' / name
                for name in ('platform.mjs', 'traits.mjs', 'test_platform.mjs')]
            sources.append(CLBG / 'overview/explorer/dist/math.mjs')
            for source in sources:
                # The relative math import resolves exactly as in the candidate.
                target = snapshot / source.relative_to(CLBG)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, target)
                assert sha(source) == sha(target)
                pins[str(source)] = pins[str(target)] = sha(source)
                target.chmod(0o444)
            qualification = json.loads((CLBG / 'c3-comparison/20261012/qualification/private-v3/report.json').read_text())
            assert sha(PREPARATION) == qualification['pins'][str(PREPARATION)]
            pins[str(PREPARATION)] = sha(PREPARATION)
            spec = importlib.util.spec_from_file_location('platform_frontend_owned_commands', PREPARATION)
            preparation = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(preparation)
            guard = preparation.load_guard()
            harness = guard.load_harness()
            for module in (guard, harness):
                path = Path(module.__file__).resolve()
                pins[str(path)] = sha(path)
            node = NODE.resolve(strict=True)
            pins[str(node)] = sha(node)
            environment = dict(qualification['runtime_environment'])
            for name in ('NODE_OPTIONS', 'NODE_PATH'):
                environment.pop(name, None)
            version = preparation.owned_command(guard, harness, [str(node), '--version'],
                environment, OUTPUT / 'node-version', lock.fileno(), 15, 'version')
            assert re.fullmatch(rb'v\d+\.\d+\.\d+\s*', version['stdout'])
            test = snapshot / 'continuation/platform-filter-proposal-v3/test_platform.mjs'
            outcome = preparation.owned_command(guard, harness,
                [str(node), '--test', '--test-reporter=tap', str(test)],
                environment, OUTPUT / 'assertions', lock.fileno(), 30, 'frontend')
            raw = outcome['stdout'].decode()
            assert '# Platform evidence and selection fixtures passed.' in raw
            for field, count in (('tests', 1), ('pass', 1), ('fail', 0), ('cancelled', 0), ('skipped', 0), ('todo', 0)):
                assert re.search(r'^# ' + field + r' ' + str(count) + r'$', raw, re.M), (field, raw)
            for path, digest in pins.items():
                assert sha(path) == digest, ('Frontend input changed', path)
            receipt.update(status='passed', node_version=version['stdout'].decode().strip(),
                actual_test_files=1, assertion_script=str(test), pins_unchanged_at_end=True,
                coverage='Platform choices, OR/AND selection, release year, manual selection, fixed axes, contradictory backend evidence and actual runtime memory provenance')
        except BaseException as error:
            receipt.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            (OUTPUT / 'failure.log').write_text(traceback.format_exc())
            raise
        finally:
            receipt.update(source_pins=pins, completed_at=datetime.now(timezone.utc).isoformat())
            save(OUTPUT / 'fixture-receipt.json', receipt)
            terminal_pins = {}
            try:
                for path in sorted(OUTPUT.rglob('*')):
                    if path.is_file(): terminal_pins[str(path)] = sha(path)
            except BaseException as error:
                save(OUTPUT / 'terminal-pins.json', dict(terminal_status='failed-terminal-integrity',
                    preceding_fixture_status=receipt['status'],pins=terminal_pins,
                    error=str(error) or type(error).__name__,error_type=type(error).__name__,
                    available_bytes_not_fully_pinned=True,exclusive_common_timing_mutex_held=True,automatic_retry=False))
                raise
            save(OUTPUT / 'terminal-pins.json', dict(terminal_status=receipt['status'],
                pins=terminal_pins,exclusive_common_timing_mutex_held=True, automatic_retry=False))
        print(json.dumps(dict(status=receipt['status'], receipt=str(OUTPUT / 'fixture-receipt.json'))), flush=True)


if __name__ == '__main__':
    main()
