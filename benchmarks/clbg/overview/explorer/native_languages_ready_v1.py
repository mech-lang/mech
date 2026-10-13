"""Publish explicitly reviewed complete cohorts without waiting for other languages.

Source-only candidate. The eight-language validator and platform/group functions
remain unchanged. Each selected cohort still needs all 20 sources, 90 original
processes, three samples, whole authored LOC and actual runtime evidence.
"""
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
from types import FunctionType

import native_languages_data_v2 as original
import native_platform_metadata_v2 as platform
import native_language_groups_v2 as groups
import publication_endpoints_v1 as endpoints

LANGUAGES = frozenset(original.LANGUAGES)
FUTURE_LANGUAGES = frozenset({'elixir', 'kotlin'})
REQUESTED_LANGUAGES = LANGUAGES | FUTURE_LANGUAGES
require = original.require


def validate_selection(packet):
    require(packet['status'] == 'reviewed-completed-native-language-subset' and
            packet['unresolved'] == [] and isinstance(packet['languages'], dict),
            'Explicit reviewed complete-cohort packet required')
    selected = set(packet['languages'])
    require(selected and selected <= LANGUAGES and
            packet['ready_languages'] == sorted(selected),
            'Only a nonempty explicitly reviewed subset of the original eight is ready')
    require(packet.get('future_requested_languages', []) in ([], ['elixir', 'kotlin']),
            'Future language scope cannot masquerade as measured entries')
    for entry in packet['languages'].values():
        require(set(entry) == {'suite', 'canonical_loc', 'metadata'} and
                all(isinstance(item, dict) and set(item) == {'path', 'sha256'} and
                    isinstance(item['path'], str) and platform.digest(item['sha256'])
                    for item in entry.values()), 'Three exact reviewed cohort artifacts required')
    return selected


def private_function(function, namespace):
    copied = FunctionType(function.__code__, namespace, function.__name__, function.__defaults__, function.__closure__)
    copied.__kwdefaults__ = deepcopy(function.__kwdefaults__)
    return copied


def subset_platform_stage(data, metadata, actual, provenance):
    """Bind only the inventory globals; original executable validators stay exact."""
    selected = set(metadata)
    require(selected and selected <= LANGUAGES, 'Unknown ready runtime cohort')
    namespace = dict(platform.stage.__globals__)
    namespace.update(LANGUAGES=selected,
        BACKEND_OPTIONS={key: deepcopy(platform.BACKEND_OPTIONS[key]) for key in selected})
    namespace['selected_backends'] = private_function(platform.selected_backends, namespace)
    stage = private_function(platform.stage, namespace)
    view = deepcopy(data)
    view['native_language_audit']['languages'] = {
        key: deepcopy(data['native_language_audit']['languages'][key]) for key in selected}
    return stage(view, metadata, actual, provenance)


def add_ready_groups(staged, selected):
    namespace = dict(groups.add_native_language_groups.__globals__)
    namespace.update(LANGUAGES=set(selected),
        ADDITIONS={key: set(value) & set(selected) for key, value in groups.ADDITIONS.items()},
        LANGUAGE_FACTS={key: deepcopy(value) for key, value in groups.LANGUAGE_FACTS.items() if key in selected})
    view = deepcopy(staged)
    view['native_language_audit']['languages'] = {
        key: deepcopy(staged['native_language_audit']['languages'][key]) for key in selected}
    private_function(groups.add_native_language_groups, namespace)(view)
    staged['language_groups'] = view['language_groups']
    staged['protocol']['language_groups'] = view['protocol']['language_groups']
    facts = deepcopy(staged.get('native_language_filter_facts', {}))
    for key, value in view['native_language_filter_facts'].items():
        require(key not in facts or facts[key] == value, 'Conflicting existing source trait evidence')
        facts[key] = value
    staged['native_language_filter_facts'] = facts


