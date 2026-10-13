"""Additive metadata qualification from already completed Clojure receipts only.

No native, fixture, benchmark, JVM, compiler, server, Git or dataset write.
The exact source/descriptor must be independently accepted and frozen first.
"""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import traceback
import types

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
COHORT = CLBG / 'clojure-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
IMPORTER = CLBG / 'overview/explorer/native_languages_data_v1.py'
IMPORTER_SHA = '27779d09a6cfea98d0377c58d4cdd459fc7f59aac46efdb8f61144070a871dc7'
SUITE = COHORT / 'reconciliation/full-v1-suite-v2.json'
SUITE_SHA = 'c7664909b757e9fff1d736af0b9ba72bdcc47634e264b1b789e986639121a8f4'
COUNTS = COHORT / 'source-counts/qualified-grammar-v3-overlay-restored-v2/qualified-loc-overlay.json'
COUNTS_SHA = 'e5ac3aa1b8d7f054b17cf7f2ef5426f30027d89dad875225d975ba7e5e48902d'
INDEX = COHORT / 'studies/full-v1/index.json'
INDEX_SHA = '7391f432d86a78b0995a1895ff828930642c2eb82f4dbcb36ef86fe97c75b6fd'
REVIEW = CLBG / 'continuation/jobs/completed-native-review-v1/review.json'
REVIEW_SHA = '68f46a27837d3181028751fab0ea67c777b29790f6860833ef09bd9003217b05'
NATIVE = COHORT / 'qualification/private-v2/report.json'
NATIVE_SHA = '69bf7cdb025759e6d10cb8c1ae1c189839e253ced9aca50e4f9cc1e838d36d79'
WORK = COHORT / 'work-qualification/qualified/work-v1-reviewed-v2/report.json'
WORK_SHA = '7e0029f49eacaad3bfd4a3a24e339dccf46ee757ae5ca6db6b062be0bc9b7ebf'
RUNTIME = COHORT / 'qualification/private-v2/runtime-receipt.json'
RUNTIME_SHA = 'a23352ac520614665400ac0f14d5d088cfd0bef9cb4365f25002b21273b3c02a'
JOB = CLBG / 'continuation/owned-jobs/clojure-full90-independent-reconcile-v2'
FLAGS = ['-XX:ActiveProcessorCount=8','-Xms64m','-Xmx8g','-XX:+UseG1GC']
ENVIRONMENT_KEYS = ('CLASSPATH','JAVA_TOOL_OPTIONS','JDK_JAVA_OPTIONS','_JAVA_OPTIONS')


def require(value, message):
    if not value: raise ValueError(message)


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream: json.dump(value,stream,indent=2);stream.write('\n')


