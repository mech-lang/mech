"""Attribute the parser portion of the existing full frontend stack captures."""

from collections import Counter
from pathlib import Path

from stage import HERE, ROADMAP, read, sha, write


def main():
    traces = HERE / 'source-stack-samples-profiled'
    summary = read(traces / 'summary.json')
    earlier = Path((ROADMAP / 'post-statement-profile-path').read_text().strip())
    classifier = earlier / 'sample_hotspots.py'
    scope = {'__name__': 'current_parser_stack_analysis', '__file__': str(classifier)}
    exec(compile(classifier.read_text(), str(classifier), 'exec'), scope)
    analyze = scope['parse_callgraph']
    counts = Counter()
    runs = []
    for index, observation in enumerate(summary['observations'], 1):
        raw = traces / f'sample-{index}' / 'call-graph.txt'
        assert sha(raw) == observation['raw_sha256']
        total, family, _, _ = analyze(raw.read_text().splitlines())
        assert sum(family.values()) == total
        counts.update(family)
        runs.append({'parser_samples': total, 'by_family': dict(family),
                     'raw_sha256': observation['raw_sha256']})
    record = {'compiler_sha256': summary['compiler_sha256'], 'analysis_driver_sha256': sha(classifier),
              'profiles': runs, 'aggregate_parser_samples': sum(row['parser_samples'] for row in runs),
              'aggregate_families': dict(counts.most_common())}
    write(HERE / 'parser-profile-context.json', record)
    print(record['aggregate_parser_samples'], record['aggregate_families'])


if __name__ == '__main__':
    main()
