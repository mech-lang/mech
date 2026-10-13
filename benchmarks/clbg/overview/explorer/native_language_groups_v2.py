"""Candidate filter memberships after all eight measured backend overlays.

This additive step operates on an already admitted in-memory payload. Runtime
groups require the actual per-program backend metadata. Language typing/style
are separate source-reviewed facts, not guesses about a measured platform.
The existing importer and its qualified fixtures remain unchanged.
"""
from native_languages_data_v2 import LANGUAGES, require
from native_platform_metadata_v2 import BACKEND_OPTIONS

GROUPS = {'aot', 'jit', 'interpreted', 'array', 'functional', 'dynamic'}
ADDITIONS = {
    'aot': {'nim', 'crystal', 'pony', 'c3'},
    'jit': {'gleam', 'clojure'},
    'interpreted': {'prolog', 'rebol'},
    'array': set(),
    'functional': {'gleam', 'clojure'},
    'dynamic': {'clojure', 'prolog', 'rebol'},
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
    'prolog': {
        'typing': 'dynamic', 'style': 'logic',
        'source': 'https://www.swi-prolog.org/pldoc/man?section=cpp2-intro',
        'style_source': 'https://www.swi-prolog.org/pldoc/man?section=ssu',
        'basis': 'SWI-Prolog describes integration with dynamically typed Prolog, and logic programming with procedural constructs.',
    },
    'rebol': {
        'typing': 'dynamic',
        'source': 'https://www.rebol.com/docs/core23/rebolcore-4.html',
        'implementation_source': 'https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/c-do.c',
        'basis': 'REBOL words can be set to any type of value; the selected Rebol3 evaluator assigns the evaluated value at runtime. No Array or Functional style claim is made.',
    },
}


def add_native_language_groups(data):
    """Stage all group/prose checks before any mutation.

    Called after the eight-language importer and actual platform overlay, before
    endpoint pruning. Preserves every timing, source, LOC, baseline and existing
    language membership. It does not qualify or publish a measurement.
    """
    audit = data['native_language_audit']['languages']
    require(set(audit) == LANGUAGES, 'All eight complete cohorts must be admitted first')
    languages = {row['key']: row for row in data['languages']}
    require(len(languages) == len(data['languages']) and LANGUAGES <= set(languages)
            and not {'v','red'} & set(languages), 'Unique complete language inventory required; V and Red excluded')
    require(not any(row['language'] in {'v','red'} for row in data['programs'].values())
            and not any(group['language'] in {'v','red'} for benchmark in data['benchmarks'] for group in benchmark['groups']),
            'Excluded languages cannot enter visualization filter groups')
    actual_backends = {}
    for language in sorted(LANGUAGES):
        cohort = audit[language]
        require(cohort['source_count'] == 20 and cohort['actual_full_native_processes'] == 90
                and cohort['sample_count'] == 3, 'Complete measured cohort required for filter membership')
        programs = [row for row in data['programs'].values() if row['language'] == language]
        require(len(programs) == 20, 'Filter grouping must precede endpoint pruning')
        selections = {(row.get('measured_backend', {}).get('key'), row.get('measured_backend', {}).get('execution'))
            for row in programs}
        require(len(selections) == 1 and selections <= BACKEND_OPTIONS[language],
                'Runtime groups require one actual qualified backend for the whole cohort')
        backend, execution = next(iter(selections)); actual_backends[language] = execution
        require(languages[language]['execution'].lower() == execution,
                'Language execution differs from the actual qualified backend')
        for row in programs:
            record = row.get('measured_backend', {})
            require(record.get('verified') is True and record.get('key') == backend
                    and record.get('execution') == execution and row['execution_mode'] == execution,
                    'Runtime membership needs actual qualified per-program evidence')
    groups = data['language_groups']
    require(len(groups) == len(GROUPS) and {group['key'] for group in groups} == GROUPS,
            'Original six trait-group identities required')
    additions = {key: set(value) for key,value in ADDITIONS.items()}
    additions['aot'].update(language for language,execution in actual_backends.items() if execution == 'aot')
    additions['jit'].update(language for language,execution in actual_backends.items() if execution == 'jit')
    additions['interpreted'].update(language for language,execution in actual_backends.items() if execution == 'interpreted')
    staged = []
    for group in groups:
        members = group['languages']
        require(type(members) is list and len(members) == len(set(members))
                and all(type(key) is str and key in languages for key in members),
                'Trait groups contain missing or duplicate language identities')
        staged.append([key for key in members if key not in LANGUAGES]
                      + sorted(additions[group['key']]))
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
                dynamic=['clojure', 'prolog', 'rebol'], interpreted=sorted(additions['interpreted']), measured_runtime_groups_verified=True,
                source_or_timing_changes=0, V_visualization_entries=0)
