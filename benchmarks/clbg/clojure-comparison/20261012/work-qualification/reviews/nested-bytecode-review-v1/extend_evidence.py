"""Additive disassembly of already qualified K classes; no compile or subject run."""
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import traceback

HERE = Path(__file__).resolve().parents[3]
SOURCE = Path(__file__).resolve()
LOCK = Path('/tmp/mech-clbg-timing.lock')
ACTUAL = HERE / 'work-qualification/jobs/work-v1/report.json'
ACTUAL_SHA = 'e193dfcebfada205469caef3f2b80c8cdffa59ff0cbb955d7e6ff240fe29c371'
OUTPUT = SOURCE.parent / 'actual-evidence'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write_new(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def main():
    print('WAIT common mutex for additive qualified-class javap review', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not OUTPUT.exists(), 'Exclusive additional evidence; no automatic retry'
        OUTPUT.mkdir()
        pins = {str(SOURCE): sha(SOURCE)}
        terminal = dict(status='started-additive-class-disassembly', commands=[], pins=pins,
                        scored_classes_mutated=False, observer_classes_mutated=False,
                        compilation_commands=0, native_benchmark_subjects=0)
        duplicate = None
        try:
            assert sha(ACTUAL) == ACTUAL_SHA
            actual = json.loads(ACTUAL.read_text())
            assert actual['status'] == 'actual-work-output-ownership-passed-awaiting-source-bytecode-review'
            assert actual['pins_unchanged_at_end'] is True and actual['active'] is None
            pins[str(ACTUAL)] = ACTUAL_SHA
            source = HERE / 'revisions/work-full-v1/qualify_work.py'
            assert sha(source) == actual['pins'][str(source)]
            sys.dont_write_bytecode = True
            name = 'clojure_additive_javap_qualified_work'
            assert name not in sys.modules
            spec = importlib.util.spec_from_file_location(name, source)
            work = importlib.util.module_from_spec(spec)
            sys.modules[name] = work
            spec.loader.exec_module(work)
            assert Path(work.__file__).resolve() == source
            work.verify(actual['pins'])
            qualified, preparation = work.sealed_preparation(Path(actual['qualified_preparation']['path']),
                actual['qualified_preparation']['sha256'])
            pins.update(actual['pins'])
            service = preparation.module('clojure_additive_javap_service', preparation.SERVICE)
            guard = service.load_guard()
            harness = guard.load_harness()
            runtime_path = Path(actual['qualified_preparation']['path']).with_name('runtime-receipt.json')
            runtime = json.loads(runtime_path.read_text())
            environment = dict(os.environ, **preparation.controlled_environment(Path(runtime['java_home'])))
            javap = Path(runtime['java_home']) / 'bin/javap'
            assert str(javap) in pins and sha(javap) == pins[str(javap)]
            jars = [item['path'] for item in runtime['vendor_jars']]
            rows = {row['id']: row for row in qualified['profiles']}
            review_inputs = {}
            for identity, row in rows.items():
                review_inputs[identity] = dict(source_sha256=row['source_sha256'],
                    binary_sha256=row['binary_sha256'],
                    compiled_input_inventory_sha256=row['compiled_input_inventory_sha256'],
                    compiled_input_inventory=row['compiled_input_inventory'],
                    source_bundle=row['source_bundle'], compiled_source_bundle=row['compiled_source_bundle'])
                if identity in actual['profiles']:
                    measured = actual['profiles'][identity]
                    review_inputs[identity].update(bytecode_evidence=measured['bytecode_evidence'],
                        actual_work_flags=measured['actual_work_flags'], actual_observations=measured['observations'])
            write_new(OUTPUT / 'review-input-index.json', dict(status='verified-qualified-inputs-for-manual-review',
                actual_work_report=dict(path=str(ACTUAL), sha256=ACTUAL_SHA), profiles=review_inputs))
            duplicate = fcntl.fcntl(lock.fileno(), fcntl.F_DUPFD_CLOEXEC, 64)
            assert os.fstat(duplicate).st_ino == os.fstat(lock.fileno()).st_ino
            commands = []
            for identity, role, klass in (
                ('clojure-knucleotide-compact', 'scored-original', 'clbg.knucleotide.compact$histogram$fn__159'),
                ('clojure-knucleotide-compact', 'observer', 'clbg.knucleotideobserver.compact$histogram_raw$fn__250'),
                ('clojure-knucleotide-optimized', 'observer', 'clbg.knucleotideobserver.optimized$histogram_raw$fn__252')):
                if role == 'scored-original':
                    classpath = rows[identity]['runtime_classpath']
                    artifacts = rows[identity]['compiled_artifacts']
                else:
                    inventory_ref = actual['profiles'][identity]['observer_compiled_input_inventory']
                    assert sha(inventory_ref['path']) == inventory_ref['sha256'] == pins[inventory_ref['path']]
                    inventory = json.loads(Path(inventory_ref['path']).read_text())
                    assert inventory['observed_main_namespace'] == 'clbg.knucleotideobserver.' + rows[identity]['variant']
                    classpath = [str(HERE / 'work-qualification/jobs/work-v1/classes' / identity), *jars]
                    artifacts = inventory['generated_class_artifacts']
                relative = klass.replace('.', '/') + '.class'
                path = Path(classpath[0]) / relative
                assert str(path) in artifacts and artifacts[str(path)] == pins[str(path)] == sha(path)
                commands.append((identity + '-' + role + '-nested-histogram', role, klass, classpath, str(path)))
            for klass in ('clojure.lang.PersistentHashMap', 'clojure.lang.PersistentHashMap$TransientHashMap'):
                commands.append(('vendor-' + klass.rsplit('.', 1)[-1], 'pinned-vendor-transient-storage', klass, jars, None))
            for label, role, klass, classpath, artifact in commands:
                command = [str(javap), '-c', '-p', '-cp', os.pathsep.join(classpath), klass]
                service.owned_command(guard, harness, command, environment, OUTPUT, duplicate, 60, label)
                receipt = OUTPUT / (label + '.json')
                record = json.loads(receipt.read_text())
                assert record['command'] == command and record['inherited_common_mutex'] is True
                assert type(record['exit_status']) is type(record['final_exit_status']) is int
                assert record['exit_status'] == record['final_exit_status'] == 0
                assert 'cleanup_error' not in record and 'cleanup_errors' not in record
                terminal['commands'].append(dict(role=role, class_name=klass, class_artifact=artifact,
                    command=command, receipt=str(receipt), stdout=str(OUTPUT / (label + '.stdout'))))
            work.verify(actual['pins'])
            terminal.update(status='actual-additive-nested-class-bytecode-awaiting-semantic-review',
                actual_work_report=dict(path=str(ACTUAL), sha256=ACTUAL_SHA),
                pins_unchanged_at_end=True, native_benchmark_subjects=0, compilation_commands=0)
        except BaseException as error:
            terminal.update(status='failed-additive-class-review', error=str(error) or type(error).__name__,
                error_type=type(error).__name__, traceback=traceback.format_exc())
            raise
        finally:
            for path in OUTPUT.iterdir():
                if path.is_file():
                    pins[str(path)] = sha(path)
            terminal['pins'] = pins
            write_new(OUTPUT / 'terminal.json', terminal)
            if duplicate is not None:
                os.close(duplicate)
            print(json.dumps(dict(status=terminal['status'], receipt=str(OUTPUT / 'terminal.json'),
                sha256=sha(OUTPUT / 'terminal.json'))), flush=True)


if __name__ == '__main__':
    main()