def program_record(language, meta, item):
    row, metric = item['row'], item['metric']
    ids, roles = [], {}
    for index, source in enumerate(item['source_files']):
        identity = source['sha256']; ids.append(identity)
        note = ('Authored application source; included in canonical LOC.' if index == 0 else
                'Authored application/helper/ABI declarations; included in canonical LOC.')
        roles[identity] = dict(role=source['role'], loc_note=note,
            normalized_loc=source['normalized_loc'], physical_loc=source['physical_loc'])
    result = dict(key=row['key'], id=row['id'], benchmark=row['benchmark'], language=language,
        loc=metric['normalized_loc'], seconds=row['median_seconds'], samples=list(row['samples_seconds']),
        timing_sd=row['sample_stdev_seconds'], files=ids, file_roles=roles,
        report=item['report_name'], source_report=item['counts_name'], measurement_status='measured',
        physical_loc=metric['physical_loc'], loc_metric='statement-normalized', loc_report=item['counts_name'],
        loc_expansions=deepcopy(metric['expansions']), binary_sha256=row['binary_sha256'], required_work_validated=True,
        full_output_validation=deepcopy(row['sample_validation']),
        sample_execution_metadata=deepcopy(row['sample_execution_metadata']), runtime_arguments=list(row['command']),
        execution_mode=meta['execution'].lower(), runtime_policy=meta['backend_label'], memory_policy=meta['memory_label'],
        runtime_support=meta['runtime_support_by_benchmark'][row['benchmark']], timing_boundary=meta['timing_boundary'],
        timing_summary=meta['timing_boundary'], source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+row['benchmark']+'.html',
        source_link_label='Benchmark requirements', local_adaptation=True, source_provenance=meta['source_provenance'],
        matched_mech_control=deepcopy(item['matched_control']), time_ratio_to_matched_mech=row['time_ratio_to_matched_mech'])
    if language == 'prolog': result['runtime_artifacts'] = deepcopy(metric['runtime_artifacts'])
    if language in {'prolog', 'rebol'}:
        result.update(runtime_input_bundle=deepcopy(metric['runtime_input_bundle']),
            runtime_load_evidence=deepcopy(metric['runtime_load_evidence']), source_path=row['source'])
    return result


