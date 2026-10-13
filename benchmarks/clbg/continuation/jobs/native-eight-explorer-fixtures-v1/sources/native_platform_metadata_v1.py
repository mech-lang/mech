"""Attach verified platform evidence to the six fully admitted native cohorts.

Run after native_languages_data_v1 and before endpoint pruning. Source names,
language names and display labels never establish a measured backend. The
reviewed language metadata must bind an actual compiler/runtime receipt.
"""
from copy import deepcopy
import hashlib
import json
from pathlib import Path

from native_languages_data_v1 import held_mutex, require

BACKENDS = {
    'nim': ('native-aot', 'aot'), 'crystal': ('native-aot', 'aot'),
    'pony': ('native-aot', 'aot'), 'c3': ('native-aot', 'aot'),
    'gleam': ('beam-jit', 'jit'), 'clojure': ('jvm-jit', 'jit'),
}


def digest(value):
    return isinstance(value, str) and len(value) == 64 and all(c in '0123456789abcdef' for c in value)


def pointer_value(document, pointer):
    require(isinstance(pointer, str) and pointer.startswith('/'), 'Actual runtime JSON pointer required')
    value = document
    for raw in pointer[1:].split('/'):
        token = raw.replace('~1', '/').replace('~0', '~')
        if isinstance(value, list):
            require(token.isdigit() and str(int(token)) == token, 'Invalid runtime array pointer')
            value = value[int(token)]
        else:
            require(isinstance(value, dict) and token in value, 'Missing actual runtime receipt field')
            value = value[token]
    return value


def stage(data, metadata, actual_runtime_reports, artifact_provenance):
    require(set(metadata) == set(actual_runtime_reports) == set(BACKENDS), 'All six actual language metadata entries required')
    require(set(data['native_language_audit']['languages']) == set(BACKENDS), 'Import all six complete cohorts first')
    program_keys = [row['key'] for row in data['programs'].values()]
    require(len(program_keys) == len(set(program_keys))
            and all(key == row['key'] for key, row in data['programs'].items()),
            'Actual program mapping keys and unique row identities must match before commit')
    language_keys = [item['key'] for item in data['languages']]
    require(set(BACKENDS) <= set(language_keys) and len(language_keys) == len(set(language_keys))
            and 'v' not in language_keys and not any(row['language'] == 'v' for row in data['programs'].values())
            and not any(group['language'] == 'v' for benchmark in data['benchmarks'] for group in benchmark['groups']),
            'Actual language inventory must contain all six and keep V report-only')
    changes, memory = {}, {}
    for language, (backend, execution) in BACKENDS.items():
        meta = metadata[language]
        require(meta['language'] == language and meta['status'] == 'reviewed-actual-native-language-metadata',
                'Planned metadata cannot qualify a measured platform')
        require(all(meta[name] is True for name in ('actual_compiler_version_verified',
                    'actual_native_architecture_verified', 'actual_memory_policy_verified')),
                'Actual compiler, architecture and memory evidence required')
        template = meta['measured_backend_template']
        require(template['verified'] is True and template['key'] == backend and template['execution'] == execution
                and isinstance(template['label'], str) and template['label'].strip(), 'Contradictory actual backend template')
        runtime = template['runtime_receipt']
        require(runtime['verified'] is True and digest(runtime['report_sha256'])
                and artifact_provenance.get(runtime['report']) == runtime['report_sha256'], 'Runtime receipt byte identity missing')
        actual = pointer_value(actual_runtime_reports[language], runtime['field'])
        if runtime['value_encoding'] == 'literal-saved-string':
            require(isinstance(actual, str), 'Actual saved literal runtime string required')
            expected = actual
        else:
            require(runtime['value_encoding'] == 'json-serialized-preserved-value', 'Unknown runtime receipt encoding')
            expected = json.dumps(actual, sort_keys=True)
        require(isinstance(runtime['value'], str) and runtime['value'] and runtime['value'] == expected,
                'Runtime label does not match the pinned actual receipt field')
        strategies = meta['memory_strategies']
        require(isinstance(strategies, list) and strategies and len(strategies) == len(set(strategies))
                and set(strategies) <= {'gc', 'rc', 'owned'}, 'Actual runtime memory strategies required')
        audit = data['native_language_audit']['languages'][language]
        require(audit['source_count'] == 20 and audit['actual_full_native_processes'] == 90 and audit['sample_count'] == 3,
                'Complete admitted source and measurement cohort required')
        rows = [row for row in data['programs'].values() if row['language'] == language]
        require(len(rows) == 20 and len({row['key'] for row in rows}) == 20, 'Platform evidence must precede endpoint pruning')
        for row in rows:
            require(row['execution_mode'] == execution and row['required_work_validated'] is True,
                    'Measured source execution mode differs from its actual backend')
            report = row['report']
            report_sha = data['provenance'].get(report)
            require(digest(report_sha), 'Actual full measurement report provenance required')
            command = row['runtime_arguments']
            require(isinstance(command, list) and command and all(isinstance(v, str) and v for v in command),
                    'Actual measured command required')
            sources = [data['files'][identity]['sha256'] for identity in row['files']]
            require(sources and all(digest(value) for value in sources), 'Actual counted authored source bundle required')
            record = dict(key=backend, execution=execution, label=template['label'], verified=True,
                evidence=dict(verified=True, report=report, report_sha256=report_sha,
                    command=deepcopy(command), source_sha256=sources, runtime_receipt=deepcopy(runtime)))
            require('measured_backend' not in row or row['measured_backend'] == record, 'Conflicting measured backend evidence')
            changes[row['key']] = record
        meta_evidence = meta['publication_metadata_evidence']
        require(artifact_provenance.get(meta_evidence['path']) == meta_evidence['sha256'],
                'Reviewed memory metadata byte identity required')
        memory[language] = dict(verified=True, strategies=deepcopy(strategies), label=meta['memory_label'],
            evidence=deepcopy(meta_evidence))
    provenance = dict(data['provenance'])
    for path, value in artifact_provenance.items():
        require(path not in provenance or provenance[path] == value, 'Conflicting runtime provenance')
        provenance[path] = value
    return changes, memory, provenance


