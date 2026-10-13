"""Candidate strict eight-language importer; no unqualified target is admitted.

This additive importer is intentionally not connected to the existing builder
yet. It accepts an explicit reviewed publication packet, not parser proposals
or preparation reports. The caller owns the common timing mutex. V is never
accepted. Existing source files, measurements and Mech endpoints are preserved.
"""
from copy import deepcopy
import fcntl
import math
import hashlib
import json
import os
from pathlib import Path
import statistics

LANGUAGES = {'nim', 'crystal', 'pony', 'c3', 'gleam', 'clojure', 'prolog', 'rebol'}
BENCHMARKS = {'regexredux', 'revcomp', 'pidigits', 'spectralnorm', 'mandelbrot',
              'fasta', 'knucleotide', 'binarytrees', 'nbody', 'fannkuchredux'}
LOCK = '/tmp/mech-clbg-timing.lock'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def runtime_binding(row, authored_inputs):
    return dict(profile_id=row['id'], command=deepcopy(row['command']),
        binary_sha256=row['binary_sha256'], main_source_path=row['source'], main_source_sha256=row['source_sha256'],
        authored_inputs=deepcopy(authored_inputs), runtime_artifacts=deepcopy(row.get('runtime_artifacts', [])),
        source_loading_verified=True, startup_input_closure_verified=True)


def validate_runtime_load(item, counts, meta):
    row = item['row']; inventory = counts['compiler_bound_input_inventory'][row['id']]
    receipt = inventory['runtime_receipt']
    require(receipt['status'] == 'passed' and
            counts['pins'].get(receipt['path']) == receipt['sha256'] and
            meta['pins'].get(receipt['path']) == receipt['sha256'] and
            isinstance(receipt['binding_field'], str) and receipt['binding_field'].startswith('/') and
            receipt['binding']['source_loading_verified'] is True and
            receipt['binding']['startup_input_closure_verified'] is True and
            receipt['binding'] == runtime_binding(row, inventory['files']),
            'Original runtime/load receipt must bind complete scored command and all authored/artifact inputs')
    item['metric']['runtime_input_bundle'] = deepcopy(inventory['files'])
    item['metric']['runtime_load_evidence'] = deepcopy(receipt)


def validate_runtime_load_value(receipt, document):
    value = document
    require(receipt['status'] == 'passed' and isinstance(receipt['binding_field'], str) and
            receipt['binding_field'].startswith('/'), 'Actual runtime/load receipt pointer required')
    for raw in receipt['binding_field'][1:].split('/'):
        token = raw.replace('~1', '/').replace('~0', '~')
        if isinstance(value, list):
            require(token.isdigit() and str(int(token)) == token, 'Invalid runtime/load array pointer')
            value = value[int(token)]
        else:
            require(isinstance(value, dict) and token in value, 'Missing actual runtime/load field')
            value = value[token]
    require(isinstance(value, dict) and value.get('source_loading_verified') is True and
            value.get('startup_input_closure_verified') is True and value == receipt['binding'],
            'Declared runtime/load binding differs from actual pinned receipt value')


