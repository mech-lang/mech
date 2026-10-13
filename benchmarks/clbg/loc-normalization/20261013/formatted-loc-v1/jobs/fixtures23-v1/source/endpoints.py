"""Select canonical and separately qualified formatted LOC endpoints.

The caller must supply the full admitted candidate pool before canonical
fast/small pruning. Source, measurement and formatter receipt admission belong
to the caller. This selector performs no reads, formatting or qualification.
It stages every validation and calculation before changing the supplied data.
"""
from copy import deepcopy
import math
import statistics
from overlay import catalog


FORMATTED_STATUSES = frozenset(('qualified', 'unavailable', 'pending', 'failed'))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def positive_time(value):
    return type(value) in (int, float) and math.isfinite(value) and value > 0


def excluded_language(key):
    return type(key) is str and key.casefold() in ('v', 'red')


def formatted_candidate(row):
    """Validate the display contract, without accepting formatter evidence.

    Old rows with no overlay fields are pending. A qualified record must have
    a complete positive integer count and an evidence object; the adapter must
    have already qualified that object's source, formatter and file receipts.
    No failed, partial or unavailable count becomes a canonical LOC fallback.
    """
    status = row.get('formatted_loc_status', 'pending')
    loc = row.get('formatted_loc')
    complete = row.get('formatted_loc_complete', False)
    evidence = row.get('formatted_loc_evidence', {})
    require(type(status) is str and status in FORMATTED_STATUSES,
            'Invalid formatted LOC status')
    require(type(complete) is bool, 'Formatted LOC completeness must be a boolean')
    require(type(evidence) is dict, 'Formatted LOC evidence must be an object')
    if status == 'qualified':
        require(type(loc) is int and loc > 0 and complete is True,
                'Qualified formatted LOC requires a complete positive integer count')
    else:
        require(loc is None and complete is False,
                'Unqualified formatted LOC must have a null count and incomplete status')
    row.setdefault('formatted_loc', None)
    row.setdefault('formatted_loc_status', 'pending')
    row.setdefault('formatted_loc_complete', False)
    row.setdefault('formatted_loc_evidence', {})
    return status == 'qualified'


def geometric_reference(language, groups, programs, formatted=False):
    prefix = 'formatted_' if formatted else ''
    x_axis = 'formatted_x' if formatted else 'x'
    return dict(lang=language['key'], label=language['label'], **{
        kind: {
            axis: math.exp(sum(math.log(programs[group[prefix + kind]][
                x_axis if axis == 'x' else 'y']) for group in groups) / len(groups))
            for axis in ('x', 'y')
        } for kind in ('fast', 'small')
    })


