"""Pure formatted LOC admission; all actual byte reads belong to the caller.

The guarded caller supplies a saved baseline wrapper and a qualification whose
artifact, terminal and decoded receipt objects were loaded under the common
mutex. No digest in this module is a claim that its bytes were checked here.
"""
from copy import deepcopy


STATUS = 'completed-formatted-loc-measurement'
DEFINITION = 'nonblank-noncomment-physical-lines-v1'
STATUSES = frozenset(('qualified', 'unavailable', 'pending', 'failed'))
MEASUREMENT_FIELDS = ('key', 'id', 'benchmark', 'language', 'seconds', 'samples',
                      'report', 'source_report', 'timing_boundary', 'binary_sha256',
                      'runtime_sha256', 'execution_mode', 'measurement_status',
                      'sample_execution_metadata', 'sample_details', 'full_output_validation')
SOURCE_FIELDS = ('name', 'path', 'code', 'sha256')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(value):
    return type(value) is str and len(value) == 64 and all(c in '0123456789abcdef' for c in value)


def nonempty(value):
    return type(value) is str and bool(value.strip())


def allowed_language(value):
    return nonempty(value) and value.casefold() not in ('v', 'red')


def exact_value(left, right):
    """JSON value equality that does not equate booleans, integers or floats."""
    if type(left) is not type(right):
        return False
    if type(left) is dict:
        return set(left) == set(right) and all(exact_value(left[key], right[key]) for key in left)
    if type(left) is list:
        return len(left) == len(right) and all(exact_value(a, b) for a, b in zip(left, right))
    return left == right


def row_identity(row, key):
    """Project exact present measurement fields, independent of LOC metadata.

    Older rows may omit a field. Preserve that distinction rather than turning
    absence into a null observation. The native importer uses runtime_arguments
    for the measured command; contradictory simultaneous aliases are rejected.
    """
    require(type(row) is dict and row.get('key') == key, 'Program mapping identity differs')
    result = {field: deepcopy(row[field]) for field in MEASUREMENT_FIELDS if field in row}
    if 'command' in row and 'runtime_arguments' in row:
        require(exact_value(row['command'], row['runtime_arguments']), 'Contradictory measured command aliases')
    if 'command' in row or 'runtime_arguments' in row:
        result['command'] = deepcopy(row['command'] if 'command' in row else row['runtime_arguments'])
    return result


def catalog(data):
    """Bind full source inventory and chart rows without changing either map."""
    require(type(data) is dict and type(data.get('programs')) is dict and type(data.get('files')) is dict,
            'Program and source maps required')
    inventory = data.get('program_inventory', {})
    require(type(inventory) is dict, 'Full program inventory must be a map')
    result = {}
    for pool in (inventory, data['programs']):
        for key, row in pool.items():
            require(nonempty(key) and type(row) is dict and row.get('key') == key and
                    allowed_language(row.get('language')), 'Invalid or excluded full source identity')
            files = row.get('files')
            require(type(files) is list and files and all(nonempty(value) and value in data['files'] for value in files),
                    'Full source catalog has a missing authored source')
            if key in result:
                other = result[key]
                require(exact_value(row_identity(row, key), row_identity(other, key)) and
                        exact_value(files, other['files']), 'Shared inventory measurement/source identity differs')
                for field in ('loc', 'physical_loc', 'x', 'y'):
                    require((field in row) == (field in other) and
                            (field not in row or exact_value(row[field], other[field])),
                            'Shared inventory canonical field differs: '+field)
            result[key] = row
    return result


def identity(data, key):
    pool = catalog(data)
    require(key in pool, 'Missing measurement identity')
    return row_identity(pool[key], key)


def source_identity(data, published_file):
    require(type(data.get('files')) is dict and published_file in data['files'], 'Missing published source')
    row = data['files'][published_file]
    require(type(row) is dict and all(field in row for field in SOURCE_FIELDS), 'Incomplete source identity')
    require(nonempty(row['name']) and nonempty(row['path']) and type(row['code']) is str and digest(row['sha256']),
            'Invalid published source identity')
    return {field: deepcopy(row[field]) for field in SOURCE_FIELDS}