def validate_prolog_vm(prepared, counts, meta):
    """Require positive actual VM/QLF evidence, never infer AOT/JIT from a name."""
    evidence = meta['interpreted_vm_evidence']
    require(evidence['status'] == 'qualified-actual-interpreted-native-vm' and
            evidence['interpreted_vm_execution_verified'] is True and
            evidence['native_aot_proven'] is evidence['native_machine_code_jit_proven'] is False,
            'Actual interpreted VM evidence is required; QLF does not prove native AOT/JIT')
    review = evidence['review']
    require(review['status'] == 'passed' and review['review_identity'] and
            meta['pins'].get(review['evidence_path']) == review['evidence_sha256'],
            'Measured original VM/WAM execution needs separately pinned review')
    template = meta['measured_backend_template']
    require(template['verified'] is True and template['key'] == 'native-interpreter' and
            template['execution'] == 'interpreted' and meta['execution'] == 'Interpreted' and
            meta['scored_native_architecture'] == 'arm64' and meta['emulation_used'] is False,
            'Prolog measured backend must be explicitly interpreted on the native platform')
    require(len(evidence['profile_ids']) == len(set(evidence['profile_ids'])) == 20 and
            set(evidence['profile_ids']) == {item['row']['id'] for item in prepared},
            'Actual VM review must cover all twenty measured original profiles')
    for item in prepared:
        row = item['row']; inventory = counts['compiler_bound_input_inventory'][row['id']]
        require(row['command'][0] == evidence['runtime_binary'] and
                row['binary_sha256'] == evidence['runtime_binary_sha256'] and
                meta['pins'].get(evidence['runtime_binary']) == evidence['runtime_binary_sha256'],
                'Measured Prolog launcher must be the reviewed actual native VM image')
        artifacts = inventory['runtime_artifacts']
        require(type(artifacts) is list and artifacts and artifacts == row['runtime_artifacts'] and
                len({artifact['path'] for artifact in artifacts}) == len(artifacts),
                'Actual selected QLF artifacts must match each measured command inventory')
        for artifact in artifacts:
            require(artifact['kind'] == 'swi-qlf-bytecode' and
                    counts['pins'].get(artifact['path']) == artifact['sha256'] and
                    meta['pins'].get(artifact['path']) == artifact['sha256'],
                    'Every original scored QLF artifact needs input and runtime receipt pins')
        validate_runtime_load(item, counts, meta)
        item['metric']['runtime_artifacts'] = deepcopy(artifacts)


def validate_rebol_execution(prepared, counts, meta):
    """Rebol interpretation is positively reviewed against all original rows."""
    review = meta['execution_model_review']
    require(review['status'] == 'passed' and review['review_identity'] and
            meta['pins'].get(review['evidence_path']) == review['evidence_sha256'] and
            review['execution'] == meta['execution'] and
            len(review['profile_ids']) == len(set(review['profile_ids'])) == 20 and
            set(review['profile_ids']) == {item['row']['id'] for item in prepared},
            'Actual execution model review must cover all twenty original profiles')
    require(meta['scored_native_architecture'] == 'arm64' and meta['emulation_used'] is False,
            'A missing native ARM64 target cannot be replaced by an emulator')
    template = meta['measured_backend_template']
    require(meta['execution'] == 'Interpreted' and template['verified'] is True and
            template['key'] == 'native-interpreter' and template['execution'] == 'interpreted',
            'Rebol measured execution must be positively interpreted')
    vm = meta['interpreted_vm_evidence']
    require(vm['status'] == 'qualified-actual-interpreted-native-vm' and
            vm['interpreted_vm_execution_verified'] is True and
            vm['native_aot_proven'] is vm['native_machine_code_jit_proven'] is False,
            'Compiled host runtime does not establish native application AOT/JIT')
    for item in prepared:
        row = item['row']; inventory = counts['compiler_bound_input_inventory'][row['id']]
        require(row['command'][0] == vm['runtime_binary'] and
                row['binary_sha256'] == vm['runtime_binary_sha256'] and
                meta['pins'].get(vm['runtime_binary']) == vm['runtime_binary_sha256'],
                'Measured interpreter differs from its exact reviewed native image')
        validate_runtime_load(item, counts, meta)


