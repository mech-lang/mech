"""Recover inclusive source-lowering costs from the pinned call graphs."""

from collections import Counter

from sample import FRAME, HERE, attributed, sha
from stage import read, write

PROFILES = HERE / 'source-stack-samples-profiled'
FAMILIES = {
    'owned_syntax_children': ('retained_sequence', 'into_values'),
    'eager_nodes_and_tokens': ('children_with_tokens',),
    'schema_canonicalization': ('canonicalize_dimension_environment',),
    'schema_embedding': ('embed_schema_draft',),
    'function_argument_schema': ('function_argument_shape', 'embed'),
}
CONTEXT = ('finalize_schema', 'finalize_ref', 'from_schema_body', 'from_kind', 'with_dimension_origin_policy',
           'specialize_annotation_dimensions', 'rewrite_kind_dimensions', 'resolve_maintained_call',
           'conform_schema_draft', 'function_argument_shape')


def analyze(path):
    counts = Counter()
    lowering = Counter()
    canonical_context = Counter()
    stack = []
    active = False

    def close():
        _, visits, _, child_visits = stack[-1]
        exclusive = visits - child_visits
        assert exclusive >= 0
        names = tuple(row[2] for row in stack)
        if exclusive and any(attributed(name) for name in names):
            counts['semantic'] += exclusive
            in_lowering = any('compile_document_units' in name for name in names)
            lowering['semantic'] += exclusive if in_lowering else 0
            for label, markers in FAMILIES.items():
                if any(all(key in name for key in markers) for name in names):
                    counts[label] += exclusive
                    if in_lowering:
                        lowering[label] += exclusive
                    if label == 'schema_canonicalization':
                        covered = [label for label in CONTEXT if any(label in fn for fn in names)]
                        canonical_context.update({label: exclusive for label in covered})
        stack.pop()

    for line in path.read_text(errors='replace').splitlines():
        if line == 'Call graph:':
            active = True
        if line.startswith(('Total number in stack', 'Sort by top of stack', 'Binary Images:')):
            break
        match = FRAME.match(line) if active else None
        if not match:
            continue
        depth, visits, name = len(match[1]), int(match[2]), match[3]
        while stack and stack[-1][0] >= depth:
            close()
        if stack:
            stack[-1][3] += visits
        stack.append([depth, visits, name, 0])
    while stack:
        close()
    return counts, lowering, canonical_context


def main():
    summary = read(PROFILES / 'summary.json')
    observed, lowering, domains = Counter(), Counter(), Counter()
    for step, row in enumerate(summary['observations'], 1):
        path = PROFILES / f'sample-{step}' / 'call-graph.txt'
        assert sha(path) == row['raw_sha256']
        counts, visit, context = analyze(path)
        assert counts['semantic'] == row['semantic_samples']
        assert visit['semantic'] == row['document_lowering_samples']
        observed.update(counts)
        lowering.update(visit)
        domains.update(context)
    result = {'compiler_sha256': summary['compiler_sha256'], 'profile_runs': len(summary['observations']),
              'frontend_samples': dict(observed), 'document_lowering_samples': dict(lowering),
              'canonicalization_context_samples': dict(domains)}
    write(HERE / 'stack-context.json', result)
    print(result)


if __name__ == '__main__':
    main()
