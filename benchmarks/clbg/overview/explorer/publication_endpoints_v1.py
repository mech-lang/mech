"""Finalize an admitted payload after additive measurement/LOC imports.

This module is unhooked until its locked fixtures and integration are reviewed.
It does no file reads, hashing or qualification. Its caller supplies only the
already admitted measurement pool, while retaining all raw reports separately.
"""
from copy import deepcopy
import math
import statistics


def require(condition, message):
    if not condition:
        raise ValueError(message)


def positive_time(value):
    return type(value) in (int, float) and math.isfinite(value) and value > 0


def finalize_endpoints(data):
    """Stage endpoint selection, ratios and complete-suite reference summaries.

    New imports add both forms; the chart requires fast/small identities. Never
    use a partial sample count as a timing, or infer a runtime from a language
    name. Backend, source-work and LOC admission happen before this function.
    """
    staged = deepcopy(data)
    programs = staged['programs']
    files = staged['files']
    languages = staged['languages']
    benchmarks = staged['benchmarks']
    language_keys = [language['key'] for language in languages]
    benchmark_keys = [benchmark['key'] for benchmark in benchmarks]
    require(language_keys and benchmark_keys, 'Nonempty language and benchmark inventory required')
    require(len(language_keys) == len(set(language_keys)), 'Duplicate language')
    require(len(benchmark_keys) == len(set(benchmark_keys)), 'Duplicate benchmark')
    require('v' not in language_keys and all(row['language'] != 'v' for row in programs.values()),
            'V is report-only and cannot enter the visualization')
    for benchmark in benchmarks:
        seen_languages = set()
        for group in benchmark['groups']:
            language = group['language']
            require(language in language_keys and language not in seen_languages,
                    'Missing or duplicate benchmark language')
            seen_languages.add(language)
            keys = group['programs']
            require(keys and len(keys) == len(set(keys)), 'Empty or duplicate program pool')
            candidates = []
            for key in keys:
                require(key in programs, 'Missing candidate program')
                row = programs[key]
                require(row['key'] == key and row['language'] == language and
                        row['benchmark'] == benchmark['key'], 'Candidate belongs to another group')
                require(type(row['loc']) is int and row['loc'] > 0, 'Positive admitted LOC required')
                require(row['seconds'] is None or positive_time(row['seconds']), 'Invalid admitted elapsed time')
                if row['seconds'] is not None:
                    samples = row.get('samples')
                    require(type(samples) is list and len(samples) >= 3 and all(positive_time(value) for value in samples),
                            'Timed endpoint requires at least three complete actual samples')
                    require(math.isclose(statistics.median(samples), row['seconds'], rel_tol=0, abs_tol=1e-10),
                            'Admitted elapsed time differs from the actual sample median')
                    status = row.get('measurement_status')
                    require(status is None or type(status) is str and not status.lower().startswith(
                        ('partial', 'pending', 'running', 'failed', 'stopped', 'unqualified')),
                        'An unfinished or failed measurement cannot become a timed endpoint')
                require(row['files'] and all(identity in files for identity in row['files']), 'Missing admitted source')
                candidates.append(row)
            timed = [row for row in candidates if row['seconds'] is not None]
            # Preserve the existing producer's stable tie-breaking policy.
            group['fast'] = min(timed, key=lambda row: (row['seconds'], row['loc']))['key'] if timed else None
            group['small'] = min(candidates, key=lambda row: (row['loc'],
                row['seconds'] if row['seconds'] is not None else math.inf))['key']
            group['programs'] = list(dict.fromkeys(key for key in (group['fast'], group['small']) if key))
        current = [programs[key] for group in benchmark['groups'] for key in group['programs']]
        timed = [row for row in current if row['seconds'] is not None]
        require(current and timed, 'Benchmark has no measured endpoint baseline')
        benchmark['baseline_loc'] = min(row['loc'] for row in current)
        benchmark['baseline_seconds'] = min(row['seconds'] for row in timed)
        for row in current:
            row['x'] = row['loc'] / benchmark['baseline_loc']
            row['y'] = row['seconds'] / benchmark['baseline_seconds'] if row['seconds'] is not None else None
    retained = {key for benchmark in benchmarks for group in benchmark['groups'] for key in group['programs']}
    staged['programs'] = {key: row for key, row in programs.items() if key in retained}
    retained_files = {identity for row in staged['programs'].values() for identity in row['files']}
    staged['files'] = {key: row for key, row in files.items() if key in retained_files}
    if 'interpreter_integration_audit' in staged:
        staged['interpreter_integration_audit']['plotted_programs'] = sorted(
            key for key in staged['interpreter_integration_audit']['selected_programs'] if key in retained)
    reference = []
    for language in languages:
        groups = [next((group for group in benchmark['groups'] if group['language'] == language['key']), None)
                  for benchmark in benchmarks]
        language['timed_benchmarks'] = [benchmark['key'] for benchmark, group in zip(benchmarks, groups)
                                       if group and group['fast']]
        language['source_benchmarks'] = [benchmark['key'] for benchmark, group in zip(benchmarks, groups) if group]
        if all(group and group['fast'] and programs[group['small']]['seconds'] is not None for group in groups):
            reference.append(dict(lang=language['key'], label=language['label'], **{
                kind: {axis: math.exp(sum(math.log(programs[group[kind]][axis]) for group in groups) / len(groups))
                       for axis in ('x', 'y')} for kind in ('fast', 'small')}))
    staged['reference_summary'] = reference
    require(not any(group['language'] == 'v' for benchmark in benchmarks for group in benchmark['groups']),
            'V visualization group is forbidden')
    # No callback, file operation or other fallible prerequisite after commit.
    for key in ('programs', 'files'):
        data[key].clear()
        data[key].update(staged[key])
    for key in ('languages', 'benchmarks'):
        data[key][:] = staged[key]
    data['reference_summary'] = staged['reference_summary']
    if 'interpreter_integration_audit' in staged:
        data['interpreter_integration_audit'] = staged['interpreter_integration_audit']
    return dict(retained_programs=len(retained), retained_sources=len(retained_files),
                complete_timed_suites=len(reference), V_visualization_entries=0)