def validate_cohort(language, suite, counts, meta):
    """Pure shape/identity checks after the caller verifies all file pins."""
    require(language in LANGUAGES, 'Only the eight requested publication languages are accepted')
    require(suite['language'] == language and
            suite['status'] == 'ten-native-workloads-reconciled-awaiting-canonical-LOC-and-publication',
            'Native records have not been independently reconciled')
    require(set(suite['benchmarks']) == BENCHMARKS and
            type(suite['canonical_full_native_processes']) is int and suite['canonical_full_native_processes'] == 90 and
            type(suite['candidate_sources']) is int and suite['candidate_sources'] == 20 and
            type(suite['actual_samples_per_source']) is int and suite['actual_samples_per_source'] == 3,
            'Complete ten-workload, twenty-source, ninety-process cohort required')
    require(suite['source_timing_relabels'] == suite['source_or_report_mutations'] == 0 and
            suite['chart_admitted'] is False, 'Reconciliation provenance changed')
    require(counts['language'] == language and counts['status'] == 'qualified-exact-source-LOC' and
            type(counts['profile_count']) is int and counts['profile_count'] == 20 and
            counts['canonical_LOC_admitted'] is True and counts['chart_admission'] is False and
            counts['complete_authored_source_bundles'] is True and counts['unresolved'] == [],
            'Canonical LOC needs complete source-specific semantic review')
    fixture = counts['fixture_receipt']
    require(fixture['status'] == 'passed' and counts['pins'].get(fixture['path']) == fixture['sha256'] and
            counts['grammar_snapshot_pins'] and counts['replay_receipts'] and
            all(counts['pins'].get(path) == digest for path,digest in counts['grammar_snapshot_pins'].items()),
            'Actual qualified grammar, fixture and exact-source replay receipts are required')
    for receipt in counts['replay_receipts']:
        require(counts['pins'].get(receipt['path']) == receipt['sha256'], 'Unbound actual source replay')
    require(meta['language'] == language and meta['status'] == 'reviewed-actual-native-language-metadata' and
            meta['actual_compiler_version_verified'] is True and
            meta['actual_native_architecture_verified'] is True and
            meta['actual_memory_policy_verified'] is True,
            'Planned compiler/runtime metadata cannot describe measured programs')
    require(type(meta['release_year']) is int and meta['release_source'].startswith('https://') and
            meta['release_note'] and meta['backend_label'] and meta['memory_label'] and
            meta['execution'] in ({'Interpreted'} if language in {'prolog', 'rebol'} else
                                 {'JIT'} if language in {'gleam', 'clojure'} else {'AOT'}),
            'Measured backend and sourced year are required')
    require(meta['color_source'] in ('GitHub Linguist', 'documented display fallback') and
            isinstance(meta['color'], str) and len(meta['color']) == 7 and meta['color'].startswith('#'),
            'Display color must have an explicit basis')
    require(set(meta['runtime_support_by_benchmark']) == BENCHMARKS and
            all(isinstance(value, str) and value for value in meta['runtime_support_by_benchmark'].values()) and
            isinstance(meta['timing_boundary'], str) and meta['timing_boundary'] and
            isinstance(meta['source_provenance'], str) and meta['source_provenance'] and meta['label'],
            'All displayed runtime/library and source provenance must be explicit')
    prepared, source_keys = [], set()
    for benchmark in sorted(BENCHMARKS):
        evidence = suite['benchmarks'][benchmark]
        profiles = list(evidence['profiles'].values())
        candidates = [row for row in profiles if row['language'] == language]
        controls = [row for row in profiles if row['language'] == 'mech']
        require(len(profiles) == 3 and len(candidates) == 2 and len(controls) == 1 and
                {row['variant'] for row in candidates} == {'compact', 'optimized'},
                'Both candidate forms and exact matched Mech control required')
        control = controls[0]
        require(len(control['samples_seconds']) == 3 and
                control['median_seconds'] == statistics.median(control['samples_seconds']),
                'Matched Mech control is incomplete')
        for row in candidates:
            require(row['benchmark'] == benchmark and row['eligible_for_chart'] is False and
                    row['publication_status'] == 'records-reconciled-awaiting-qualified-canonical-LOC',
                    'Candidate was not preserved through independent reconciliation')
            samples = row['samples_seconds']
            require(len(samples) == len(row['sample_validation']) == len(row['sample_execution_metadata']) == 3 and
                    all(type(value) in (int, float) and math.isfinite(value) and value > 0 for value in samples) and
                    row['median_seconds'] == statistics.median(samples) and
                    row['sample_stdev_seconds'] == statistics.stdev(samples) and
                    all(check['reference_match'] is True for check in row['sample_validation']),
                    'Actual complete samples and medians differ')
            key = row['id']
            require(isinstance(key,str) and key and row['key'] == benchmark+':'+key and
                    key not in source_keys and key in counts['profiles'], 'Duplicate, inconsistent or uncounted source identity')
            source_keys.add(key)
            metric = deepcopy(counts['profiles'][key])
            semantic = metric['semantic_review']
            require(metric['source'] == row['source'] and metric['source_sha256'] == row['source_sha256'] and
                    metric['benchmark'] == benchmark and metric['variant'] == row['variant'] and
                    semantic['status'] == 'passed' and semantic['review_identity'] and
                    counts['pins'].get(semantic['evidence_path']) == semantic['evidence_sha256'],
                    'Canonical review names another compiled source or lacks semantic evidence')
            files = metric['source_bundle']
            require(files and files[0]['path'] == row['source'] and files[0]['sha256'] == row['source_sha256'] and
                    len({item['path'] for item in files}) == len(files), 'Exact counted application bundle required')
            inventory = counts['compiler_bound_input_inventory'][key]
            require(inventory['status'] == 'qualified-exact-authored-compiler-inputs' and
                    inventory['source_sha256'] == row['source_sha256'] and
                    inventory['binary_sha256'] == row['binary_sha256'] and
                    counts['pins'].get(inventory['evidence_path']) == inventory['evidence_sha256'] and
                    inventory['files'], 'An authoritative compiler-bound source/helper inventory is required')
            expected_bundle = {(item['path'], item['sha256']) for item in inventory['files']}
            require(len(expected_bundle) == len(inventory['files']) and
                    {(item['path'], item['sha256']) for item in files} == expected_bundle,
                    'Canonical bundle omitted a compiled application/helper file')
            if row.get('source_bundle'):
                require(expected_bundle == {(item['path'], item['sha256']) for item in row['source_bundle']},
                        'Compiler-bound inventory differs from measured compiled inputs')
            require(all(type(item['normalized_loc']) is int and item['normalized_loc'] > 0 and
                        type(item['physical_loc']) is int and item['physical_loc'] > 0 and
                        item['role'] in ('application', 'application-helper') and
                        item['ownership_review']['status'] == 'passed' and item['ownership_review']['review_identity'] and
                        counts['pins'].get(item['ownership_review']['evidence_path']) == item['ownership_review']['evidence_sha256'] and
                        counts['pins'].get(item['count_receipt']['path']) == item['count_receipt']['sha256'] and
                        item['unresolved_diagnostics'] == [] for item in files), 'An authored source/helper is unreviewed')
            require(type(metric['canonical_loc']) is int and type(metric['physical_loc']) is int and
                    metric['canonical_loc'] == sum(item['normalized_loc'] for item in files) and
                    metric['physical_loc'] == sum(item['physical_loc'] for item in files),
                    'Counted bundle totals differ from reviewed file units')
            metric.update(normalized_loc=metric['canonical_loc'], compiled_application_files=files)
            prepared.append(dict(row=deepcopy(row), metric=deepcopy(metric),
                benchmark=benchmark, matched_control=deepcopy(control), evidence=deepcopy(evidence)))
    require(len(prepared) == len(counts['profiles']) == len(source_keys) == 20 and
            set(counts['profiles']) == source_keys, 'Canonical source inventory must match all twenty measured programs')
    if language == 'prolog': validate_prolog_vm(prepared, counts, meta)
    if language == 'rebol': validate_rebol_execution(prepared, counts, meta)
    return prepared