def finalize_endpoints(data):
    """Retain the union of canonical and qualified formatted extrema.

    Canonical endpoint ties and reference summaries follow the admitted pool.
    Saved canonical LOC/time baselines and every original x/y remain unchanged.
    Formatted selection changes
    only the additional formatted fields and which extra sources are retained.
    A missing formatted measurement remains null, including its normalized x.
    """
    staged = deepcopy(data)
    programs = catalog(staged)
    staged['programs'] = programs
    files = staged['files']
    languages = staged['languages']
    benchmarks = staged['benchmarks']
    language_keys = [language['key'] for language in languages]
    benchmark_keys = [benchmark['key'] for benchmark in benchmarks]
    require(language_keys and benchmark_keys, 'Nonempty language and benchmark inventory required')
    require(len(language_keys) == len(set(language_keys)), 'Duplicate language')
    require(len(benchmark_keys) == len(set(benchmark_keys)), 'Duplicate benchmark')
    require(not any(excluded_language(key) for key in language_keys) and
            not any(excluded_language(row['language']) for row in programs.values()),
            'V and Red are report-only and cannot enter the visualization')
    # The published chart may already be pruned. Recover every admitted source
    # form from the full inventory before either metric chooses extrema.
    for key, row in programs.items():
        require(row['language'] in language_keys and row['benchmark'] in benchmark_keys,
                'Full source inventory belongs to an unknown language/benchmark')
        benchmark = next(item for item in benchmarks if item['key'] == row['benchmark'])
        matching = [group for group in benchmark['groups'] if group['language'] == row['language']]
        require(len(matching) == 1, 'Full source inventory lacks its unique benchmark group')
        if key not in matching[0]['programs']: matching[0]['programs'].append(key)
    formatted_endpoints = set()
    for benchmark in benchmarks:
        seen_languages = set()
        formatted_pool = []
        require(type(benchmark.get('baseline_loc')) is int and benchmark['baseline_loc'] > 0 and
                positive_time(benchmark.get('baseline_seconds')), 'Saved canonical endpoint baselines required')
        for group in benchmark['groups']:
            language = group['language']
            require(language in language_keys and language not in seen_languages,
                    'Missing or duplicate benchmark language')
            seen_languages.add(language)
            keys = group['programs']
            require(keys and len(keys) == len(set(keys)), 'Empty or duplicate program pool')
            candidates = []
            formatted = []
            for key in keys:
                require(key in programs, 'Missing candidate program')
                row = programs[key]
                require(row['key'] == key and row['language'] == language and
                        row['benchmark'] == benchmark['key'], 'Candidate belongs to another group')
                require(type(row['loc']) is int and row['loc'] > 0, 'Positive admitted LOC required')
                require(row['seconds'] is None or positive_time(row['seconds']), 'Invalid admitted elapsed time')
                require(positive_time(row.get('x')) and
                        (row.get('y') is None if row['seconds'] is None else positive_time(row.get('y'))),
                        'Saved admitted canonical coordinates required')
                if row['seconds'] is not None:
                    samples = row.get('samples')
                    require(type(samples) is list and len(samples) >= 3 and
                            all(positive_time(value) for value in samples),
                            'Timed endpoint requires at least three complete actual samples')
                    require(math.isclose(statistics.median(samples), row['seconds'], rel_tol=0, abs_tol=1e-10),
                            'Admitted elapsed time differs from the actual sample median')
                    status = row.get('measurement_status')
                    require(status is None or type(status) is str and not status.lower().startswith(
                        ('partial', 'pending', 'running', 'failed', 'stopped', 'unqualified')),
                        'An unfinished or failed measurement cannot become a timed endpoint')
                require(row['files'] and all(identity in files for identity in row['files']), 'Missing admitted source')
                candidates.append(row)
                if formatted_candidate(row):
                    formatted.append(row)
            timed = [row for row in candidates if row['seconds'] is not None]
            group['fast'] = min(timed, key=lambda row: (row['seconds'], row['loc']))['key'] if timed else None
            group['small'] = min(candidates, key=lambda row: (row['loc'],
                row['seconds'] if row['seconds'] is not None else math.inf))['key']
            formatted_timed = [row for row in formatted if row['seconds'] is not None]
            group['formatted_fast'] = min(formatted_timed,
                key=lambda row: (row['seconds'], row['formatted_loc']))['key'] if formatted_timed else None
            group['formatted_small'] = min(formatted, key=lambda row: (row['formatted_loc'],
                row['seconds'] if row['seconds'] is not None else math.inf))['key'] if formatted else None
            formatted_pool.extend(formatted)
            formatted_endpoints.update(key for key in (group['formatted_fast'], group['formatted_small']) if key)
            group['programs'] = list(dict.fromkeys(key for key in (
                group['fast'], group['small'], group['formatted_fast'], group['formatted_small']) if key))
        # Formatted X has its own full eligible pool minimum. Canonical
        # historical baselines and coordinates belong to the saved dataset.
        benchmark['baseline_formatted_loc'] = min(
            row['formatted_loc'] for row in formatted_pool) if formatted_pool else None
        current = [row for row in programs.values() if row['benchmark'] == benchmark['key']]
        for row in current:
            row['formatted_x'] = (row['formatted_loc'] / benchmark['baseline_formatted_loc']
                                  if row['formatted_loc_status'] == 'qualified' else None)
    retained = {key for benchmark in benchmarks for group in benchmark['groups'] for key in group['programs']}
    overlay_fields = ('formatted_loc', 'formatted_loc_status', 'formatted_loc_complete',
                      'formatted_loc_evidence', 'formatted_x')
    if 'program_inventory' in staged:
        for key, row in staged['program_inventory'].items():
            row.update({field: deepcopy(programs[key][field]) for field in overlay_fields})
    staged['programs'] = {key: row for key, row in programs.items() if key in retained}
    retained_files = {identity for row in staged['programs'].values() for identity in row['files']}
    retained_files.update(identity for row in staged.get('program_inventory', {}).values() for identity in row['files'])
    staged['files'] = {key: row for key, row in files.items() if key in retained_files}
    if 'interpreter_integration_audit' in staged:
        staged['interpreter_integration_audit']['plotted_programs'] = sorted(
            key for key in staged['interpreter_integration_audit']['selected_programs'] if key in retained)
    reference = []
    formatted_reference = []
    for language in languages:
        groups = [next((group for group in benchmark['groups'] if group['language'] == language['key']), None)
                  for benchmark in benchmarks]
        language['timed_benchmarks'] = [benchmark['key'] for benchmark, group in zip(benchmarks, groups)
                                       if group and group['fast']]
        language['source_benchmarks'] = [benchmark['key'] for benchmark, group in zip(benchmarks, groups) if group]
        language['formatted_timed_benchmarks'] = [benchmark['key'] for benchmark, group in zip(benchmarks, groups)
                                                 if group and group['formatted_fast']]
        language['formatted_source_benchmarks'] = [benchmark['key'] for benchmark, group in zip(benchmarks, groups)
                                                  if group and group['formatted_small']]
        if all(group and group['fast'] and programs[group['small']]['seconds'] is not None for group in groups):
            reference.append(geometric_reference(language, groups, programs))
        if all(group and group['formatted_fast'] and
               programs[group['formatted_small']]['seconds'] is not None for group in groups):
            formatted_reference.append(geometric_reference(language, groups, programs, formatted=True))
    staged['reference_summary'] = reference
    staged['formatted_reference_summary'] = formatted_reference
    require(not any(excluded_language(group['language']) for benchmark in benchmarks for group in benchmark['groups']),
            'V and Red visualization groups are forbidden')
    # No callback, file operation or other fallible prerequisite after commit.
    for key in ('programs', 'files'):
        data[key].clear()
        data[key].update(staged[key])
    if 'program_inventory' in staged:
        data['program_inventory'].clear()
        data['program_inventory'].update(staged['program_inventory'])
    for key in ('languages', 'benchmarks'):
        data[key][:] = staged[key]
    data['reference_summary'] = staged['reference_summary']
    data['formatted_reference_summary'] = staged['formatted_reference_summary']
    if 'interpreter_integration_audit' in staged:
        data['interpreter_integration_audit'] = staged['interpreter_integration_audit']
    return dict(retained_programs=len(retained), retained_sources=len(retained_files),
                complete_timed_suites=len(reference), complete_formatted_timed_suites=len(formatted_reference),
                formatted_endpoint_programs=len(formatted_endpoints), V_visualization_entries=0,
                Red_visualization_entries=0)