def validate_bundle(data, key, files, compiler_inventory):
    """Bind every published file to the exact authored compiler input bundle.

    This helper accepts an already read inventory binding for the worker. stage
    additionally requires its separately pinned actual evidence descriptor.
    """
    row = catalog(data)[key]
    published = row.get('files')
    require(type(published) is list and published and all(nonempty(value) for value in published),
            'Exact published source bundle required')
    require(type(files) is list and len(files) == len(published) and
            all(type(item) is dict for item in files), 'Formatted source bundle is incomplete')
    require([item.get('published_file') for item in files] == published,
            'Formatted bundle must preserve all published file identities and order')
    authored = []
    for item in files:
        source = source_identity(data, item['published_file'])
        require(nonempty(item.get('path')) and digest(item.get('sha256')) and item['sha256'] == source['sha256'],
                'Formatted input differs from the published source digest')
        require(allowed_language(item.get('language')) and item.get('role') in ('application', 'application-helper'),
                'Excluded language or nonapplication source in formatted bundle')
        authored.append((item['path'], item['sha256']))
    # Published content IDs can repeat for distinct authored paths with equal
    # bytes. Compiler inputs must still have distinct actual paths.
    require(len(authored) == len({path for path, _ in authored}), 'Duplicate authored input path')
    if compiler_inventory is not None:
        require(type(compiler_inventory) is dict and
                compiler_inventory.get('status') == 'qualified-exact-authored-compiler-inputs',
                'Actual authored compiler input inventory is unqualified')
        require(digest(compiler_inventory.get('source_sha256')) and digest(compiler_inventory.get('binary_sha256')) and
                files[0]['role'] == 'application' and compiler_inventory['source_sha256'] == files[0]['sha256'],
                'Compiler inventory must bind the actual main source and measured executable')
        if row.get('binary_sha256') is not None:
            require(digest(row['binary_sha256']) and row['binary_sha256'] == compiler_inventory['binary_sha256'],
                    'Compiler inventory names another measured executable')
        inventory = compiler_inventory.get('files')
        require(type(inventory) is list and all(type(item) is dict and nonempty(item.get('path')) and
                digest(item.get('sha256')) for item in inventory), 'Invalid authored compiler input inventory')
        bound = [(item['path'], item['sha256']) for item in inventory]
        require(len(bound) == len({path for path, _ in bound}) and len(bound) == len(authored) and set(bound) == set(authored),
                'Formatter bundle omits or adds an authored compiler input')
    return authored


def pointer_value(document, pointer):
    require(type(pointer) is str and (pointer == '' or pointer.startswith('/')), 'Actual receipt JSON pointer required')
    if pointer == '':
        return document
    value = document
    for raw in pointer[1:].split('/'):
        token = ''
        index = 0
        while index < len(raw):
            if raw[index] == '~':
                require(index + 1 < len(raw) and raw[index + 1] in '01', 'Invalid receipt pointer escape')
                token += '~' if raw[index + 1] == '0' else '/'
                index += 2
            else:
                token += raw[index]
                index += 1
        if type(value) is list:
            require(token.isdigit() and str(int(token)) == token and int(token) < len(value),
                    'Invalid receipt array pointer')
            value = value[int(token)]
        else:
            require(type(value) is dict and token in value, 'Missing actual receipt field')
            value = value[token]
    return value


def pinned(qualification, record, label):
    require(type(record) is dict and nonempty(record.get('path')) and digest(record.get('sha256')) and
            qualification['pins'].get(record['path']) == record['sha256'], label + ' pin differs')


def receipt_binding(qualification, receipt, expected, label):
    pinned(qualification, receipt, label)
    require(receipt['path'] in qualification['receipts'], label + ' actual receipt was not supplied')
    require('binding' in receipt and exact_value(receipt['binding'], expected) and
            exact_value(pointer_value(qualification['receipts'][receipt['path']], receipt.get('binding_field')), expected),
            label + ' declared binding differs from the actual pinned receipt object')