def prepare_ready(data, entries, packet_name, provenance):
    """Pure all-cohort staging; existing measurements and file metadata survive."""
    selected = set(entries)
    require(selected and selected <= LANGUAGES, 'Unknown or empty ready language inventory')
    staged = deepcopy(data)
    require({item['key'] for item in staged['benchmarks']} == original.BENCHMARKS and
            len(staged['benchmarks']) == len(original.BENCHMARKS), 'Exact existing workload inventory required')
    existing_languages = {item['key']: item for item in staged['languages']}
    require(len(existing_languages) == len(staged['languages']) and not {'v','red'} & set(existing_languages) and
            not any(row['language'] in {'v','red'} for row in staged['programs'].values()) and
            not any(group['language'] in {'v','red'} for benchmark in staged['benchmarks'] for group in benchmark['groups']),
            'Duplicate or excluded visualization language')
    audit = staged.setdefault('native_language_audit', dict(languages={}, V_visualization_entries=0,
        statement_normalized_authored_source_bundles=True))
    require(set(audit['languages']) <= LANGUAGES, 'Unknown existing native cohort audit')
    old_ids = {}
    for key, row in staged['programs'].items():
        require(key == row['key'], 'Program map identity differs')
        if 'id' in row:
            old_ids.setdefault(row['id'], set()).add(key)
    metadata, actual = {}, {}
    for language in sorted(selected):
        item = entries[language]; meta = item['meta']; rows = item['rows']; entry = item['entry']
        require(len(rows) == 20 and {row['row']['language'] for row in rows} == {language} and
                len({row['row']['key'] for row in rows}) == len({row['row']['id'] for row in rows}) == 20,
                'Ready import must retain all twenty original source identities')
        incoming_audit = dict(source_count=20, actual_full_native_processes=90, sample_count=3,
            artifacts=deepcopy(entry), source_or_timing_mutations=0,
            matched_controls='Fresh unchanged Mech source and executable in each benchmark’s three rotated rounds.')
        if language in audit['languages']:
            existing = audit['languages'][language]
            require(all(existing.get(key) == value for key, value in incoming_audit.items()),
                    'Existing cohort does not match the same reviewed source/timing artifacts')
        else:
            require(language not in existing_languages and
                    not any(row['language'] == language for row in staged['programs'].values()) and
                    not any(group['language'] == language for benchmark in staged['benchmarks'] for group in benchmark['groups']),
                    'An existing language requires its original complete admission audit')
            audit['languages'][language] = incoming_audit
        language_record = dict(key=language, label=meta['label'], color=meta['color'], execution=meta['execution'],
            line_style='solid', runtime_policy=meta['backend_label'], release_year=meta['release_year'],
            release_name=meta['label'], release_source=meta['release_source'], release_note=meta['release_note'],
            coverage_note='All ten workloads: three complete full-process samples per source.')
        if language in existing_languages:
            require(all(existing_languages[language].get(key) == value for key, value in language_record.items()),
                    'Existing language display metadata differs from actual reviewed cohort')
        else:
            staged['languages'].append(language_record); existing_languages[language] = language_record
        cohort_keys = {row['row']['key'] for row in rows}
        for benchmark in staged['benchmarks']:
            matching = [group for group in benchmark['groups'] if group['language'] == language]
            require(len(matching) <= 1 and all(key in cohort_keys for group in matching for key in group['programs']),
                    'Existing selected endpoint group contains another source identity')
        for row in rows:
            record = program_record(language, meta, row); key, identity = record['key'], record['id']
            require(identity not in old_ids or old_ids[identity] == {key}, 'Cross-cohort source identity collision')
            if key in staged['programs']:
                require(all(staged['programs'][key].get(field) == value for field, value in record.items()),
                        'An existing observation cannot be overwritten by a new import')
            else: staged['programs'][key] = record
            old_ids[identity] = {key}
            for source in row['source_files']:
                identity = source['sha256']
                if identity not in staged['files']:
                    staged['files'][identity] = dict(name=Path(source['path']).name, path=source['public_path'],
                        sha256=identity, code=source['code'], role=source['role'], loc_note=record['file_roles'][identity]['loc_note'],
                        normalized_loc=source['normalized_loc'], physical_loc=source['physical_loc'])
            benchmark = next(value for value in staged['benchmarks'] if value['key'] == record['benchmark'])
            group = next((value for value in benchmark['groups'] if value['language'] == language), None)
            if group is None:
                group = dict(language=language, label=meta['label'], programs=[]); benchmark['groups'].append(group)
            if key not in group['programs']: group['programs'].append(key)
        metadata[language] = deepcopy(meta); actual[language] = deepcopy(item['actual_runtime_report'])
    for path, digest in provenance.items():
        require(path not in staged['provenance'] or staged['provenance'][path] == digest, 'Conflicting publication provenance')
        staged['provenance'][path] = digest
    changes, memory, reconciled = subset_platform_stage(staged, metadata, actual, provenance)
    for key, record in changes.items(): staged['programs'][key]['measured_backend'] = record
    for item in staged['languages']:
        if item['key'] in memory:
            require('measured_memory_policy' not in item or item['measured_memory_policy'] == memory[item['key']],
                    'Existing memory evidence conflicts')
            item['measured_memory_policy'] = memory[item['key']]
    staged['provenance'] = reconciled
    add_ready_groups(staged, selected)
    ready = deepcopy(staged.get('native_ready_publication_audit', dict(packets={}, languages={})))
    ready['packets'][packet_name] = sorted(selected)
    for language in selected: ready['languages'][language] = deepcopy(audit['languages'][language])
    ready.update(full_requested_languages=sorted(REQUESTED_LANGUAGES),
        source_or_timing_mutations=0, V_visualization_entries=0, Red_visualization_entries=0)
    staged['native_ready_publication_audit'] = ready
    admitted = sorted(audit['languages'])
    staged['protocol']['native_ready_languages'] = (', '.join(existing_languages[key]['label'] for key in admitted) +
        ': complete ten-workload cohorts, two original source forms, three full-process samples per form, '
        'whole authored canonical LOC and actual measured runtime evidence. Other requested languages remain pending; '
        'V is report-only and Red is excluded.')
    return staged


