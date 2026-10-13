"""Unexecuted Clojure actual-work controller candidate.

Metadata-only seal, then one reviewed native observer qualification. A separate
review/admission creates a new qualified receipt; original raw outcomes and
scored sources/classes stay immutable. Every operation owns the common mutex.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import traceback

HERE = Path(__file__).resolve().parents[2]
PACKET = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
PREPARATION = HERE / 'revisions/private-v2/guarded_prepare.py'
QUALIFIED_REPORT = HERE / 'qualification/private-v2/report.json'
QUALIFIED_SHA = '69bf7cdb025759e6d10cb8c1ae1c189839e253ced9aca50e4f9cc1e838d36d79'
QUALIFIED_PROPOSAL = HERE / 'proposals/private-v2.json'
QUALIFIED_PROPOSAL_SHA = 'c234215757179d3fe51f047d041df594fc31d0e589515bb88af9669cab697ad3'
SUBJECTS = ('binarytrees', 'knucleotide', 'pidigits')
OBSERVER_FLAGS = ['--add-opens=java.base/java.util=ALL-UNNAMED']
STATIC_FILES = ('qualify_work.py', 'observer.clj', 'instrument_sources.py', 'native_fd_observer.py',
    'work_proofs.py', 'test_work_proofs.py', 'work_fixture_runner.py', 'test_native_fd_observer.py',
    'full_study.py', 'seal_full_studies.py', 'test_full_study.py', 'REVIEW.md')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def module(name, path):
    assert name not in sys.modules, 'Fresh pinned module identity'
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    sys.modules[name] = value
    spec.loader.exec_module(value)
    assert Path(value.__file__).resolve() == Path(path).resolve()
    return value


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def merge(pins, values):
    for path, digest in values.items():
        assert path not in pins or pins[path] == digest, ('Conflicting input pin', path)
        pins[path] = digest


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Reviewed source/evidence changed', path)


def sealed_preparation(path, digest):
    assert path.resolve() == QUALIFIED_REPORT and digest == QUALIFIED_SHA
    assert sha(path) == digest
    report = json.loads(path.read_text())
    assert report['status'] == 'bounded-output-ownership-passed-awaiting-actual-work-and-full-review'
    assert report['pins_unchanged_at_end'] is True and report['active'] is None
    assert report['full_performance_runs'] == report['full_manifests_sealed'] == 0
    assert report['chart_admission'] is report['canonical_LOC_admitted'] is False
    assert report['no_automatic_retry'] is True
    assert report['fresh_profile_builds'] == 16 and report['reused_profiles'] == 4
    assert report['reused_bounded_cases'] == 24 and report['fresh_native_attempts'] == 132
    assert report['old_cases_relabelled_as_new_attempts'] is False
    verify(report['pins'])
    assert str(PREPARATION) in report['pins']
    preparation = module('clojure_work_qualified_preparation', PREPARATION)
    reuse_source = PREPARATION.with_name('proof_reuse.py')
    assert str(reuse_source) in report['pins'] and str(preparation.SERVICE) in report['pins']
    reuse_reader = module('clojure_work_qualified_reuse', reuse_source)
    service = preparation.module('clojure_work_qualified_service', preparation.SERVICE)
    guard = service.load_guard()
    harness = guard.load_harness()
    assert report['proposal_sha256'] == QUALIFIED_PROPOSAL_SHA
    assert report['pins'][str(QUALIFIED_PROPOSAL)] == QUALIFIED_PROPOSAL_SHA
    assert sha(QUALIFIED_PROPOSAL) == QUALIFIED_PROPOSAL_SHA
    proposal = json.loads(QUALIFIED_PROPOSAL.read_text())
    assert proposal['status'] == 'sealed-awaiting-root-review'
    assert proposal['intended_total_bounded_cases'] == 156 and proposal['fresh_bounded_cases'] == 132
    assert proposal['reused_profiles'] == 4 and proposal['reused_bounded_cases'] == 24
    verify(proposal['pins'])
    reuse_path = Path(report['prior_proof_reuse'])
    assert reuse_path == HERE / 'proposal-seals/private-v2/proof-reuse.json'
    assert report['pins'][str(reuse_path)] == report['prior_proof_reuse_sha256'] == sha(reuse_path)
    assert proposal['prior_proof_reuse'] == str(reuse_path)
    assert proposal['prior_proof_reuse_sha256'] == report['prior_proof_reuse_sha256']
    retained = json.loads(reuse_path.read_text())
    assert retained == reuse_reader.reconstruct(preparation, service, guard, harness)
    rows = {row['id']: row for row in report['profiles']}
    declared = {row['id']: row for row in preparation.profiles(proposal)}
    assert len(rows) == len(report['profiles']) == 20 and set(rows) == set(declared)
    assert set(retained['profiles']) == set(reuse_reader.IDENTITIES)
    assert report['attempts'][:24] == retained['attempts'] and len(report['attempts']) == 156
    assert report['runtime_receipt_origin'] == retained['runtime_receipt']
    runtime_path = path.with_name('runtime-receipt.json')
    assert report['pins'][str(runtime_path)] == retained['runtime_receipt_sha256'] == sha(runtime_path)
    assert runtime_path.read_bytes() == Path(retained['runtime_receipt']).read_bytes()
    runtime = retained['runtime']
    java = Path(runtime['java'])
    jars = [Path(item['path']) for item in runtime['vendor_jars']]
    for identity, row in rows.items():
        origin = 'private-v1' if identity in retained['profiles'] else 'private-v2'
        assert row['bounded_origin'] == ('retained-private-v1' if origin == 'private-v1' else 'fresh-private-v2')
        if origin == 'private-v1':
            assert row == retained['profiles'][identity]
        assert row['compiled_source_bundle'] == declared[identity]['compiled_source_bundle']
        assert row['namespace'] == declared[identity]['namespace'] and row['benchmark'] == declared[identity]['benchmark']
        package = HERE / '.build' / origin / 'packages' / identity
        classes = HERE / '.build' / origin / 'classes' / identity
        assert row['source'] == str(package / ('sources/' + row['namespace'].replace('.', '/') + '.clj'))
        assert row['runtime_classpath'] == [str(classes), *map(str, jars)]
        assert row['binary'] == str(java) and row['binary_sha256'] == report['pins'][str(java)]
        assert row['command'] == preparation.runtime_command(java, classes, jars, row['namespace'])
        assert row['runtime_environment'] == preparation.controlled_environment(Path(runtime['java_home']))
        assert row['maximum_compute_workers'] == declared[identity]['maximum_compute_workers'] <= 8
        assert len(row['source_bundle']) == 2
        assert [item['role'] for item in row['source_bundle']] == ['application', 'application-helper']
        for item, authored in zip(row['source_bundle'], row['compiled_source_bundle']):
            assert report['pins'][item['path']] == item['sha256'] == report['pins'][str(HERE / authored)]
        assert row['source_sha256'] == row['source_bundle'][0]['sha256']
        inventory_path = Path(row['compiled_input_inventory'])
        assert inventory_path == HERE / 'qualification' / origin / 'builds' / identity / 'compiler-input-inventory.json'
        assert report['pins'][str(inventory_path)] == row['compiled_input_inventory_sha256'] == sha(inventory_path)
        inventory = json.loads(inventory_path.read_text())
        command = preparation.compile_command(java, package, classes, jars, row['namespace'])
        assert inventory['status'] == 'compiled-exact-authored-input-inventory' and inventory['profile'] == identity
        assert inventory['source_bundle'] == row['source_bundle'] and inventory['build_command'] == command
        assert inventory['compiled_artifacts'] == row['compiled_artifacts'] and inventory['runtime_classpath'] == row['runtime_classpath']
        assert row['compiled_artifacts'] and all(report['pins'][name] == digest and Path(name).suffix == '.class'
            for name, digest in row['compiled_artifacts'].items())
        compile_path = Path(row['compile_receipt'])
        assert compile_path == inventory_path.with_name('compile.json')
        assert report['pins'][str(compile_path)] == row['compile_receipt_sha256'] == sha(compile_path)
        reuse_reader.receipt(compile_path, command)
    expected = {(identity, case) for identity, row in rows.items() for case in proposal['case_inventory'][row['benchmark']]}
    actual = {(item['id'], item['case']) for item in report['attempts']}
    assert len(expected) == len(actual) == 156 and expected == actual
    for ordinal, attempt in enumerate(report['attempts'], 1):
        preparation.exact_true_checks(attempt)
        reuse_reader.observations(attempt)
        assert type(attempt['attempt']) is int and attempt['attempt'] == ordinal
        row = rows[attempt['id']]
        assert attempt['benchmark'] == row['benchmark'] and attempt['key'] == row['key'] and attempt['language'] == 'clojure'
        assert attempt['source_sha256'] == row['source_sha256'] and attempt['cwd'] == row['cwd']
        assert attempt['runtime_environment'] == row['runtime_environment']
        assert attempt['execution_fingerprint'] == harness.execution_fingerprint(row)
        assert attempt['command'] == [part.replace('{n}', str(attempt['workload_n'])) for part in row['command']]
        native = attempt['supervised_native_group']
        assert native['type'] == 'native-group-started' and type(native['pid']) is type(native['pgid']) is int
        assert native['pid'] > 0 and native['pgid'] == native['pid'] and native['command'] == attempt['command']
        assert native['inherited_common_mutex'] is True
        origin = HERE / 'qualification' / ('private-v1' if ordinal <= 24 else 'private-v2')
        raw = origin / 'attempts' / ('%03d-%s.json' % (ordinal, attempt['id']))
        enriched_path = raw.with_suffix('.supervisor.json')
        assert attempt['result_path'] == str(raw) and attempt['supervisor_result_path'] == str(enriched_path)
        assert str(raw) in report['pins'] and str(enriched_path) in report['pins']
        sampled, enriched = json.loads(raw.read_text()), json.loads(enriched_path.read_text())
        reuse_reader.observations(sampled); reuse_reader.observations(enriched)
        assert 'attempt' not in sampled
        assert type(enriched['attempt']) is int and enriched['attempt'] == ordinal
        assert type(enriched['sampler_wait_status']) is int and enriched['sampler_wait_status'] == 0
        for key in reuse_reader.OBSERVATIONS:
            assert sampled[key] == enriched[key] == attempt[key]
        for key in ('qualified', 'qualification_checks', 'supervised_native_group', 'sampler_wait_status'):
            assert enriched[key] == attempt[key]
        stdout = origin / attempt['id'] / attempt['case'] / 'actual.stdout'
        assert report['pins'][str(stdout)] == attempt['stdout_sha256'] == sha(stdout)
        assert stdout.stat().st_size == attempt['stdout_bytes']
        assert attempt['validation']['reference_match'] is True
        assert attempt['validation']['bytes'] == attempt['stdout_bytes'] and attempt['validation']['sha256'] == attempt['stdout_sha256']
    return report, preparation


def seal(args, lock):
    assert not args.proposal_file.exists()
    qualified, preparation = sealed_preparation(args.qualified_file, args.qualified_sha256)
    output = HERE / 'work-qualification/proposal-seals' / args.revision
    assert not output.exists(), 'Exclusive metadata revision'
    snapshot = output / 'snapshot'
    snapshot.mkdir(parents=True)
    pins = dict(qualified['pins'])
    merge(pins, {str(args.qualified_file): args.qualified_sha256})
    for name in STATIC_FILES:
        source, target = PACKET / name, snapshot / name
        shutil.copyfile(source, target)
        assert sha(source) == sha(target)
        merge(pins, {str(source): sha(source), str(target): sha(target)})
        target.chmod(0o444)
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(snapshot))
    fixtures = module('clojure_work_sealed_fixture_runner', snapshot / 'work_fixture_runner.py')
    receipts = fixtures.run_locked(snapshot, output, pins, lock.fileno())
    editor = module('clojure_work_sealed_instrumentation', snapshot / 'instrument_sources.py')
    subjects = []
    original_proposals = [path for path, digest in qualified['pins'].items()
        if digest == qualified['proposal_sha256'] and Path(path).is_relative_to(HERE / 'proposals')]
    assert len(original_proposals) == 1
    for row in qualified['profiles']:
        if row['benchmark'] not in SUBJECTS:
            continue
        original_main = Path(row['source'])
        original_support = Path(row['source_bundle'][1]['path'])
        observed_main, main_edits = editor.main(original_main.read_text(), row['benchmark'], row['variant'])
        observed_support, support_edits = editor.support(original_support.read_text())
        package = output / 'packages' / row['id']
        observed_namespace = 'clbg.' + row['benchmark'] + 'observer.' + row['variant']
        main = package / ('sources/' + observed_namespace.replace('.', '/') + '.clj')
        support = package / 'support/clbg/work/support.clj'
        observer = package / 'support/clbg/work/observer.clj'
        for path, value in ((main, observed_main), (support, observed_support)):
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('x') as stream:
                stream.write(value)
        observer.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(snapshot / 'observer.clj', observer)
        files = [dict(path=str(path), sha256=sha(path), role=role) for path, role in (
            (main, 'observed-application'), (support, 'observed-application-helper'), (observer, 'generic-object-and-operand-observer'))]
        edit_path = output / 'edits' / (row['id'] + '.json')
        new_json(edit_path, dict(status='exact-text-observer-edits-awaiting-root-review', profile=row['id'],
            scored_source=str(original_main), scored_source_sha256=row['source_sha256'],
            scored_helper=str(original_support), scored_helper_sha256=row['source_bundle'][1]['sha256'],
            original_compiled_input_inventory=row['compiled_input_inventory'],
            original_compiled_input_inventory_sha256=row['compiled_input_inventory_sha256'],
            original_compiled_artifacts=row['compiled_artifacts'], main_edits=main_edits, support_edits=support_edits,
            observer_source_bundle=files,
            original_tree_creator_checker_invoked=(row['benchmark'] == 'binarytrees'),
            original_tree_creator_checker_copy_removed=(row['benchmark'] == 'binarytrees'),
            original_tree_compiled_namespace=row['namespace'] if row['benchmark'] == 'binarytrees' else None,
            observer_command_only_extra_flags=OBSERVER_FLAGS, scored_command_unchanged=True))
        merge(pins, {file['path']: file['sha256'] for file in files})
        merge(pins, {str(edit_path): sha(edit_path)})
        subjects.append(dict(original=copy.deepcopy(row), package=str(package), observer_source_bundle=files,
            source=str(main), source_sha256=sha(main), namespace=observed_namespace,
            edit_inventory=dict(path=str(edit_path), sha256=sha(edit_path))))
    assert len(subjects) == 6
    verify(pins)
    new_json(args.proposal_file, dict(status='metadata-sealed-awaiting-root-review',
        qualified_preparation=dict(path=str(args.qualified_file), sha256=args.qualified_sha256),
        first_proposal=dict(path=original_proposals[0], sha256=qualified['proposal_sha256']),
        subjects=subjects, pins=pins, snapshot=str(snapshot), source_only_fixture_receipts=receipts,
        observer_only_runtime_flags=OBSERVER_FLAGS, native_processes=0, full_performance_runs=0,
        no_automatic_retry=True, chart_admission=False))
    print(json.dumps(dict(proposal=str(args.proposal_file), sha256=sha(args.proposal_file))), flush=True)


def bytecode_audits(service, guard, harness, row, observer, runtime, environment, output, fd):
    javap = Path(runtime['java_home']) / 'bin/javap'
    records = []
    for role, profile in (('scored-original', row), ('observer', observer)):
        names = (['make_tree', 'check_tree'] if role == 'scored-original' else
                 ['run_scored'] + (['interval'] if row['variant'] == 'optimized' else [])) if row['benchmark'] == 'binarytrees' else (
                 ['histogram'] if row['benchmark'] == 'knucleotide' else ['_main'])
        for name in names:
            if role == 'observer' and name == 'histogram':
                name = 'histogram_raw'
            if role == 'observer' and name == '_main':
                name = 'run_scored'
            klass = profile['namespace'] + '$' + name
            command = [str(javap), '-c', '-p', '-cp', os.pathsep.join(profile['runtime_classpath']), klass]
            label = role + '-' + name
            service.owned_command(guard, harness, command, environment, output, fd, 60, label)
            records.append(dict(role=role, class_name=klass, command=command,
                stdout=str(output / (label + '.stdout')), receipt=str(output / (label + '.json'))))
    if row['benchmark'] == 'binarytrees':
        label = 'vendor-RT-object-array'
        command = [str(javap), '-c', '-p', '-cp', os.pathsep.join(runtime['vendor_jar_paths']), 'clojure.lang.RT']
        service.owned_command(guard, harness, command, environment, output, fd, 60, label)
        records.append(dict(role='pinned-vendor-allocation-primitive', class_name='clojure.lang.RT',
            command=command, stdout=str(output / (label + '.stdout')), receipt=str(output / (label + '.json'))))
    path = output / 'bytecode-evidence.json'
    new_json(path, dict(status='actual-bytecode-evidence-awaiting-semantic-review', profile=row['id'],
        scored_source_sha256=row['source_sha256'], scored_compiled_artifacts=row['compiled_artifacts'],
        observer_source_sha256=observer['source_sha256'], observer_compiled_artifacts=observer['compiled_artifacts'],
        actual_commands=records, no_original_creator_checker_source_replacement=True,
        tree_observer_uses_original_compiled_namespace=row['namespace'] if row['benchmark'] == 'binarytrees' else None,
        observer_contains_no_copied_tree_creator_checker=(row['benchmark'] == 'binarytrees')))
    return dict(path=str(path), sha256=sha(path))


def aggregate(flags, names):
    return {name: any(item[name] is True for item in flags) for name in names}


def run(args, lock):
    assert sha(args.proposal_file) == args.proposal_sha256
    proposal = json.loads(args.proposal_file.read_text())
    assert proposal['status'] == 'metadata-sealed-awaiting-root-review' and proposal['no_automatic_retry'] is True
    verify(proposal['pins'])
    reference = proposal['qualified_preparation']
    qualified, preparation = sealed_preparation(Path(reference['path']), reference['sha256'])
    service = preparation.module('clojure_work_frozen_owned_service', preparation.SERVICE)
    guard = service.load_guard()
    harness = guard.load_harness()
    harness.install_interrupt_handlers()
    frozen = Path(proposal['snapshot'])
    decoder = module('clojure_work_frozen_proofs', frozen / 'work_proofs.py')
    descriptor = module('clojure_work_frozen_fd_observer', frozen / 'native_fd_observer.py')
    output = HERE / 'work-qualification/jobs' / args.revision
    assert not output.exists(), 'No duplicate qualification or implicit retry'
    output.mkdir(parents=True)
    guard.OUT, guard.REPORT = output, output / 'report.json'
    runtime = json.loads(Path(reference['path']).with_name('runtime-receipt.json').read_text())
    runtime['vendor_jar_paths'] = [row['path'] for row in runtime['vendor_jars']]
    java, jars = Path(runtime['java']), [Path(path) for path in runtime['vendor_jar_paths']]
    original_environment = preparation.controlled_environment(Path(runtime['java_home']))
    compiler_environment = dict(os.environ, **original_environment)
    references = preparation.module('clojure_work_independent_references', preparation.REFERENCE)
    dna = preparation.module('clojure_work_independent_DNA', preparation.DNA_REFERENCE)
    references.knucleotide = dna.knucleotide
    pins = dict(proposal['pins'])
    merge(pins, {str(args.proposal_file): args.proposal_sha256})
    report = dict(status='qualifying-actual-work', attempts=[], active=None, profiles={}, pins=pins,
        qualified_preparation=reference, full_performance_runs=0, canonical_LOC_admitted=False, chart_admission=False)
    guard.save(guard.REPORT, report)
    duplicate = fcntl.fcntl(lock.fileno(), fcntl.F_DUPFD_CLOEXEC, 64)
    actual_fd_reference = None
    try:
        assert os.fstat(duplicate).st_ino == os.fstat(lock.fileno()).st_ino
        for declared in proposal['subjects']:
            original = declared['original']
            identity, benchmark, variant = original['id'], original['benchmark'], original['variant']
            package = Path(declared['package'])
            classes = output / 'classes' / identity
            classes.mkdir(parents=True)
            original_class_dependencies = original['runtime_classpath'][:1] if benchmark == 'binarytrees' else []
            compile_command = preparation.compile_command(java, package, classes, jars, declared['namespace'])
            expression = compile_command[-1].replace('(compile \'clbg.support)', '(compile \'clbg.work.observer) (compile \'clbg.work.support)')
            assert expression != compile_command[-1]
            compile_command[-1] = expression
            compile_command[compile_command.index('-cp') + 1] = os.pathsep.join(
                map(str, [package / 'sources', package / 'support', *original_class_dependencies, *jars]))
            service.owned_command(guard, harness, compile_command, compiler_environment,
                output / 'builds' / identity, duplicate, 180, 'compile-observer')
            artifacts = service.pin_tree(classes)
            assert artifacts and all(Path(path).suffix == '.class' for path in artifacts)
            merge(pins, artifacts)
            compiler_inputs = output / 'builds' / identity / 'compiled-input-inventory.json'
            new_json(compiler_inputs, dict(status='qualified-exact-observer-compiler-inputs',
                actual_compile_command=compile_command,
                actual_compile_receipt=str(output / 'builds' / identity / 'compile-observer.json'),
                authored_observer_source_bundle=declared['observer_source_bundle'],
                original_scored_compiled_dependency_roots=original_class_dependencies,
                original_scored_compiled_dependencies=original['compiled_artifacts'] if benchmark == 'binarytrees' else {},
                original_scored_compiled_input_inventory=original['compiled_input_inventory'],
                original_scored_compiled_input_inventory_sha256=original['compiled_input_inventory_sha256'],
                vendor_jars=runtime['vendor_jars'], generated_class_artifacts=artifacts,
                observed_main_namespace=declared['namespace'], observed_helper_namespace='clbg.work.support',
                no_original_authored_source_directory_on_classpath=True))
            merge(pins, {str(compiler_inputs): sha(compiler_inputs)})
            runtime_classpath = [str(classes), *original_class_dependencies, *map(str, jars)]
            observer = dict(original, id=identity + '-work-observer', source=declared['source'],
                namespace=declared['namespace'],
                source_sha256=declared['source_sha256'], source_bundle=declared['observer_source_bundle'],
                compiled_artifacts=artifacts, runtime_classpath=runtime_classpath,
                compiled_input_inventory=str(compiler_inputs), compiled_input_inventory_sha256=sha(compiler_inputs),
                command=[str(java), *preparation.JVM_FLAGS, *OBSERVER_FLAGS, '-cp', os.pathsep.join(runtime_classpath), declared['namespace'], '{n}'],
                maximum_workers=original['maximum_compute_workers'])
            bytecode = bytecode_audits(service, guard, harness, original, observer, runtime,
                compiler_environment, output / 'bytecode' / identity, duplicate)
            cases = [('allocated-n' + str(n), n, None) for n in (0, 7, 9)] if benchmark == 'binarytrees' else (
                [('digits-97', 97, None), ('zero-digits', 0, None)] if benchmark == 'pidigits' else
                [(name, harness.WORK[benchmark][0], data) for name, data in preparation.input_cases(service, references, benchmark)])
            observations, flag_rows = [], []
            for case, n, data in cases:
                directory = output / observer['id'] / case
                trace = directory / 'actual-work.jsonl'
                observed = dict(observer, runtime_environment=dict(original_environment,
                    CLBG_WORK_TRACE=str(trace), CLBG_WORK_BENCHMARK=benchmark, CLBG_WORK_VARIANT=variant, CLBG_WORK_N=str(n)))
                native_command = [part.replace('{n}', str(n)) for part in observed['command']]
                # One long actual tree witness establishes native JVM retention;
                # zero-digit/empty cases need no extra process lifetime padding.
                witness = benchmark == 'binarytrees' and variant == 'compact' and n == 9
                fd_proof, restore = descriptor.install(harness, native_command, duplicate,
                    output / 'native-fd' / identity / case) if witness else (None, lambda: None)
                def validate_work(actual_directory):
                    events = [json.loads(line) for line in trace.read_text().splitlines()]
                    assert events and events[-1]['kind'] == 'document'
                    document = events[-1]['value']
                    decoder.event_order(events, document, benchmark, variant)
                    flags = decoder.trees(document, n, variant) if benchmark == 'binarytrees' else (
                        decoder.pi(document, n) if benchmark == 'pidigits' else decoder.maps(document, dna.target_sequence(data), variant))
                    if witness:
                        # The immutable sampler forks. Its observed Popen and
                        # thread mutate child memory; consume the actual child
                        # record rather than the parent's untouched proof dict.
                        raw_path = output / 'attempts' / ('%03d-%s.json' % (len(report['attempts']) + 1, observed['id']))
                        child = json.loads(raw_path.read_text())
                        fd_proof.clear()
                        fd_proof.update(child['bounded_mutex_fd_evidence'])
                        fd_proof['actual_sampler_raw_result'] = dict(path=str(raw_path), sha256=sha(raw_path))
                        assert fd_proof['native_observed'] is True and fd_proof['errors'] == []
                        assert fd_proof['actual_native_pass_fds_contains_common_descriptor'] is True
                    proof_path = actual_directory / 'work-proof.json'
                    new_json(proof_path, dict(status='passed', flags=flags, actual_ordered_trace=str(trace),
                        actual_ordered_trace_sha256=sha(trace), native_fd_evidence=fd_proof,
                        source_sha256=observed['source_sha256'], original_scored_source_sha256=original['source_sha256']))
                    flag_rows.append(flags)
                    return dict(status='passed', flags=flags, path=str(proof_path), sha256=sha(proof_path))
                try:
                    expected = getattr(references, benchmark)(n if data is None else data)
                    attempt = service.probe(guard, harness, observed, n, data, expected,
                        pins, output, duplicate, report, case, trace=validate_work)
                    preparation.exact_true_checks(attempt)
                finally:
                    restore()
                if witness:
                    assert fd_proof['native_observed'] is True and fd_proof['errors'] == []
                    observed_fd = fd_proof['observations'][0]
                    detail = json.loads(Path(observed_fd['receipt']).read_text())
                    native = attempt['supervised_native_group']
                    mutex_document = dict(native_pid=observed_fd['native_pid'], native_pgid=observed_fd['native_pgid'],
                        inherited_fd=duplicate, authoritative_native_identity=native,
                        native_command=observed_fd['native_command'], reviewed_command=native_command,
                        observer_owned_group=detail['owned_observer_pid'] > 0,
                        observer_exit_status=detail['exit_status'], observer_reaped=detail.get('owned_observer_reaped') is True,
                        observer_remaining_group=detail['remaining_group'], observer_cleanup_errors=detail['cleanup_errors'],
                        observer_signalled_foreign_group=False, supervisor_common_device_inode=fd_proof['common_device_inode'],
                        actual_native_open_files=[dict(pid=row['pid'], fd=file['fd'], path=file.get('path'),
                            device_inode=[file.get('device'), file.get('inode')]) for row in observed_fd['actual_files'] for file in row['files']],
                        actual_inherited_description_verified=fd_proof['actual_native_pass_fds_contains_common_descriptor'] is True)
                    mutex_flags = decoder.native_mutex(mutex_document)
                    actual_fd_path = output / 'native-mutex-fd-proof.json'
                    new_json(actual_fd_path, dict(status='passed', actual_native_binary=original['binary'],
                        actual_native_binary_sha256=original['binary_sha256'], actual_native_pid=observed_fd['native_pid'],
                        actual_native_pgid=observed_fd['native_pgid'], inherited_descriptor=duplicate,
                        **mutex_flags,
                        actual_direct_command=native_command, actual_source_work_attempt=attempt['attempt'],
                        raw_bounded_descriptor_evidence=fd_proof, reconciled_native_mutex_evidence=mutex_document,
                        inference_basis='Actual exact direct Java Popen inherited this fcntl duplicate of the held common description; retained lsof bytes match native PID, FD, device and inode; no scored/observer source opens the mutex path.',
                        other_case_binding='All other work commands retain the same pinned Java image/flags/vendor runtime and inherit this same duplicate through the unchanged exact-command native guard.'))
                    actual_fd_reference = dict(path=str(actual_fd_path), sha256=sha(actual_fd_path))
                observations.append(dict(status='passed', case=case, attempt=attempt['attempt'],
                    scored_source_sha256=original['source_sha256'], observer_source_sha256=observer['source_sha256'],
                    edit_inventory=declared['edit_inventory'], proof=dict(path=attempt['source_work_proof']['path'], sha256=attempt['source_work_proof']['sha256'])))
            fields = {'binarytrees': {'actual_fresh_node_storage', 'actual_leaf_allocation', 'retained_identity_after_churn', 'all_iteration_graphs_verified', 'worker_cap_verified'},
                'knucleotide': {'all_seven_actual_maps', 'all_windows_and_complete_inventories', 'actual_growth_observed', 'private_workers_and_same_map_handoff'},
                'pidigits': {'both_actual_extraction_divisions', 'accepted_and_rejected_recurrence', 'zero_digits_no_extraction'}}[benchmark]
            flags = aggregate(flag_rows, fields)
            assert all(value is True for value in flags.values()), ('Required actual work not established', identity, flags)
            report['profiles'][identity] = dict(status='actual-work-passed-awaiting-source-bytecode-review',
                source_sha256=original['source_sha256'], binary_sha256=original['binary_sha256'],
                compiled_input_inventory_sha256=original['compiled_input_inventory_sha256'],
                observations=observations, actual_work_flags=flags, bytecode_evidence=bytecode,
                observer_compiled_input_inventory=dict(path=str(compiler_inputs), sha256=sha(compiler_inputs)))
            guard.save(guard.REPORT, report)
        merge(pins, {str(path): sha(path) for path in output.rglob('*') if path.is_file() and path != guard.REPORT})
        assert actual_fd_reference is not None
        verify(pins)
        report.update(status='actual-work-output-ownership-passed-awaiting-source-bytecode-review',
            pins_unchanged_at_end=True, scored_sources_and_classes_unchanged=True,
            first_proposal=proposal['first_proposal'], work_source_proposal=dict(path=str(args.proposal_file), sha256=args.proposal_sha256),
            native_mutex_fd_proof=dict(json.loads(Path(actual_fd_reference['path']).read_text()), **actual_fd_reference),
            proof_parser_fixture_receipt=proposal['source_only_fixture_receipts']['proof_parser'],
            full_pipeline_fixture_receipt=proposal['source_only_fixture_receipts']['full_pipeline'])
        guard.save(guard.REPORT, report)
    except BaseException as error:
        report.update(status='stopped-unqualified', active=None, error=str(error) or type(error).__name__, error_type=type(error).__name__)
        merge(pins, {str(path): sha(path) for path in output.rglob('*') if path.is_file() and path != guard.REPORT})
        guard.save(guard.REPORT, report)
        raise
    finally:
        os.close(duplicate)
    print(json.dumps(dict(report=str(guard.REPORT), sha256=sha(guard.REPORT), status=report['status'])), flush=True)


def admit(args):
    """Add separately reviewed source/actual-bytecode meaning, never rerun work."""
    assert sha(args.native_report) == args.native_report_sha256
    assert sha(args.source_review) == args.source_review_sha256
    actual = json.loads(args.native_report.read_text())
    review = json.loads(args.source_review.read_text())
    assert actual['status'] == 'actual-work-output-ownership-passed-awaiting-source-bytecode-review'
    assert actual['pins_unchanged_at_end'] is True and actual['active'] is None
    assert review['status'] == 'passed-exact-source-actual-bytecode-semantic-review'
    assert review['language'] == 'clojure' and review['review_identity'] and review['unresolved'] == []
    assert review['actual_work_report'] == dict(path=str(args.native_report), sha256=args.native_report_sha256)
    qualified, preparation = sealed_preparation(Path(actual['qualified_preparation']['path']), actual['qualified_preparation']['sha256'])
    pins = dict(actual['pins'])
    merge(pins, {str(args.native_report): args.native_report_sha256, str(args.source_review): args.source_review_sha256})
    verify(pins)
    rows = {row['id']: row for row in qualified['profiles']}
    assert len(rows) == 20 and set(review['profiles']) == set(rows)
    selected = {identity for identity, row in rows.items() if row['benchmark'] in SUBJECTS}
    assert set(actual['profiles']) == selected
    result = copy.deepcopy(actual)
    result['profiles'] = {}
    for identity, original in rows.items():
        semantic = review['profiles'][identity]
        assert semantic['status'] == 'passed' and semantic['review_identity'] and semantic['unresolved'] == []
        assert semantic['source_sha256'] == original['source_sha256'] and semantic['binary_sha256'] == original['binary_sha256']
        assert semantic['compiled_input_inventory_sha256'] == original['compiled_input_inventory_sha256']
        assert semantic['complete_static_source_review'] is True
        row = copy.deepcopy(actual['profiles'].get(identity, {}))
        row.update(status='passed', review_identity=semantic['review_identity'], unresolved=[],
            source_sha256=original['source_sha256'], binary_sha256=original['binary_sha256'],
            compiled_input_inventory_sha256=original['compiled_input_inventory_sha256'], complete_static_source_review=True,
            semantic_source_review=dict(path=str(args.source_review), sha256=args.source_review_sha256))
        if identity in selected:
            assert semantic['bytecode_review']['status'] == 'passed'
            assert {key: semantic['bytecode_review'][key] for key in ('path', 'sha256')} == row['bytecode_evidence']
            assert all(value is True for value in semantic['bytecode_review']['exact_work_checks'].values())
            checks = {'binarytrees': {'original_unconditional_array_allocation_before_depth_test',
                'original_real_recursive_checker', 'pinned_actual_RT_allocation_primitive', 'observer_invokes_actual_original_creator_checker'},
                'knucleotide': {'original_actual_new_storage', 'original_all_seven_widths_full_window_loops', 'original_private_complete_map_return'},
                'pidigits': {'original_two_BigInteger_divide_calls', 'original_both_quotients_feed_equality'}}[original['benchmark']]
            assert set(semantic['bytecode_review']['exact_work_checks']) == checks
            row['bytecode_review'] = copy.deepcopy(semantic['bytecode_review'])
        result['profiles'][identity] = row
    for attempt in result['attempts']:
        preparation.exact_true_checks(attempt)
    verify(pins)
    result.update(status='passed-exact-source-compiled-JVM-work-review', review_identity=review['review_identity'],
        source_bytecode_review=dict(path=str(args.source_review), sha256=args.source_review_sha256),
        unresolved=[], pins=pins, original_native_report_mutated=False, full_performance_runs=0,
        canonical_LOC_admitted=False, chart_admission=False)
    output = HERE / 'work-qualification/qualified' / args.revision / 'report.json'
    assert not output.parent.exists(), 'Exclusive additive reviewed receipt, no overwrite'
    new_json(output, result)
    print(json.dumps(dict(report=str(output), sha256=sha(output), status=result['status'])), flush=True)


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true')
    mode.add_argument('--run', action='store_true')
    mode.add_argument('--admit', action='store_true')
    parser.add_argument('--qualified-file', type=Path)
    parser.add_argument('--qualified-sha256')
    parser.add_argument('--proposal-file', type=Path)
    parser.add_argument('--proposal-sha256')
    parser.add_argument('--native-report', type=Path)
    parser.add_argument('--native-report-sha256')
    parser.add_argument('--source-review', type=Path)
    parser.add_argument('--source-review-sha256')
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    if not args.admit:
        assert args.proposal_file is not None
        args.proposal_file = args.proposal_file.resolve()
        assert args.proposal_file.is_relative_to(HERE / 'work-qualification/proposals')
    for value in (args.qualified_sha256, args.proposal_sha256, args.native_report_sha256, args.source_review_sha256):
        assert value is None or re.fullmatch(r'[0-9a-f]{64}', value)
    if args.seal:
        assert args.qualified_file is not None and args.qualified_sha256
        args.qualified_file = args.qualified_file.resolve(strict=True)
    elif args.run:
        assert args.proposal_sha256
    else:
        assert args.native_report is not None and args.native_report_sha256 and args.source_review is not None and args.source_review_sha256
        args.native_report = args.native_report.resolve(strict=True)
        args.source_review = args.source_review.resolve(strict=True)
        assert args.native_report.is_relative_to(HERE / 'work-qualification/jobs')
        assert args.source_review.is_relative_to(HERE / 'work-qualification/reviews')
    def interrupted(actual, frame):
        raise KeyboardInterrupt('Clojure work controller interrupted ' + str(actual))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    with (PACKET / 'work-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        print('WAIT common mutex: Clojure work metadata/qualification', flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            operation = 'seal' if args.seal else 'run' if args.run else 'admit'
            terminal = HERE / 'work-qualification/controller-outcomes' / operation / args.revision
            assert not terminal.exists(), 'Exclusive actual controller outcome, no overwrite/retry'
            terminal.mkdir(parents=True)
            input_pins = {str(PACKET / name): sha(PACKET / name) for name in STATIC_FILES}
            new_json(terminal / 'started.json', dict(operation=operation, revision=args.revision,
                actual_argv=sys.argv, held_common_descriptor=lock.fileno(), common_mutex=str(LOCK),
                exact_controller_source_pins=input_pins, full_performance_runs=0))
            result = dict(operation=operation, revision=args.revision, status='failed-before-completion',
                full_performance_runs=0, actual_common_mutex_held=True)
            try:
                if args.seal:
                    seal(args, lock)
                elif args.run:
                    run(args, lock)
                else:
                    admit(args)
                verify(input_pins)
                result['status'] = 'completed'
            except BaseException as error:
                result.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
                (terminal / 'actual-error.log').write_text(traceback.format_exc())
                raise
            finally:
                # Failed fixtures/import/inventory/source replay keep every
                # actually available byte, including a prior passed fixture
                # receipt. No failed metadata revision is relabeled or retried.
                stage = HERE / ('work-qualification/proposal-seals' if args.seal else
                    'work-qualification/jobs' if args.run else 'work-qualification/qualified') / args.revision
                files = [path for root in (stage, terminal) if root.exists() for path in root.rglob('*') if path.is_file()]
                if args.seal and args.proposal_file.exists():
                    files.append(args.proposal_file)
                result['actual_terminal_evidence_pins'] = {str(path): sha(path) for path in files}
                new_json(terminal / 'outcome.json', result)


if __name__ == '__main__':
    main()
