"""Unhooked, transactional overlay of reviewed historical backend metadata.

The caller supplies the actual reviewed qualification and terminal receipt
digests and holds the common timing mutex. No benchmark, version probe, source
edit, timing change, language admission, endpoint selection or LOC occurs here.
"""
from copy import deepcopy
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import re
import statistics

LOCK = '/tmp/mech-clbg-timing.lock'
COUNTS = {'java': 17, 'erlang': 14, 'csharp': 20, 'csharp-aot': 19,
          'fsharp': 15, 'fsharp-aot': 15}
BACKENDS = {'java': ('openjdk-jit', 'jit'), 'erlang': ('beam-jit', 'jit'),
            'csharp': ('dotnet-jit', 'jit'), 'fsharp': ('dotnet-jit', 'jit'),
            'csharp-aot': ('dotnet-native-aot', 'aot'), 'fsharp-aot': ('dotnet-native-aot', 'aot')}
STATUSES = {'qualified-preserved-platform-metadata', 'reconciled-with-unresolved-platform-metadata'}
FIXTURE_IDS = {'test_qualify_metadata.MetadataEvidence.' + name for name in (
    'test_java_saved_jit_and_classpath_are_required',
    'test_dotnet_host_dll_cannot_be_reclassified_as_native_aot',
    'test_native_aot_requires_saved_publish_and_native_command',
    'test_native_arm64_header_rejects_x86_and_non_native_bytes',
    'test_changed_partial_nan_or_fabricated_samples_are_rejected',
    'test_exact_inventory_and_preserved_identity_are_required')}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(value):
    return type(value) is str and len(value) == 64 and all(character in '0123456789abcdef' for character in value)


def positive(value):
    return type(value) in (int, float) and math.isfinite(value) and value > 0


def held_mutex(fd):
    require(type(fd) is int, 'Backend metadata requires the caller-held common timing mutex')
    named, opened = os.stat(LOCK), os.fstat(fd)
    require((named.st_dev, named.st_ino) == (opened.st_dev, opened.st_ino), 'Different common timing descriptor')
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        raise ValueError('Backend metadata must wait for the common timing boundary') from None


def add_provenance(staged, path, value):
    require(type(path) is str and path and digest(value), 'Invalid backend provenance pin')
    require(path not in staged or staged[path] == value, 'Conflicting existing provenance: ' + path)
    staged[path] = value