def finalize_ready_endpoints(data, selected):
    """Use frozen selection gates, keeping existing cohort baselines/rows exact."""
    selected = set(selected); require(selected and selected <= LANGUAGES, 'Unknown ready endpoints')
    destination = data
    data = deepcopy(destination)
    view = deepcopy(data)
    view['languages'] = [item for item in view['languages'] if item['key'] in selected]
    view['programs'] = {key: row for key, row in view['programs'].items() if row['language'] in selected}
    for benchmark in view['benchmarks']:
        benchmark['groups'] = [group for group in benchmark['groups'] if group['language'] in selected]
    endpoints.finalize_endpoints(view)
    for benchmark in data['benchmarks']:
        finalized = next(item for item in view['benchmarks'] if item['key'] == benchmark['key'])
        for group in benchmark['groups']:
            if group['language'] in selected:
                chosen = next(item for item in finalized['groups'] if item['language'] == group['language'])
                group.update(deepcopy(chosen))
        require(type(benchmark['baseline_loc']) is int and benchmark['baseline_loc'] > 0 and
                endpoints.positive_time(benchmark['baseline_seconds']), 'Existing fixed endpoint baseline required')
    retained = {key for benchmark in data['benchmarks'] for group in benchmark['groups'] for key in group['programs']}
    for key in list(data['programs']):
        row = data['programs'][key]
        if row['language'] not in selected: continue
        if key not in retained: del data['programs'][key]; continue
        benchmark = next(item for item in data['benchmarks'] if item['key'] == row['benchmark'])
        row.update(x=row['loc']/benchmark['baseline_loc'], y=row['seconds']/benchmark['baseline_seconds'])
    # Retained original file metadata is never rewritten or deleted here.
    summaries = [deepcopy(item) for item in data.get('reference_summary', []) if item['lang'] not in selected]
    for language in data['languages']:
        if language['key'] not in selected: continue
        language_groups = [next(group for group in benchmark['groups'] if group['language'] == language['key'])
            for benchmark in data['benchmarks']]
        language['timed_benchmarks'] = [benchmark['key'] for benchmark in data['benchmarks']]
        language['source_benchmarks'] = list(language['timed_benchmarks'])
        summaries.append(dict(lang=language['key'], label=language['label'], **{
            kind: {axis: math.exp(sum(math.log(data['programs'][group[kind]][axis]) for group in language_groups)/len(language_groups))
                for axis in ('x', 'y')} for kind in ('fast', 'small')}))
    data['reference_summary'] = summaries
    destination['programs'].clear(); destination['programs'].update(data['programs'])
    destination['languages'][:] = data['languages']
    destination['benchmarks'][:] = data['benchmarks']
    destination['reference_summary'] = data['reference_summary']
    return dict(ready_languages=sorted(selected), retained_ready_programs=sum(row['language'] in selected for row in data['programs'].values()),
        unchanged_existing_baselines=True, source_or_timing_mutations=0)