def qualify(args, output, pins):
    def bind(path, expected=None):
        path = str(Path(path).resolve(strict=True));value = sha(path)
        require(expected is None or value == expected, 'Changed Clojure evidence: '+path)
        require(path not in pins or pins[path] == value, 'Conflicting Clojure pin: '+path)
        pins[path] = value;return value
    def read(path, expected=None):
        bind(path,expected);return json.loads(Path(path).read_text())
    def verify(values):
        for path,value in values.items():bind(path,value)
    def pinned(path):
        path = Path(path);require(str(path) in pins, 'Unbound original receipt: '+str(path))
        return read(path,pins[str(path)])
    def text(path):
        path = Path(path);require(str(path) in pins, 'Unbound original output: '+str(path))
        bind(path,pins[str(path)]);return path.read_text()
    def owned(path, command):
        value = pinned(path)
        require(value['command'] == command and value['inherited_common_mutex'] is True and
            value['preparation_only'] is value['never_chart_eligible'] is True and
            type(value['exit_status']) is type(value['final_exit_status']) is int and
            value['exit_status'] == value['final_exit_status'] == 0 and
            not any(value.get(key) for key in ('error','error_type','cleanup_error','cleanup_errors',
                'remaining_group_members','output_drain_error')), 'Original owned command not normal')
        return value
    for path in (Path(__file__),HERE / 'descriptor.json',HERE / 'REVIEW.md'):
        require(not path.stat().st_mode & 0o222, 'Reviewed frozen Clojure packet required')
    bind(__file__,args.qualifier_sha256);bind(HERE / 'REVIEW.md',args.review_sha256)
    descriptor = read(HERE / 'descriptor.json',args.descriptor_sha256)
    require(descriptor['language'] == 'clojure' and descriptor['execution'] == 'JIT' and
        descriptor['status'] == 'source-reviewed-descriptor-awaiting-guarded-actual-evidence-qualification' and
        all(descriptor[key] is False for key in ('actual_compiler_version_verified','actual_native_architecture_verified','actual_memory_policy_verified')),
        'Descriptor is not the reviewed pending Clojure candidate')
    bind(sys.executable)
    review = read(REVIEW,REVIEW_SHA)
    review_terminal = read(REVIEW.parent / 'terminal.json')
    require(review_terminal['status'] == review['status'] == 'passed-three-completed-suite-receipt-and-pin-review' and
        review_terminal['body_error'] is None and review_terminal['review'] == str(REVIEW) and
        review_terminal['review_sha256'] == REVIEW_SHA and review_terminal['exclusive_common_timing_mutex_held'] is True and
        review_terminal['actual_benchmark_runs'] == review_terminal['actual_fixture_runs'] == 0,
        'Independent completed review terminal differs')
    terminal = read(JOB / 'terminal.json',review['cohorts']['clojure']['terminal_sha256'])
    request,started,worker = (read(JOB / name) for name in ('request.json','command-started.json','worker-started.json'))
    command = ['/opt/homebrew/opt/python@3.14/bin/python3.14','-B','-u',
        str(CLBG / 'continuation/reconcile_native_suite_v2.py'),'--language','clojure','--study-root',str(INDEX.parent),
        '--reviewed-index-sha256',INDEX_SHA,'--output',str(SUITE)]
    require(review['status'] == 'passed-three-completed-suite-receipt-and-pin-review' and
        review['cohorts']['clojure']['suite'] == str(SUITE) and review['cohorts']['clojure']['sha256'] == SUITE_SHA and
        terminal['status'] == 'command-exited-zero-awaiting-report-review' and type(terminal['exit_code']) is int and terminal['exit_code'] == 0 and
        terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and terminal['stop_signals'] == [] and
        not any(terminal.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure')) and
        request['command'] == started['command'] == worker['command'] == command and
        request['cwd'] == started['cwd'] == worker['cwd'] == str(CLBG.parents[1]) and
        request['common_mutex_owned_by_target_command'] is True and request['automatic_retry'] is False and
        started['command_identity_observation_missing'] is False and
        started['command_pid'] == terminal['command_pid'] and started['command_identity'] == terminal['command_identity'] and
        worker['worker_identity'] == terminal['worker_identity'], 'Independent completed Clojure suite closure differs')
    suite,counts,index,native,work,runtime = (read(path,digest) for path,digest in (
        (SUITE,SUITE_SHA),(COUNTS,COUNTS_SHA),(INDEX,INDEX_SHA),(NATIVE,NATIVE_SHA),(WORK,WORK_SHA),(RUNTIME,RUNTIME_SHA)))
    for values in (suite['provenance'],counts['pins'],native['pins'],work['pins']):verify(values)
    require(review['verified_pins'][str(SUITE)] == SUITE_SHA and index['qualified_preparation'] == str(NATIVE) and
        index['qualified_preparation_sha256'] == NATIVE_SHA and index['actual_work_review'] == str(WORK) and
        index['actual_work_review_sha256'] == WORK_SHA and index['samples'] == 3 and index['full_processes'] == 90 and index['chart_admission'] is False,
        'Clojure original suite ancestry differs')
    require(native['status'] == 'bounded-output-ownership-passed-awaiting-actual-work-and-full-review' and
        work['status'] == 'passed-exact-source-compiled-JVM-work-review', 'Original bounded/source-bytecode work review incomplete')
    require(runtime == pinned(COHORT / 'qualification/private-v1/runtime-receipt.json') and
        runtime['status'] == 'qualified-native-arm64-jvm-identity' and runtime['compiler_version'] == '1.12.6' and
        runtime['java_runtime_version'] == '25.0.4.1+1' and runtime['measured_architecture'] == 'arm64' and
        runtime['backend'] == 'JVM bytecode then HotSpot JIT' and runtime['collector'] == 'G1' and runtime['flags'] == FLAGS,
        'Actual original JVM identity differs')
    java,jars = runtime['java'],[item['path'] for item in runtime['vendor_jars']]
    verify({item['path']:item['sha256'] for item in runtime['vendor_jars']})
    identity = COHORT / 'qualification/private-v1/runtime-identity'
    owned(identity / 'java-version.json',[java,'-version'])
    version = text(identity / 'java-version.stdout')+text(identity / 'java-version.stderr')
    require('Temurin-25.0.4.1+1' in version and 'mixed mode, sharing' in version, 'Original JVM version/mixed-mode output absent')
    owned(identity / 'jvm-properties.json',[java,*FLAGS,'-XshowSettings:properties','-version'])
    properties = text(identity / 'jvm-properties.stdout')+text(identity / 'jvm-properties.stderr')
    require(all(value in properties for value in ('os.arch = aarch64','java.runtime.version = 25.0.4.1+1-LTS',
        'java.vendor = Eclipse Adoptium','sun.management.compiler = HotSpot 64-Bit Tiered Compilers')), 'Actual architecture or compiler properties differ')
    expression = '(assert (= "1.12.6" (clojure-version))) (assert (= 8 (.availableProcessors (Runtime/getRuntime)))) (doseq [g (java.lang.management.ManagementFactory/getGarbageCollectorMXBeans)] (println (.getName g)))'
    owned(identity / 'language-collector-identity.json',[java,*FLAGS,'-cp',os.pathsep.join(jars),'clojure.main','-e',expression])
    require(text(identity / 'language-collector-identity.stdout').splitlines() == ['G1 Young Generation','G1 Concurrent GC','G1 Old Generation'],
        'Actual selected collectors differ')
    for name,image in (('java',java),('libjvm',str(Path(runtime['java_home']) / 'lib/server/libjvm.dylib'))):
        root = COHORT / 'qualification/private-v1/runtime-images' / name
        bind(image,pins[image]);owned(root / 'architecture.json',['/usr/bin/lipo','-archs',image])
        require(text(root / 'architecture.stdout').split() == ['arm64'], 'Original JVM native slice differs')
        owned(root / 'mach-header.json',['/usr/bin/otool','-hv',image])
        require('ARM64' in text(root / 'mach-header.stdout'), 'Original JVM native header differs')
        owned(root / 'linked-images.json',['/usr/bin/otool','-L',image])
        images = pinned(root / 'dependency-review.json')
        require(images['binary'] == image and images['architectures'] == ['arm64'], 'Original JVM image review differs')
        require([item['install_name'] for item in images['dependencies']] ==
            [line.strip().split(' (',1)[0] for line in text(root / 'linked-images.stdout').splitlines()[1:]], 'Original linked-image closure differs')
        for item in images['dependencies']:
            if item.get('scope') == 'OS-image-not-hashed':
                require(item['install_name'].startswith(('/usr/lib/','/System/Library/')), 'Unexpected unpinned runtime dependency')
            else: bind(item['path'],item['sha256'])
    originals = {row['id']:row for row in native['profiles']};profiles = {}
    require(len(originals) == len(native['profiles']) == 20, 'Original native20 inventory incomplete')
    for benchmark,evidence in suite['benchmarks'].items():
        require(index['benchmarks'][benchmark] == dict(manifest=evidence['manifest'],sha256=evidence['manifest_sha256']), 'Original manifest identity differs')
        manifest,report = pinned(evidence['manifest']),pinned(evidence['report'])
        require(report['status'] == 'complete' and report['active'] is None and report['pins_unchanged_at_end'] is True, 'Original full benchmark incomplete')
        verify(manifest['pins'])
        for row in evidence['profiles'].values():
            if row['language'] != 'clojure': continue
            original = originals[row['id']]
            require(all(row[key] == original[key] for key in ('source','source_sha256','binary_sha256','command','runtime_environment','source_bundle')),
                'A measured source/command was relabelled')
            require(row['command'] == [java,*FLAGS,'-cp',os.pathsep.join(original['runtime_classpath']),original['namespace'],'{n}'] and
                original['runtime_classpath'][1:] == jars and all(row['runtime_environment'].get(key) == '' for key in ENVIRONMENT_KEYS) and
                row['runtime_environment']['JAVA_HOME'] == runtime['java_home'], 'Original scored JVM flags/classpath/environment differs')
            inventory = pinned(original['compiled_input_inventory']);build = owned(original['compile_receipt'],inventory['build_command'])
            require(inventory['status'] == 'compiled-exact-authored-input-inventory' and inventory['profile'] == row['id'] and
                inventory['source_bundle'] == row['source_bundle'] and inventory['compiled_artifacts'] == original['compiled_artifacts'] and
                inventory['runtime_classpath'] == original['runtime_classpath'] and inventory['compiler_jar'] == jars[0] and
                inventory['compiler_sha256'] == runtime['vendor_jars'][0]['sha256'], 'Original compiled application/bytecode inventory differs')
            require(build['command'][:6] == [java,*FLAGS,'-cp'] and build['command'][-3:-1] == ['clojure.main','-e'] and
                "(compile 'clbg.support)" in build['command'][-1] and "(compile '"+original['namespace']+")" in build['command'][-1] and
                '{:direct-linking true}' in build['command'][-1], 'Original compile route differs')
            verify(inventory['compiled_artifacts'])
            require(inventory['compiled_artifacts'] and all(Path(path).suffix == '.class' for path in inventory['compiled_artifacts']), 'Original class artifacts missing')
            expected_inputs = {(item['path'],item['sha256']) for item in counts['compiler_bound_input_inventory'][row['id']]['files']}
            require(expected_inputs == {(item['path'],item['sha256']) for item in row['source_bundle']} and len(expected_inputs) == 2,
                'Canonical whole authored main/support bundle differs')
            profiles[row['id']] = dict(command=row['command'],binary_sha256=row['binary_sha256'],source_sha256=row['source_sha256'],
                original_compile_receipt=original['compile_receipt'],compiler_input_inventory=original['compiled_input_inventory'],
                class_artifact_count=len(inventory['compiled_artifacts']),collector='G1',actual_native_architecture='arm64')
    require(len(profiles) == 20, 'Incomplete measured Clojure runtime projection')
    metadata = deepcopy(descriptor)
    metadata.update(status='reviewed-actual-native-language-metadata',actual_compiler_version_verified=True,
        actual_native_architecture_verified=True,actual_memory_policy_verified=True,memory_strategies=['gc'],
        scored_native_architecture='arm64',emulation_used=False,actual_runtime_evidence=profiles,pins=dict(pins))
    template = metadata['measured_backend_template'];template['verified'] = True
    template['runtime_receipt'].update(verified=True,report=str(RUNTIME),report_sha256=RUNTIME_SHA)
    require(template['runtime_receipt']['value'] == runtime['backend'], 'Runtime display binding differs')
    bind(IMPORTER,IMPORTER_SHA);require('clojure_metadata_importer_v1' not in sys.modules, 'Cached unreviewed importer')
    raw = IMPORTER.read_bytes();require(hashlib.sha256(raw).hexdigest() == IMPORTER_SHA, 'Changed importer bytes')
    module = types.ModuleType('clojure_metadata_importer_v1');module.__file__ = str(IMPORTER)
    sys.modules[module.__name__] = module;exec(compile(raw,str(IMPORTER),'exec'),module.__dict__)
    require(len(module.validate_cohort('clojure',suite,counts,metadata)) == 20, 'Original full20/90/3/LOC validator rejected')
    verify(dict(pins));metadata['pins'] = dict(pins);save(output / 'metadata.json',metadata)


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','qualifier-sha256','descriptor-sha256','review-sha256'):parser.add_argument('--'+name,required=True)
    args = parser.parse_args();require(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision), 'Fresh revision required')
    require(all(re.fullmatch('[0-9a-f]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256')), 'Exact reviewed digest required')
    print('WAIT closed Clojure reconciliation lease and common mutex: metadata only',flush=True)
    with (JOB / 'owner.lock').open('r+') as dependency,LOCK.open('r+') as common:
        fcntl.flock(dependency,fcntl.LOCK_EX);held,named = os.fstat(dependency.fileno()),(JOB / 'owner.lock').stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different closed reconciliation lease')
        fcntl.flock(common,fcntl.LOCK_EX);held,named = os.fstat(common.fileno()),LOCK.stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino) and held.st_ino == 79859134, 'Different common mutex')
        output = HERE / 'qualified' / args.revision;output.mkdir(parents=True,exist_ok=False)
        pins = {};outcome = dict(status='failed-Clojure-publication-metadata',chart_admission=False,native_subjects=0,fixture_reexecutions=0)
        try:
            qualify(args,output,pins);outcome['status'] = 'metadata-qualified-awaiting-root-current-candidate-review'
        except BaseException as error:
            outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            try:(output / 'actual-error.log').write_text(traceback.format_exc())
            except BaseException as journal_error:outcome['error_log_write_failure'] = str(journal_error)
            raise
        finally:
            available,errors = {},[]
            for path,value in pins.items():
                try:require(sha(path) == value, 'Changed prerequisite at close')
                except BaseException as error:errors.append(dict(path=path,error=str(error)))
            for path in output.rglob('*'):
                if path.is_file():
                    try:available[str(path)] = sha(path)
                    except BaseException as error:errors.append(dict(path=str(path),error=str(error)))
            if errors:outcome.update(status='failed-terminal-integrity',integrity_errors=errors,available_bytes_not_fully_pinned=True)
            outcome.update(prerequisite_pins=pins,available_evidence_pins=available,actual_common_mutex_held=True)
            save(output / 'outcome.json',outcome)
            if errors:raise RuntimeError('Clojure metadata terminal integrity failed')


if __name__ == '__main__':main()
