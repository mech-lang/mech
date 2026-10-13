"""Pure transactional metadata for the exact 133 existing measured programs.

The caller qualifies and pins original evidence under the common mutex. This
module performs no I/O, hashing, fixture/native launch or measurement changes.
"""
from copy import deepcopy
import math
from pathlib import PurePosixPath

COUNTS = {'c': 18, 'cpp': 18, 'rust': 19, 'mech': 15, 'mech-jit': 14,
          'mech-resident': 14, 'julia': 17, 'javascript': 18}
STATUS = {'qualified-existing-platform-history', 'reconciled-with-unresolved-existing-platform-history'}
CONFIDENCE = {'historical_platform_verified', 'historical_execution_verified',
              'historical_executable_bytes_verified'}
BACKENDS = {'native-aot': 'aot', 'native-jit': 'jit', 'native-interpreter': 'interpreted',
            'native-runtime': 'unknown', 'javascript-runtime': 'unknown',
            'javascript-jit': 'jit', 'javascript-interpreter': 'interpreted', 'unknown': 'unknown'}
ALLOWED = {'c': {'native-aot', 'native-runtime', 'unknown'},
           'cpp': {'native-aot', 'native-runtime', 'unknown'},
           'rust': {'native-aot', 'native-runtime', 'unknown'},
           'mech': {'native-aot', 'native-runtime', 'unknown'},
           'mech-jit': {'native-jit', 'native-runtime', 'unknown'},
           'mech-resident': {'native-interpreter', 'native-runtime', 'unknown'},
           'julia': {'native-jit', 'native-interpreter', 'native-runtime', 'unknown'},
           'javascript': {'javascript-runtime', 'javascript-jit', 'javascript-interpreter', 'unknown'}}
IDENTITY_FIELDS = {'key', 'id', 'benchmark', 'language', 'files', 'report', 'source_report',
                   'samples', 'seconds', 'timing_boundary'}
METADATA_FIELDS = {'measured_backend', 'runtime_policy', 'execution_mode', 'prior_runtime_metadata'}
SAVED_MEASUREMENT_FIELDS = {'binary_sha256', 'native_object_sha256', 'native_flags', 'command', 'runtime_arguments',
    'sample_details', 'sample_validation', 'full_output_validation', 'whole_process_seconds', 'full_process_seconds',
    'preparation_seconds', 'whole_process_samples', 'full_process_samples', 'preparation_samples'}
FILE_IDENTITY_FIELDS = ('path', 'sha256', 'code', 'name')


def require(value, message):
    if not value: raise ValueError(message)


def digest(value):
    return type(value) is str and len(value) == 64 and all(c in '0123456789abcdef' for c in value)


def relative(value):
    if type(value) is not str or not value or '\\' in value: return False
    path = PurePosixPath(value)
    return not path.is_absolute() and path.as_posix() == value and '..' not in path.parts and ':' not in path.parts[0]


def same(left, right):
    """Saved JSON identity includes scalar types; bool is not a sample/count."""
    if type(left) is not type(right): return False
    if type(left) is dict: return set(left) == set(right) and all(same(left[k], right[k]) for k in left)
    if type(left) is list: return len(left) == len(right) and all(same(a, b) for a, b in zip(left, right))
    return left == right


def positive(value):
    return type(value) in (int, float) and math.isfinite(value) and value > 0


def source_files(data, row):
    require(type(row['files']) is list and row['files'] and len(row['files']) == len(set(row['files'])),
            'Exact nonempty unique source bundle required')
    result = []
    for identity in row['files']:
        require(type(identity) is str and identity in data['files'], 'Missing measured source identity')
        item = data['files'][identity]
        require(relative(item['path']) and digest(item['sha256']), 'Invalid measured source path/hash')
        result.append(dict(path=item['path'], sha256=item['sha256']))
    require(len({item['path'] for item in result}) == len(result), 'Repeated measured source path')
    return result


def identity(data, row):
    return dict(key=row['key'], id=row['id'], benchmark=row['benchmark'], language=row['language'],
        files=source_files(data, row), report=row['report'], source_report=row.get('source_report'),
        samples=deepcopy(row['samples']), seconds=row['seconds'], timing_boundary=deepcopy(row.get('timing_boundary')))