def add_ready_native_languages(root, data, *, packet_path, packet_sha256, common_timing_mutex_fd, finalize=False):
    """Read and verify everything, stage import/platform/groups/endpoints, commit once."""
    original.held_mutex(common_timing_mutex_fd); root = Path(root).resolve(strict=True)
    pins, provenance = {}, {}
    def read(path, expected=None, project=True):
        path = Path(path).resolve(strict=True)
        require(not project or path.is_relative_to(root), 'Ready artifact escaped benchmark root')
        raw = path.read_bytes(); digest = hashlib.sha256(raw).hexdigest()
        require(expected is None or platform.digest(expected) and expected == digest, 'Changed ready artifact: '+str(path))
        require(str(path) not in pins or pins[str(path)] == digest, 'Conflicting ready artifact pin')
        pins[str(path)] = digest
        if path.is_relative_to(root): provenance[path.relative_to(root).as_posix()] = digest
        return raw
    packet_path = Path(packet_path).resolve(strict=True)
    require(packet_path.is_relative_to(root/'continuation') and platform.digest(packet_sha256), 'Reviewed local ready packet SHA required')
    packet = json.loads(read(packet_path, packet_sha256)); selected = validate_selection(packet)
    entries = {}
    for language in sorted(selected):
        entry = packet['languages'][language]; artifacts = {}
        for kind in ('suite', 'canonical_loc', 'metadata'):
            descriptor = entry[kind]; value = json.loads(read(descriptor['path'], descriptor['sha256']))
            closure = value.get('provenance', value.get('pins', {}))
            require(isinstance(closure, dict) and closure, 'Actual source/runtime/proof pin closure required')
            for path, digest in closure.items(): read(path, digest, project=False)
            artifacts[kind] = value
        meta = artifacts['metadata']; rows = original.validate_cohort(language, artifacts['suite'], artifacts['canonical_loc'], meta)
        runtime = meta['measured_backend_template']['runtime_receipt']
        require(meta['pins'].get(runtime['report']) == runtime['report_sha256'], 'Actual runtime receipt omitted from metadata pins')
        actual = json.loads(read(runtime['report'], runtime['report_sha256']))
        for item in rows:
            row, metric, evidence = item['row'], item['metric'], item['evidence']
            read(row['command'][0], row['binary_sha256'], project=False)
            if language in {'prolog','rebol'}:
                load = metric['runtime_load_evidence']
                original.validate_runtime_load_value(load, json.loads(read(load['path'], load['sha256'])))
            for artifact in metric.get('runtime_artifacts', []): read(artifact['path'], artifact['sha256'])
            read(evidence['report'], evidence['report_sha256'])
            item.update(report_name=Path(evidence['report']).resolve().relative_to(root).as_posix(),
                counts_name=Path(entry['canonical_loc']['path']).resolve().relative_to(root).as_posix(), source_files=[])
            for source in metric['compiled_application_files']:
                raw = read(source['path'], source['sha256'])
                item['source_files'].append(dict(source, code=raw.decode('utf-8'),
                    public_path=Path(source['path']).resolve().relative_to(root).as_posix()))
        display_meta = deepcopy(meta)
        display_meta['measured_backend_template']['runtime_receipt']['report'] = Path(runtime['report']).resolve().relative_to(root).as_posix()
        display_meta['publication_metadata_evidence'] = dict(path=Path(entry['metadata']['path']).resolve().relative_to(root).as_posix(),
            sha256=entry['metadata']['sha256'])
        entries[language] = dict(entry=deepcopy(entry), meta=display_meta, rows=rows, actual_runtime_report=actual)
    # Pin actual adapter dependencies before any staged changes are committed.
    for module in (original, platform, groups, endpoints): read(module.__file__, project=True)
    read(__file__)
    staged = prepare_ready(data, entries, packet_path.relative_to(root).as_posix(), provenance)
    if finalize: finalize_ready_endpoints(staged, selected)
    for path, digest in pins.items():
        require(hashlib.sha256(Path(path).read_bytes()).hexdigest() == digest, 'Ready prerequisite changed before commit')
    for key in ('programs', 'files', 'provenance', 'protocol'):
        data[key].clear(); data[key].update(staged[key])
    for key in ('languages', 'benchmarks', 'language_groups'):
        data[key][:] = staged[key]
    for key in ('native_language_audit', 'native_ready_publication_audit', 'native_language_filter_facts', 'reference_summary'):
        if key in staged: data[key] = staged[key]
    return dict(ready_languages=sorted(selected), full_requested_languages=sorted(REQUESTED_LANGUAGES),
        actual_full_native_processes=90*len(selected), source_count=20*len(selected), samples_per_source=3,
        unchanged_existing_baselines=True, source_or_timing_mutations=0, V_visualization_entries=0, Red_visualization_entries=0)