def add_native_platform_metadata(root, data, *, packet_path, packet_sha256, common_timing_mutex_fd):
    held_mutex(common_timing_mutex_fd)
    root = Path(root).resolve()
    pins, provenance = {}, {}

    def read(path, expected, *, project_artifact=True):
        path = Path(path).resolve(strict=True)
        require((not project_artifact or path.is_relative_to(root)) and digest(expected), 'Pinned actual runtime artifact required')
        raw = path.read_bytes()
        require(hashlib.sha256(raw).hexdigest() == expected, 'Actual runtime evidence changed: ' + str(path))
        require(str(path) not in pins or pins[str(path)] == expected, 'Conflicting runtime artifact pin')
        pins[str(path)] = expected
        if path.is_relative_to(root):
            provenance[path.relative_to(root).as_posix()] = expected
        return raw

    packet = json.loads(read(packet_path, packet_sha256))
    require(packet['status'] == 'reviewed-six-language-publication' and set(packet['languages']) == set(BACKENDS)
            and packet['unresolved'] == [], 'Reviewed whole-six publication packet required')
    packet_name = Path(packet_path).resolve().relative_to(root).as_posix()
    require(data['native_language_audit']['packet'] == packet_name and data['provenance'].get(packet_name) == packet_sha256,
            'Platform metadata must use the exact packet already admitted by the native importer')
    metadata, actual = {}, {}
    for language, entry in packet['languages'].items():
        artifact = entry['metadata']
        require(data['native_language_audit']['languages'][language]['artifacts']['metadata'] == artifact,
                'Platform metadata differs from the imported actual language descriptor')
        meta = json.loads(read(artifact['path'], artifact['sha256']))
        require(meta['pins'], 'Actual metadata proof pin closure required')
        for path, expected in meta['pins'].items():
            read(path, expected, project_artifact=False)
        runtime = meta['measured_backend_template']['runtime_receipt']
        require(meta['pins'].get(runtime['report']) == runtime['report_sha256'], 'Runtime receipt omitted from reviewed metadata pins')
        actual[language] = json.loads(read(runtime['report'], runtime['report_sha256']))
        metadata[language] = deepcopy(meta)
        metadata[language]['measured_backend_template']['runtime_receipt']['report'] = Path(runtime['report']).resolve().relative_to(root).as_posix()
        metadata[language]['publication_metadata_evidence'] = dict(
            path=Path(artifact['path']).resolve().relative_to(root).as_posix(), sha256=artifact['sha256'])
    changes, memory, staged_provenance = stage(data, metadata, actual, provenance)
    # Recheck all file prerequisites before the only mutations.
    for path, expected in pins.items():
        require(hashlib.sha256(Path(path).read_bytes()).hexdigest() == expected, 'Runtime artifact changed before commit')
    for key, record in changes.items():
        data['programs'][key]['measured_backend'] = record
    for language in data['languages']:
        if language['key'] in memory:
            language['measured_memory_policy'] = memory[language['key']]
    data['provenance'].clear()
    data['provenance'].update(staged_provenance)
    return dict(verified_programs=len(changes), verified_languages=len(memory), V_visualization_entries=0)