def validate_receipt(qualification, receipt):
    require(qualification['status'] in STATUSES and receipt['status'] == qualification['status'],
            'Backend metadata has no passed terminal qualification')
    require(all(type(artifact[name]) is int for artifact in (qualification, receipt)
                for name in ('expected_program_count', 'actual_program_count', 'qualified_program_count')) and
            qualification['expected_program_count'] == qualification['actual_program_count'] ==
            receipt['expected_program_count'] == receipt['actual_program_count'] == 100,
            'Exactly one hundred actual admitted backend records required')
    require(receipt['exclusive_common_timing_mutex_held'] is True and receipt['pins_unchanged_at_end'] is True,
            'Actual exclusive qualification and unchanged evidence required')
    for artifact in (qualification, receipt):
        for flag in ('chart_admission', 'dataset_mutated', 'sources_mutated', 'timings_mutated'):
            require(artifact[flag] is False, 'Qualification changed source/timing/admission')
    require(all(type(artifact[name]) is int and artifact[name] == 0 for artifact, names in (
                (qualification, ('native_processes', 'runtime_version_reruns')),
                (receipt, ('actual_native_processes', 'runtime_version_reruns', 'benchmark_replays')))
                for name in names) and
            qualification['native_processes'] == qualification['runtime_version_reruns'] ==
            receipt['actual_native_processes'] == receipt['runtime_version_reruns'] == receipt['benchmark_replays'] == 0,
            'Historical metadata qualification must not execute benchmarks or runtime versions')
    require(receipt['expected_fixture_count'] == receipt['tests_run'] == 6, 'Actual six-fixture qualification required')
    for key in ('required_fixture_ids', 'discovered_fixture_ids', 'actual_started_fixture_ids'):
        values = receipt[key]
        require(type(values) is list and len(values) == len(set(values)) == 6 and set(values) == FIXTURE_IDS,
                'Exact actual backend fixture identities required')
    require(all(type(receipt[key]) is list and receipt[key] == [] for key in
        ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes')), 'Backend fixtures did not pass cleanly')


def stage(data, saved_data, qualification, receipt, expected, *, prior_dataset):
    """Pure semantic staging after byte pins have been checked by the caller."""
    validate_receipt(qualification, receipt)
    require(type(prior_dataset) is dict and set(prior_dataset) == {'path', 'sha256', 'origin_path', 'origin_sha256'} and
            type(prior_dataset['path']) is str and prior_dataset['path'].startswith('continuation/jobs/') and
            prior_dataset['path'].endswith('/snapshot/4-data.json') and
            prior_dataset['origin_path'] == 'overview/explorer/dist/data.json' and
            digest(prior_dataset['sha256']) and prior_dataset['sha256'] == prior_dataset['origin_sha256'],
            'Pinned immutable pre-overlay dataset and original byte identity required')
    require(type(expected) is list and len(expected) == len(set(expected)) == 100, 'Exact pinned hundred-ID inventory required')
    require(not any(row['language'] == 'v' for row in data['programs'].values()) and
            not any(row['key'] == 'v' for row in data['languages']), 'V remains report-only')
    current = {key: row for key, row in data['programs'].items() if row['language'] in COUNTS}
    old = {key: row for key, row in saved_data['programs'].items() if row['language'] in COUNTS}
    require(set(current) == set(old) == set(expected) == set(qualification['annotations']),
            'Current admitted program identities differ from the qualified hundred records')
    for language, count in COUNTS.items():
        require(sum(row['language'] == language for row in current.values()) == count, 'Current language profile count changed')
    changes, unresolved = {}, {}
    provenance = dict(data['provenance'])
    for path, value in qualification['provenance'].items():
        add_provenance(provenance, path, value)
    for key in sorted(current):
        program, previous, backend = current[key], old[key], qualification['annotations'][key]
        require(key == program['key'] == program['benchmark'] + ':' + program['id'], 'Invalid canonical program identity')
        for name in ('key', 'id', 'language', 'benchmark', 'files', 'report', 'seconds', 'samples'):
            require(program[name] == previous[name], 'Qualified current measurement/source identity changed: ' + name)
        require(type(program['samples']) is list and len(program['samples']) == 3 and
                all(positive(value) for value in program['samples']) and positive(program['seconds']) and
                math.isclose(statistics.median(program['samples']), program['seconds'], rel_tol=0, abs_tol=1e-10),
                'Actual three samples and unchanged median required')
        sources = []
        for identity in program['files']:
            source, saved_source = data['files'][identity], saved_data['files'][identity]
            require(source['path'] == saved_source['path'] and source['sha256'] == saved_source['sha256'] and digest(source['sha256']),
                    'Qualified displayed source pin/path changed')
            sources.append(source['sha256'])
        require(sources, 'Backend annotation needs an actual displayed source bundle')
        expected_backend, expected_execution = BACKENDS[program['language']]
        require(backend['key'] == expected_backend and backend['execution'] == expected_execution and
                type(backend['verified']) is bool, 'Unexpected or unqualified backend mode')
        evidence = backend['evidence']
        require(type(evidence['verified']) is bool and evidence['verified'] is backend['verified'], 'Backend evidence verification mismatch')
        require(evidence['report'] == program['report'] and digest(evidence['report_sha256']) and
                data['provenance'].get(program['report']) == evidence['report_sha256'] and
                qualification['provenance'].get(program['report']) == evidence['report_sha256'] and
                sorted(evidence['source_sha256']) == sorted(sources), 'Backend report/source provenance changed')
        require(type(evidence['command']) is list and evidence['command'] and
                all(type(part) is str and part for part in evidence['command']), 'Saved actual command is missing')
        if backend['verified']:
            require(type(backend['label']) is str and backend['label'].strip(), 'Qualified measured backend label required')
            runtime = evidence['runtime_receipt']
            require(runtime['verified'] is True and digest(runtime['report_sha256']) and
                    provenance.get(runtime['report']) == runtime['report_sha256'] and
                    type(runtime['field']) is str and runtime['field'].startswith('/') and
                    type(runtime['value']) is str and runtime['value'] and
                    runtime['value_encoding'] in ('literal-saved-string', 'json-serialized-preserved-value'),
                    'Actual pinned historical runtime receipt required')
            changes[key] = dict(measured_backend=deepcopy(backend), runtime_policy=backend['label'], execution_mode=backend['execution'])
        else:
            reason = backend.get('unresolved_reason')
            require(type(reason) is str and reason, 'Unresolved backend needs a concrete reason')
            unresolved[key] = reason
            prior = dict(scope='preserved-display-fields-not-runtime-verification',
                dataset=deepcopy(prior_dataset),
                fields={name: deepcopy(previous[name]) for name in ('runtime_policy', 'execution_mode') if name in previous})
            require('prior_runtime_metadata' not in program or program['prior_runtime_metadata'] == prior,
                    'Conflicting preserved pre-overlay runtime metadata')
            changes[key] = dict(measured_backend=deepcopy(backend), runtime_policy='Unclassified',
                                execution_mode='unknown', prior_runtime_metadata=prior)
    require(qualification['qualified_program_count'] == receipt['qualified_program_count'] == 100-len(unresolved),
            'Qualified backend record count differs')
    require(qualification['unresolved_programs'] == receipt['unresolved_programs'] and
            {item['key']: item['reason'] for item in qualification['unresolved_programs']} == unresolved,
            'Unresolved backend records differ from the terminal receipt')
    require((not unresolved) == (qualification['status'] == 'qualified-preserved-platform-metadata'), 'Backend status differs from unresolved evidence')
    java = [key for key, row in current.items() if row['language'] == 'java']
    java_verified = sum(qualification['annotations'][key]['verified'] is True for key in java)
    java_unknown = len(java) - java_verified
    java_repaired = len(java) == 17
    group_changes, protocol_changes = {}, {}
    if java_repaired:
        groups = data['language_groups']
        require(len({group['key'] for group in groups}) == len(groups), 'Duplicate language group')
        require({'aot', 'jit', 'interpreted'} <= {group['key'] for group in groups}, 'Execution group inventory incomplete')
        for index, group in enumerate(groups):
            if group['key'] not in ('aot', 'jit', 'interpreted'): continue
            values = group['languages']
            require(type(values) is list and len(values) == len(set(values)), 'Execution group has duplicate languages')
            updated = [language for language in values if language != 'java']
            # Aggregate execution groups require all displayed profiles to
            # verify that mode; mixed verification remains unclassified.
            if group['key'] == 'jit' and java_unknown == 0: updated.append('java')
            group_changes[index] = updated
        old_text = data['protocol']['language_groups']
        require(type(old_text) is str, 'Language execution protocol missing')
        stale = 'Java uses GraalVM native AOT builds in this cohort.'
        require(old_text.count(stale) <= 1, 'Ambiguous duplicated Java execution prose')
        replacement = (f'{java_verified} of 17 displayed Java profiles have verified OpenJDK JVM JIT evidence; '
                       f'{java_unknown} remain Unclassified.' if java_unknown else
                       'All 17 displayed Java profiles have verified OpenJDK JVM JIT evidence.')
        text = old_text.replace(stale, replacement)
        rebuilt_clause = 'Java uses GraalVM native AOT,'
        require(text.count(rebuilt_clause) <= 1, 'Ambiguous duplicated rebuilt Java execution prose')
        text = text.replace(rebuilt_clause, replacement + ' Other measured runtimes:')
        require(not re.search(r'\bJava\b[^.;\n]{0,200}\b(?:AOT|NativeAOT|ahead[- ]of[- ]time)\b', text, re.IGNORECASE),
                'Unrecognized contradictory measured Java AOT claim requires review')
        if replacement not in text: text += ' ' + replacement
        protocol_changes = dict(language_groups=text,
            java=replacement + ' Saved commands and per-profile historical runtime receipts are pinned. '
                 'Unresolved rows retain concrete reasons and their previous display fields as provenance. '
                 'Upstream GraalVM NativeAOT source provenance is retained.')
    return dict(changes=changes, provenance=provenance, group_changes=group_changes,
        protocol_changes=protocol_changes, java_repaired=java_repaired, java_verified=java_verified,
        java_unknown=java_unknown, unresolved=unresolved)


def add_platform_metadata(root, data, *, qualification_path, qualification_sha256,
                          receipt_path, receipt_sha256, common_timing_mutex_fd):
    held_mutex(common_timing_mutex_fd)
    root = Path(root).resolve()
    require(digest(qualification_sha256) and digest(receipt_sha256), 'Reviewed qualification and terminal receipt digests required')
    pins = {}
    def read(path, expected):
        path = Path(path).resolve(strict=True)
        value = path.read_bytes()
        actual = hashlib.sha256(value).hexdigest()
        require(digest(expected) and actual == expected, 'Reviewed backend artifact changed: ' + str(path))
        require(str(path) not in pins or pins[str(path)] == expected, 'Conflicting backend byte pin')
        pins[str(path)] = expected
        return value
    qualification_path, receipt_path = Path(qualification_path).resolve(strict=True), Path(receipt_path).resolve(strict=True)
    require(qualification_path.is_relative_to(root / 'continuation/jobs') and receipt_path.parent == qualification_path.parent,
            'Actual reviewed local backend qualification required')
    qualification = json.loads(read(qualification_path, qualification_sha256))
    receipt = json.loads(read(receipt_path, receipt_sha256))
    require(receipt['metadata_output'] == dict(path=str(qualification_path), sha256=qualification_sha256),
            'Terminal backend receipt names another metadata output')
    validate_receipt(qualification, receipt)
    dataset_origin = root / 'overview/explorer/dist/data.json'
    dataset = qualification_path.parent / 'snapshot/4-data.json'
    require(str(dataset_origin) in qualification['pins'] and str(dataset) in qualification['pins'] and
            qualification['pins'][str(dataset_origin)] == qualification['pins'][str(dataset)] and
            receipt['source_pins'].get(str(dataset)) == qualification['pins'][str(dataset)],
            'Immutable qualified baseline must preserve exact original generated dataset bytes')
    for mapping in (qualification['pins'], receipt['source_pins']):
        for path, expected in mapping.items():
            if path == str(dataset_origin):
                require(expected == qualification['pins'][str(dataset)], 'Original generated dataset pin differs from immutable baseline')
                continue
            read(path, expected)
    raw = receipt['raw_fixture_log']
    read(raw['path'], raw['sha256'])
    inventory = root / 'continuation/platform-metadata-qualification-v1/expected-program-ids.json'
    require(str(inventory) in qualification['pins'], 'Original qualified inventory pin missing')
    expected = json.loads(read(inventory, qualification['pins'][str(inventory)]))
    saved_data = json.loads(read(dataset, qualification['pins'][str(dataset)]))
    pending = stage(data, saved_data, qualification, receipt, expected,
                    prior_dataset=dict(path=dataset.relative_to(root).as_posix(), sha256=qualification['pins'][str(dataset)],
                                       origin_path=dataset_origin.relative_to(root).as_posix(),
                                       origin_sha256=qualification['pins'][str(dataset_origin)]))
    for path, value in qualification['provenance'].items():
        actual_path = (root / path).resolve(strict=True)
        require(actual_path.is_relative_to(root), 'Backend provenance escaped benchmark root')
        read(actual_path, value)
    for key, row in pending['changes'].items():
        for identity in data['programs'][key]['files']:
            source = data['files'][identity]
            actual_path = (root / source['path']).resolve(strict=True)
            require(actual_path.is_relative_to(root), 'Displayed backend source escaped benchmark root')
            read(actual_path, source['sha256'])
    add_provenance(pending['provenance'], qualification_path.relative_to(root).as_posix(), qualification_sha256)
    add_provenance(pending['provenance'], receipt_path.relative_to(root).as_posix(), receipt_sha256)
    add_provenance(pending['provenance'], dataset.relative_to(root).as_posix(), qualification['pins'][str(dataset)])
    for path, expected_digest in pins.items():
        require(hashlib.sha256(Path(path).read_bytes()).hexdigest() == expected_digest, 'Backend evidence changed during staging')
    # Complete staging before commit. Preserve existing program/map/list and
    # source/sample objects; no read, hash, callback or validation follows.
    for key, changes in pending['changes'].items():
        data['programs'][key].update(changes)
    data['provenance'].clear()
    data['provenance'].update(pending['provenance'])
    for index, values in pending['group_changes'].items():
        data['language_groups'][index]['languages'][:] = values
    data['protocol'].update(pending['protocol_changes'])
    return dict(qualified_programs=100-len(pending['unresolved']), unresolved_programs=pending['unresolved'],
        java_execution_repaired=pending['java_repaired'], java_verified_profiles=pending['java_verified'],
        java_unclassified_profiles=pending['java_unknown'],
        java_backend_verification_complete=pending['java_unknown'] == 0, actual_native_processes=0,
        source_changes=0, timing_changes=0, LOC_changes=0, V_visualization_entries=0)
