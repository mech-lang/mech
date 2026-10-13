"""Complete filter memberships after all six measured backend overlays.

This additive step operates on an already admitted in-memory payload. Runtime
groups require the actual per-program backend metadata. Language typing/style
are separate source-reviewed facts, not guesses about a measured platform.
The existing importer and its qualified fixtures remain unchanged.
"""
from native_languages_data_v1 import LANGUAGES, require
from native_platform_metadata_v1 import BACKENDS

GROUPS = {'aot', 'jit', 'interpreted', 'array', 'functional', 'dynamic'}
ADDITIONS = {
    'aot': {'nim', 'crystal', 'pony', 'c3'},
    'jit': {'gleam', 'clojure'},
    'interpreted': set(),
    'array': set(),
    'functional': {'gleam', 'clojure'},
    'dynamic': {'clojure'},
}
LANGUAGE_FACTS = {
    'gleam': {
        'typing': 'static', 'style': 'functional',
        'source': 'https://gleam.run/frequently-asked-questions/',
        'basis': 'The official comparison with Rust identifies a static type system and a functional language.',
    },
    'clojure': {
        'typing': 'dynamic', 'style': 'functional',
        'source': 'https://clojure.org/',
        'basis': 'The official language description identifies dynamic typing and a predominantly functional language.',
    },
}


def add_native_language_groups(data):
    """Stage all group/prose checks before any mutation.

    Called after the six-language importer and actual platform overlay, before
    endpoint pruning. Preserves every timing, source, LOC, baseline and existing
    language membership. It does not qualify or publish a measurement.
    """
    audit = data['native_language_audit']['languages']
    require(set(audit) == LANGUAGES, 'All six complete cohorts must be admitted first')
    languages = {row['key']: row for row in data['languages']}
    require(len(languages) == len(data['languages']) and LANGUAGES <= set(languages)
            and 'v' not in languages, 'Unique complete language inventory required; V stays report-only')
    require(not any(row['language'] == 'v' for row in data['programs'].values())
            and not any(group['language'] == 'v' for benchmark in data['benchmarks'] for group in benchmark['groups']),
            'V cannot enter visualization filter groups')
    for language, (backend, execution) in BACKENDS.items():
        require(languages[language]['execution'].lower() == execution,
                'Language execution differs from the actual qualified backend')
        cohort = audit[language]
        require(cohort['source_count'] == 20 and cohort['actual_full_native_processes'] == 90
                and cohort['sample_count'] == 3, 'Complete measured cohort required for filter membership')
        programs = [row for row in data['programs'].values() if row['language'] == language]
        require(len(programs) == 20, 'Filter grouping must precede endpoint pruning')
        for row in programs:
            record = row.get('measured_backend', {})
            require(record.get('verified') is True and record.get('key') == backend
                    and record.get('execution') == execution and row['execution_mode'] == execution,
                    'Runtime membership needs actual qualified per-program evidence')
    groups = data['language_groups']
    require(len(groups) == len(GROUPS) and {group['key'] for group in groups} == GROUPS,
            'Original six trait-group identities required')
    staged = []
    for group in groups:
        members = group['languages']
        require(type(members) is list and len(members) == len(set(members))
                and all(type(key) is str and key in languages for key in members),
                'Trait groups contain missing or duplicate language identities')
        staged.append([key for key in members if key not in LANGUAGES]
                      + sorted(ADDITIONS[group['key']]))
    text = data['protocol']['language_groups']
    require(type(text) is str, 'Existing trait filter protocol required')
    old = 'Language filters intersect Execution, Typing, Language style and Memory management categories.'
    new = 'Language filters intersect Platform, Execution, Typing, Language style and Memory management categories.'
    require(text.count(old) <= 1, 'Ambiguous duplicated filter protocol')
    if old in text:
        text = text.replace(old, new)
    elif new not in text:
        text = new + ' ' + text
    evidence = {key: dict(value) for key, value in LANGUAGE_FACTS.items()}
    # All prerequisites are complete. No callbacks, reads or hashes follow.
    for group, members in zip(groups, staged):
        group['languages'][:] = members
    data['protocol']['language_groups'] = text
    data['native_language_filter_facts'] = evidence
    return dict(languages=sorted(LANGUAGES), functional=['clojure', 'gleam'],
                dynamic=['clojure'], measured_runtime_groups_verified=True,
                source_or_timing_changes=0, V_visualization_entries=0)