def measured_execution(qualification, data, key, files, compiler_inventory, evidence, forbidden_paths=()):
    """Bind to actual recorded measurement fields, never a minted status.

    A compiler input inventory alone cannot identify which executable and
    command produced these samples. Missing independent measurement evidence
    or a missing saved command leaves the complete formatted profile pending.
    A supplied evidence descriptor must match its independently read actual
    artifact/record and every exact recorded identity field, or it rejects.
    Its command_image_sha256 field pointer binds the image actually launched
    at command[0]. Native records can select the same recorded binary SHA;
    interpreters can select their separately recorded runtime image SHA.
    """
    if evidence is None or compiler_inventory is None:
        return False
    require(type(evidence) is dict and type(evidence.get('binding')) is dict,
            'Actual measured execution descriptor required')
    require(evidence.get('path') not in forbidden_paths,
            'Measured execution artifact must be independent of the dataset, compiler and formatter receipts')
    receipt_binding(qualification, evidence, evidence['binding'], 'Measured execution')
    names = {'profile_id', 'benchmark', 'source_sha256', 'binary_sha256', 'command', 'samples'}
    fields = evidence.get('fields')
    require(type(fields) is dict and set(fields) in (names, names | {'command_image_sha256'}) and
            all(type(pointer) is str and pointer.startswith('/')
            for pointer in fields.values()), 'Exact recorded execution field pointers required')
    actual = identity(data, key)
    command = actual.get('command')
    if command is None:
        return False
    require(type(command) is list and command and all(nonempty(value) for value in command),
            'Saved actual measured command is malformed')
    expected = dict(profile_id=actual.get('id'), benchmark=actual.get('benchmark'),
                    source_sha256=files[0]['sha256'], binary_sha256=compiler_inventory['binary_sha256'],
                    command=command, samples=actual.get('samples'))
    require(nonempty(expected['profile_id']) and nonempty(expected['benchmark']) and
            type(expected['samples']) is list, 'Saved actual measured profile/sample identity is incomplete')
    for field in names:
        require(exact_value(pointer_value(evidence['binding'], fields[field]), expected[field]),
                'Actual measured execution ' + field + ' differs from the saved profile')
    # A path's current pin does not prove that those bytes produced the saved
    # samples. Missing recorded image evidence leaves the entire profile
    # pending before any formatter command, even when that path is pinned.
    if 'command_image_sha256' not in fields:
        return False
    image_sha = pointer_value(evidence['binding'], fields['command_image_sha256'])
    require(digest(image_sha), 'Recorded measured command image digest is malformed')
    runtime_is_image = (actual.get('execution_mode') in ('interpreted', 'jit', 'bytecode') or
                        image_sha != expected['binary_sha256'])
    if runtime_is_image and actual.get('runtime_sha256') is not None:
        require(digest(actual['runtime_sha256']) and actual['runtime_sha256'] == image_sha,
                'Recorded command image differs from the saved runtime image')
    pinned(qualification, dict(path=command[0], sha256=image_sha), 'Actual recorded measured executable image')
    return True


def validate_file(qualification, item):
    require(all(field in item for field in ('physical_loc', 'formatter_evidence', 'counter_evidence')),
            'Complete file formatter/count fields required')
    status = item.get('status')
    require(type(status) is str and status in STATUSES, 'Invalid file formatter status')
    pinned(qualification, dict(path=item['path'], sha256=item['sha256']), 'Original source')
    if status != 'qualified':
        require(item.get('physical_loc') is None and nonempty(item.get('reason')),
                'Unqualified file requires a null count and a reason')
        return
    require(type(item.get('physical_loc')) is int and item['physical_loc'] >= 0,
            'Qualified file physical LOC must be a nonnegative integer')
    formatter = item.get('formatter_evidence')
    counter = item.get('counter_evidence')
    require(type(formatter) is dict and type(counter) is dict, 'Actual formatter and counter evidence required')
    sources = formatter.get('official_sources')
    require(formatter.get('provider') == 'language-project' and type(sources) is list and sources and
            all(nonempty(value) and value.startswith('https://') for value in sources), 'Official formatter ownership required')
    for field in ('source_preservation_verified', 'ownership_verified', 'no_application_execution'):
        require(formatter.get(field) is True, 'Formatter did not preserve the reviewed source-only scope')
    for field in ('tool', 'config', 'input', 'output'):
        pinned(qualification, formatter.get(field), 'Formatter ' + field)
    require(nonempty(formatter['tool'].get('version')) and
            formatter['input'] == dict(path=item['path'], sha256=item['sha256']), 'Formatter source/version identity differs')
    receipt_binding(qualification, formatter.get('version_receipt'), formatter['tool']['version'], 'Formatter version')
    binding = {field: deepcopy(value) for field, value in formatter.items() if field != 'receipt'}
    receipt_binding(qualification, formatter.get('receipt'), binding, 'Formatter')
    require(counter.get('definition') == DEFINITION and counter.get('function') in ('mask_source', 'mask_foreign') and
            nonempty(counter.get('argument')) and counter.get('diagnostics') == [] and
            counter.get('no_semantic_normalization') is True, 'Only an actual diagnostic-free lexical physical count is accepted')
    pinned(qualification, dict(path=counter.get('module_path'), sha256=counter.get('module_sha256')), 'Physical counter module')
    counted = {field: deepcopy(value) for field, value in counter.items() if field != 'receipt'}
    counted.update(input_sha256=formatter['output']['sha256'], physical_loc=item['physical_loc'])
    receipt_binding(qualification, counter.get('receipt'), counted, 'Physical counter')