def add_pin(provenance, path, value):
    require(relative(path) and digest(value), 'Relative artifact path and SHA required')
    require(path not in provenance or provenance[path] == value, 'Conflicting preserved provenance: ' + path)
    provenance[path] = value


def stage(data, baseline, qualification):
    """Validate every identity/evidence record before returning detached changes."""
    require(qualification['status'] in STATUS and type(qualification['profile_count']) is int
            and qualification['profile_count'] == 133, 'Exact qualified 133-program history required')
    for document in (data, baseline):
        require(type(document['programs']) is dict and type(document['files']) is dict
                and type(document['provenance']) is dict, 'Actual dataset mappings required')
        require(all(type(row) is dict for row in document['programs'].values()), 'Literal program dictionaries required for transactional commit')
        require(all(key == row['key'] for key, row in document['programs'].items())
                and len({row['key'] for row in document['programs'].values()}) == len(document['programs']),
                'Program mapping keys must equal unique row keys before commit')
    current = {key: row for key, row in data['programs'].items() if row['language'] in COUNTS}
    old = {key: row for key, row in baseline['programs'].items() if row['language'] in COUNTS}
    annotations, measured = qualification['annotations'], qualification['measurement_identities']
    require(type(annotations) is type(measured) is dict and
            set(current) == set(old) == set(annotations) == set(measured) and len(current) == 133,
            'All exact old/current/qualified measurement identities required')
    for language, count in COUNTS.items():
        require(sum(row['language'] == language for row in current.values()) ==
                sum(row['language'] == language for row in old.values()) == count, 'Selected language cohort changed')
    provenance = dict(data['provenance'])
    for path, value in qualification['provenance'].items(): add_pin(provenance, path, value)
    changes, incomplete = {}, False
    for key in sorted(current):
        row, previous, expected, backend = current[key], old[key], measured[key], annotations[key]
        require(key == row['key'] == previous['key'] == row['benchmark'] + ':' + row['id'], 'Canonical row identity changed')
        require(all(same(row.get(name), previous.get(name)) for name in SAVED_MEASUREMENT_FIELDS),
                'Original optional measurement/runtime/source-byte evidence changed')
        require(type(expected) is dict and set(expected) == IDENTITY_FIELDS and same(expected, identity(data, row))
                and same(expected, identity(baseline, previous)), 'Qualified source/sample/report/boundary identity differs')
        require(all(same({field: data['files'][name].get(field) for field in FILE_IDENTITY_FIELDS},
                         {field: baseline['files'][name].get(field) for field in FILE_IDENTITY_FIELDS}) for name in row['files']),
                'Original displayed source content/provenance changed')
        require(type(row['samples']) is list and len(row['samples']) >= 3 and all(positive(v) for v in row['samples'])
                and positive(row['seconds']), 'Saved complete positive samples required; sample count must not be normalized')
        require(relative(row['report']) and row.get('source_report') is not None and relative(row['source_report']),
                'Original report and source-report identities required')
        for report in (row['report'], row['source_report']):
            require(digest(baseline['provenance'].get(report)) and
                    baseline['provenance'][report] == data['provenance'].get(report) == qualification['provenance'].get(report),
                    'Historical report provenance changed or omitted')
        require(type(backend) is dict and backend['key'] in ALLOWED[row['language']]
                and backend['execution'] == BACKENDS[backend['key']] and type(backend['verified']) is bool,
                'Unexpected backend or execution for this measured cohort')
        confidence, evidence = backend['confidence'], backend['evidence']
        require(type(confidence) is dict and set(confidence) == CONFIDENCE
                and all(type(value) is bool for value in confidence.values()), 'Independent literal confidence flags required')
        require(backend['verified'] is confidence['historical_platform_verified']
                and evidence['verified'] is backend['verified'], 'Platform/evidence confidence mismatch')
        if backend['execution'] != 'unknown':
            require(confidence['historical_execution_verified'] is True, 'Mode-specific backend needs actual saved mode proof')
        else:
            require(confidence['historical_execution_verified'] is False, 'Unknown mode cannot claim qualified execution')
        require(backend['key'] != 'unknown' or backend['verified'] is False, 'Unclassified cannot assert a proved platform')
        require(type(backend['missing_evidence']) is list and len(backend['missing_evidence']) == len(set(backend['missing_evidence']))
                and all(type(reason) is str and reason.strip() for reason in backend['missing_evidence']), 'Concrete missing evidence required')
        if not all(confidence.values()): require(backend['missing_evidence'], 'Unknown historical fact needs a retained reason')
        # Confidence dimensions are independent of other retained limitations,
        # such as a timed argv absent despite an exactly correlated native build.
        incomplete = incomplete or bool(backend['missing_evidence'])
        require(evidence['report'] == row['report'] and evidence['report_sha256'] == provenance[row['report']]
                and digest(evidence['report_sha256']), 'Annotation is not bound to its actual measurement report')
        require(type(evidence['source_sha256']) is list and same(evidence['source_sha256'],
                [item['sha256'] for item in expected['files']]), 'Complete ordered measured source hashes required')
        command = evidence['command']
        require(type(command) is list and all(type(part) is str and part for part in command)
                and (command or backend['verified'] is False), 'Saved actual runtime command required for proved platform')
        role = evidence.get('command_role', 'historical-timed-argv')
        missing_argv = evidence.get('missing_historical_timed_argv', False)
        require(type(missing_argv) is bool, 'Missing historical argv fact must be literal bool')
        if role == 'native-build-link':
            require(row['language'] == 'mech' and backend['key'] in ('native-aot', 'native-runtime')
                    and missing_argv is True and backend['missing_evidence'],
                    'Build/link command must retain absent timed argv and actual native proof scope')
        else:
            require(role == 'historical-timed-argv' and missing_argv is False, 'Unknown or contradictory command role')
            for field in ('command', 'runtime_arguments'):
                if field in previous: require(same(previous[field], command), 'Saved displayed command differs from qualified command')
        runtime = evidence['runtime_receipt']
        require(type(runtime) is dict and type(runtime['verified']) is bool, 'Actual runtime receipt state required')
        if backend['verified']: require(runtime['verified'] is True, 'Proved platform needs actual cohort-bound runtime observation')
        if runtime['verified']:
            require(relative(runtime['report']) and digest(runtime['report_sha256'])
                    and provenance.get(runtime['report']) == runtime['report_sha256']
                    and qualification['provenance'].get(runtime['report']) == runtime['report_sha256'], 'Pinned runtime receipt missing')
            require(type(runtime['field']) is str and runtime['field'].startswith('/')
                    and type(runtime['value']) is str and runtime['value']
                    and runtime['value_encoding'] in ('literal-saved-string', 'json-serialized-preserved-value'),
                    'Exact saved runtime pointer/value/encoding required')
        if backend['verified']:
            require(type(backend['label']) is str and backend['label'].strip(), 'Actual or generic measured backend label required')
        annotation = deepcopy(backend)
        if not annotation.get('label'): annotation['label'] = 'Unclassified'
        require('measured_backend' not in row or same(row['measured_backend'], annotation), 'Conflicting prior backend annotation')
        prior = dict(scope='preserved-display-fields-not-historical-verification',
            fields={name: deepcopy(previous[name]) for name in ('runtime_policy', 'execution_mode') if name in previous})
        require('prior_runtime_metadata' not in row or same(row['prior_runtime_metadata'], prior), 'Conflicting preserved runtime metadata')
        changes[key] = dict(measured_backend=annotation, prior_runtime_metadata=prior,
            runtime_policy=annotation['label'] if backend['verified'] else 'Unclassified',
            execution_mode=backend['execution'] if backend['verified'] else 'unknown')
    require((not incomplete) == (qualification['status'] == 'qualified-existing-platform-history'),
            'Terminal qualification status contradicts retained unresolved history')
    return changes, provenance


def apply(data, baseline, qualification):
    changes, provenance = stage(data, baseline, qualification)
    for key, fields in changes.items(): data['programs'][key].update(fields)
    data['provenance'].clear(); data['provenance'].update(provenance)
    return dict(programs=len(changes), metadata_only=True, sources_mutated=False, timings_mutated=False)