def held_mutex(fd):
    require(type(fd) is int, 'Publication source packaging requires the caller-held timing mutex')
    expected, actual = os.stat(LOCK), os.fstat(fd)
    require((expected.st_dev, expected.st_ino) == (actual.st_dev, actual.st_ino), 'Different timing descriptor')
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        raise ValueError('Source packaging must wait for the common timing boundary') from None


def add_native_languages(root, data, *, packet_path, common_timing_mutex_fd):
    """Stage every source/read/metadata change, then commit into existing maps.

    Do not call augment_data's mutation callbacks: they capture the live input
    maps. Independent staging also preserves metadata for preexisting file IDs.
    """
    held_mutex(common_timing_mutex_fd)
    root = Path(root).resolve()
    staged = deepcopy(data)
    def sha(path):
        with Path(path).open('rb') as stream:
            return hashlib.file_digest(stream, 'sha256').hexdigest()
    def load(path):
        path = Path(path).resolve(strict=True)
        require(path.is_relative_to(root), 'Publication artifact escaped the benchmark root')
        value = json.loads(path.read_text())
        staged['provenance'][path.relative_to(root).as_posix()] = sha(path)
        return value
    packet_path = Path(packet_path).resolve(strict=True)
    require(packet_path.is_relative_to(root / 'continuation'), 'Publication packet must be the reviewed local artifact')
    packet_name = packet_path.relative_to(root).as_posix()
    packet = load(packet_path)
    require(packet['status'] == 'reviewed-eight-language-publication' and
            set(packet['languages']) == LANGUAGES and packet['unresolved'] == [],
            'The reviewed packet must cover all eight requested languages')
    require(set(b['key'] for b in data['benchmarks']) == BENCHMARKS and
            not any(item['key'] in LANGUAGES | {'v'} for item in data['languages']) and
            not any(item['key'] == 'red' for item in data['languages']) and
            not any(row['language'] in {'v','red'} for row in data['programs'].values()) and
            not any(group['language'] in {'v','red'} for b in data['benchmarks'] for group in b['groups']),
            'Unexpected workload, duplicate requested language or excluded visualization entry')
    pending = []
    pending_program_keys = set(data['programs'])
    pending_program_ids = {row['id'] for row in data['programs'].values() if 'id' in row}
    for language in sorted(LANGUAGES):
        entry = packet['languages'][language]
        artifacts = {}
        for kind in ('suite', 'canonical_loc', 'metadata'):
            filename = Path(entry[kind]['path']).resolve(strict=True)
            require(filename.is_relative_to(root) and sha(filename) == entry[kind]['sha256'],
                    'Reviewed publication artifact changed')
            value = load(filename)
            pins = value.get('provenance', value.get('pins', {}))
            require(isinstance(pins, dict) and pins, 'Reviewed artifact lacks actual source/runtime/proof pins')
            for path, digest in pins.items():
                require(sha(path) == digest, 'Source/runtime/proof pin changed: ' + path)
            artifacts[kind] = value
        rows = validate_cohort(language, artifacts['suite'], artifacts['canonical_loc'], artifacts['metadata'])
        for item in rows:
            row, metric, evidence = item['row'], item['metric'], item['evidence']
            require(row['key'] not in pending_program_keys and row['id'] not in pending_program_ids,
                    'Duplicate source key or identity across publication cohorts')
            pending_program_keys.add(row['key'])
            pending_program_ids.add(row['id'])
            require(sha(row['command'][0]) == row['binary_sha256'], 'Changed measured executable')
            if language in {'prolog','rebol'}:
                receipt = metric['runtime_load_evidence']
                path = Path(receipt['path']).resolve(strict=True)
                require(path.is_relative_to(root) and sha(path) == receipt['sha256'], 'Original runtime/load receipt changed')
                validate_runtime_load_value(receipt, json.loads(path.read_bytes()))
            for artifact in metric.get('runtime_artifacts', []):
                path = Path(artifact['path']).resolve(strict=True)
                require(path.is_relative_to(root) and sha(path) == artifact['sha256'],
                        'Changed original scored Prolog QLF artifact')
            # Preflight every path conversion and source read before committing.
            report = Path(evidence['report']).resolve(strict=True)
            counts = Path(entry['canonical_loc']['path']).resolve(strict=True)
            require(report.is_relative_to(root) and sha(report) == evidence['report_sha256'], 'Actual measured report changed')
            item.update(report_name=report.relative_to(root).as_posix(), counts_name=counts.relative_to(root).as_posix())
            item['source_files'] = []
            for source in metric['compiled_application_files']:
                path = Path(source['path']).resolve(strict=True)
                require(path.is_relative_to(root) and sha(path) == source['sha256'], 'Counted application/helper changed')
                item['source_files'].append(dict(source, code=path.read_text(), public_path=path.relative_to(root).as_posix()))
        pending.append((language, entry, artifacts, rows))
    audits = {}
    for language, entry, artifacts, rows in pending:
        meta = artifacts['metadata']
        staged['languages'].append(dict(key=language, label=meta['label'], color=meta['color'],
            execution=meta['execution'], line_style='solid', runtime_policy=meta['backend_label'],
            release_year=meta['release_year'], release_name=meta['label'],
            release_source=meta['release_source'], release_note=meta['release_note'],
            coverage_note='All ten workloads: three complete full-process samples per source.'))
        for item in rows:
            row, metric = item['row'], item['metric']
            ids, file_roles = [], {}
            for index, source in enumerate(item['source_files']):
                identity = source['sha256']
                ids.append(identity)
                note = 'Authored application source; included in canonical LOC.' if index == 0 else 'Authored application/helper/ABI declarations; included in canonical LOC.'
                # Reused content IDs keep their earlier metadata exactly intact.
                if identity not in staged['files']:
                    staged['files'][identity] = dict(name=Path(source['path']).name,
                        path=source['public_path'], sha256=identity, code=source['code'],
                        role=source['role'], loc_note=note, normalized_loc=source['normalized_loc'], physical_loc=source['physical_loc'])
                file_roles[identity] = dict(role=source['role'], loc_note=note,
                    normalized_loc=source['normalized_loc'], physical_loc=source['physical_loc'])
            staged['programs'][row['key']] = dict(key=row['key'], id=row['id'], benchmark=row['benchmark'],
                language=language, loc=metric['normalized_loc'], seconds=row['median_seconds'],
                samples=list(row['samples_seconds']), timing_sd=row['sample_stdev_seconds'],
                files=ids, file_roles=file_roles, report=item['report_name'], source_report=item['counts_name'],
                measurement_status='measured', physical_loc=metric['physical_loc'],
                loc_metric='statement-normalized', loc_report=item['counts_name'], loc_expansions=metric['expansions'],
                binary_sha256=row['binary_sha256'], required_work_validated=True,
                full_output_validation=deepcopy(row['sample_validation']),
                sample_execution_metadata=deepcopy(row['sample_execution_metadata']), runtime_arguments=list(row['command']),
                execution_mode=meta['execution'].lower(), runtime_policy=meta['backend_label'], memory_policy=meta['memory_label'],
                runtime_support=meta['runtime_support_by_benchmark'][row['benchmark']],
                timing_boundary=meta['timing_boundary'], timing_summary=meta['timing_boundary'],
                source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/' + row['benchmark'] + '.html',
                source_link_label='Benchmark requirements', local_adaptation=True, source_provenance=meta['source_provenance'],
                matched_mech_control=deepcopy(item['matched_control']), time_ratio_to_matched_mech=row['time_ratio_to_matched_mech'])
            if language == 'prolog':
                staged['programs'][row['key']]['runtime_artifacts'] = deepcopy(metric['runtime_artifacts'])
            if language in {'prolog','rebol'}:
                staged['programs'][row['key']]['runtime_input_bundle'] = deepcopy(metric['runtime_input_bundle'])
                staged['programs'][row['key']]['runtime_load_evidence'] = deepcopy(metric['runtime_load_evidence'])
                staged['programs'][row['key']]['source_path'] = row['source']
            benchmark = next(b for b in staged['benchmarks'] if b['key'] == row['benchmark'])
            group = next((g for g in benchmark['groups'] if g['language'] == language), None)
            if group is None:
                group = dict(language=language, label=meta['label'], programs=[])
                benchmark['groups'].append(group)
            group['programs'].append(row['key'])
        audits[language] = dict(source_count=20, actual_full_native_processes=90,
            sample_count=3, artifacts=deepcopy(entry), source_or_timing_mutations=0,
            matched_controls='Fresh unchanged Mech source and executable in each benchmark’s three rotated rounds.')
    staged['native_language_audit'] = dict(packet=packet_name, languages=audits,
        V_visualization_entries=0, statement_normalized_authored_source_bundles=True)
    staged['provenance'][Path(__file__).name] = sha(Path(__file__))
    staged['protocol']['native_languages'] = ('Nim, Crystal, Pony, C3, Gleam, Clojure, Prolog and Rebol each cover all ten canonical workloads with two locally authored '
        'application forms, three actual fresh-process samples per form and matched unchanged Mech AOT controls. '
        'All authored application/helpers/ABI glue count; pinned general vendor libraries supply ordinary operations. '
        'Measured compiler, runtime platform, memory, library and source provenance are shown per program. '
        'Prolog runs reviewed QLF/WAM bytecode in the actual native SWI-Prolog VM. '
        'Rebol interpretation describes the actual reviewed scored commands; compiling a host interpreter does not establish application AOT. '
        'QLF compilation is not application native AOT, and clause indexing does not establish native-code JIT. '
        'V is report-only and has no visualization entry.')
    # Keep map/list identities used by the surrounding existing builder. There
    # are no file reads, hashes or callbacks after this commit begins.
    for key in ('programs', 'files', 'provenance', 'protocol'):
        data[key].clear()
        data[key].update(staged[key])
    for key in ('languages', 'benchmarks'):
        data[key][:] = staged[key]
    data['native_language_audit'] = staged['native_language_audit']
    return audits