def validate_exclusions(data):
    require(type(data.get('languages')) is list and all(type(item) is dict and allowed_language(item.get('key'))
            for item in data['languages']), 'V and Red language inventories are forbidden')
    require(type(data.get('programs')) is dict and all(type(row) is dict and allowed_language(row.get('language'))
            for row in data['programs'].values()), 'V and Red program inventories are forbidden')
    catalog(data)
    require(type(data.get('benchmarks')) is list and all(type(benchmark) is dict and
            type(benchmark.get('groups')) is list and all(type(group) is dict and allowed_language(group.get('language'))
            for group in benchmark['groups']) for benchmark in data['benchmarks']),
            'V and Red benchmark groups are forbidden')


def _stage(data, baseline, qualification):
    require(type(data) is dict and type(data.get('programs')) is dict and type(data.get('files')) is dict and
            type(data.get('provenance')) is dict, 'Mutable dataset maps required')
    require(type(baseline) is dict and type(baseline.get('data')) is dict and
            nonempty(baseline.get('path')) and digest(baseline.get('sha256')), 'Saved dataset wrapper required')
    validate_exclusions(data)
    validate_exclusions(baseline['data'])
    q = qualification
    require(type(q) is dict and q.get('status') == STATUS and q.get('chart_admission') is False and
            q.get('physical_definition') == DEFINITION and q.get('actual_application_processes') == 0 and
            type(q.get('actual_application_processes')) is int, 'Completed formatter-only qualification required')
    require(q.get('source_dataset_sha256') == baseline['sha256'] and
            q.get('source_dataset_path') == baseline['path'], 'Qualification names another saved dataset')
    require(type(q.get('pins')) is dict and q['pins'] and all(nonempty(path) and digest(value)
            for path, value in q['pins'].items()), 'Complete actual receipt/source/tool/config pins required')
    pinned(q, dict(path=baseline['path'], sha256=baseline['sha256']), 'Saved dataset')
    require(type(q.get('receipts')) is dict and set(q['receipts']) <= set(q['pins']), 'Unpinned decoded receipt object')
    artifact = q.get('artifact')
    require(type(artifact) is dict and nonempty(artifact.get('path')) and digest(artifact.get('sha256')),
            'Guarded qualification artifact provenance required')
    terminal_artifact = q.get('terminal_artifact')
    require(type(terminal_artifact) is dict and nonempty(terminal_artifact.get('path')) and
            digest(terminal_artifact.get('sha256')) and terminal_artifact['path'] != artifact['path'],
            'Independently read terminal artifact provenance required')
    artifact_pins = {artifact['path']: artifact['sha256'], terminal_artifact['path']: terminal_artifact['sha256']}
    require(type(q.get('provenance')) is dict and exact_value(q['provenance'], artifact_pins),
            'Qualification and terminal artifact provenance must match the guarded reads exactly')
    terminal = q.get('terminal')
    require(type(terminal) is dict and terminal.get('status') == STATUS and 'body_error' in terminal and terminal['body_error'] is None and
            terminal.get('integrity_errors') == [] and terminal.get('available_bytes_not_fully_pinned') is False and
            terminal.get('inherited_common_mutex') is True and terminal.get('verified_pins') == q['pins'],
            'Qualification terminal must close without body, integrity or pin errors')
    require(terminal.get('qualification') == artifact['path'] and
            terminal.get('qualification_sha256') == artifact['sha256'], 'Terminal names another qualification artifact')
    rows = q.get('rows')
    profiles = q.get('profile_keys')
    measured = q.get('measurement_identities')
    require(type(rows) is dict and rows and type(profiles) is list and all(nonempty(key) for key in profiles) and
            len(profiles) == len(set(profiles)) and type(measured) is dict and
            set(rows) == set(measured) == set(profiles), 'Exact reviewed profile and measurement inventory required')
    changes = {}
    for key in profiles:
        row = rows[key]
        require(type(row) is dict and row.get('key') == key and allowed_language(row.get('language')),
                'Invalid or excluded qualification profile')
        actual = identity(data, key)
        saved = identity(baseline['data'], key)
        require(exact_value(actual, saved) and exact_value(saved, measured[key]) and row['language'] == actual.get('language'),
                'Formatted qualification changed the saved measurement identity')
        current_files = catalog(data)[key]['files']
        require(current_files == catalog(baseline['data'])[key]['files'], 'Published source bundle changed')
        for published in current_files:
            require(source_identity(data, published) == source_identity(baseline['data'], published),
                    'Original source identity changed after formatter qualification')
        inventory = row.get('compiler_inventory')
        if inventory is not None:
            require(type(inventory) is dict and type(inventory.get('binding')) is dict,
                    'Independently reviewed compiler inventory descriptor required')
            evidence = dict(path=inventory.get('evidence_path'), sha256=inventory.get('evidence_sha256'),
                            binding_field=inventory.get('binding_field'), binding=inventory['binding'])
            receipt_binding(q, evidence, inventory['binding'], 'Compiler inventory')
        files = row.get('files')
        validate_bundle(data, key, files, inventory['binding'] if inventory is not None else None)
        if inventory is not None:
            require(inventory['evidence_path'] not in (artifact['path'], baseline['path']),
                    'Compiler inventory cannot be the qualification or source dataset artifact')
            require(all(inventory['evidence_path'] not in (
                item.get('formatter_evidence', {}).get('receipt', {}).get('path'),
                item.get('formatter_evidence', {}).get('version_receipt', {}).get('path'),
                item.get('counter_evidence', {}).get('receipt', {}).get('path'))
                for item in files if item.get('status') == 'qualified'),
                'Compiler inventory evidence must be independent of formatter/count receipts')
        forbidden = {artifact['path'], terminal_artifact['path'], baseline['path']}
        if inventory is not None:
            forbidden.add(inventory['evidence_path'])
        for item in files:
            if item.get('status') == 'qualified':
                forbidden.update(item['formatter_evidence'][field]['path'] for field in ('receipt', 'version_receipt'))
                forbidden.add(item['counter_evidence']['receipt']['path'])
        bound_execution = measured_execution(q, data, key, files,
            inventory['binding'] if inventory is not None else None, row.get('measurement_evidence'), forbidden)
        for item in files:
            validate_file(q, item)
        status = row.get('status')
        require(type(status) is str and status in STATUSES and type(row.get('complete')) is bool,
                'Invalid profile formatted LOC status/completeness')
        if status == 'qualified':
            require(inventory is not None and bound_execution and row['complete'] is True and all(item['status'] == 'qualified' for item in files) and
                    type(row.get('formatted_loc')) is int and row['formatted_loc'] > 0 and
                    row['formatted_loc'] == sum(item['physical_loc'] for item in files),
                    'Qualified formatted LOC requires every authored file and the exact positive bundle total')
        else:
            require(bound_execution or all(item['status'] == 'pending' for item in files),
                    'An unbound measured execution must leave every authored formatter input pending')
            reasons = row.get('reasons')
            file_statuses = {item['status'] for item in files if item['status'] != 'qualified'}
            expected_status = ('failed' if 'failed' in file_statuses else
                               'unavailable' if 'unavailable' in file_statuses else 'pending')
            require(row['complete'] is False and row.get('formatted_loc') is None and
                    file_statuses and status == expected_status and
                    type(reasons) is list and reasons and all(nonempty(reason) for reason in reasons),
                    'Unqualified bundle requires a null total, incomplete status and reasons')
        changes[key] = dict(formatted_loc=row['formatted_loc'], formatted_loc_status=status,
                            formatted_loc_complete=row['complete'], formatted_loc_evidence=deepcopy(row))
    provenance = deepcopy(data['provenance'])
    for path, value in q['pins'].items():
        require(path not in provenance or provenance[path] == value, 'Conflicting formatted LOC provenance')
        provenance[path] = value
    for path, value in artifact_pins.items():
        require(path not in provenance or provenance[path] == value, 'Conflicting formatted artifact provenance')
        provenance[path] = value
    return changes, provenance


def stage(data, baseline, qualification):
    """Return only proposed program overlay fields and merged provenance."""
    try:
        return _stage(data, baseline, qualification)
    except (KeyError, TypeError, IndexError, AttributeError) as failure:
        raise ValueError('Malformed formatted LOC qualification schema: ' + str(failure)) from None


def apply(data, baseline, qualification):
    """Validate the complete packet before committing any overlay fields."""
    changes, provenance = stage(data, baseline, qualification)
    program_changes = {key: deepcopy(fields) for key, fields in changes.items() if key in data['programs']}
    inventory_changes = {key: deepcopy(fields) for key, fields in changes.items() if key in data.get('program_inventory', {})}
    for key, fields in program_changes.items(): data['programs'][key].update(fields)
    for key, fields in inventory_changes.items(): data['program_inventory'][key].update(fields)
    data['provenance'].clear()
    data['provenance'].update(provenance)
    return dict(updated_programs=len(changes), qualified_programs=sum(
        fields['formatted_loc_status'] == 'qualified' for fields in changes.values()))
