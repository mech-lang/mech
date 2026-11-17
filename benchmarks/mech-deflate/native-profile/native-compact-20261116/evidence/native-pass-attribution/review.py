"""Rank the current cleanup calls and read older verifier-internal timings."""
from collections import defaultdict
from hashlib import file_digest
from pathlib import Path
from statistics import median
import argparse
import json


CALLS = {'analysis.prune-phi', 'analysis.prune', 'analysis.phi',
         'analysis.cfg', 'analysis.dominators', 'analysis.flowgraph', 'analysis.verify'}


def read(path):
    return json.loads(path.read_bytes())


def digest(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def sequence(events):
    calls = []
    for event in events:
        if event['pass'] not in CALLS:
            continue
        ancestors = []
        parent = event['parent']
        while parent is not None:
            node = events[parent]
            if node['pass'] == 'root.cleanup':
                calls.append((event['pass'], ancestors, event['end_ns'] - event['start_ns']))
                break
            ancestors.append(node['pass'])
            parent = node['parent']
    return calls


def verify_internals(events):
    result = defaultdict(lambda: {'calls': 0, 'ms': 0.0})
    for event in events:
        ancestors = set()
        parent = event['parent']
        while parent is not None:
            node = events[parent]
            ancestors.add((node['category'], node['detail']))
            parent = node['parent']
        label = event['detail']
        if ('cfg_cleanup', 'checked root') in ancestors and label in (
            'Verify Cranelift IR', 'Remove constant phi-nodes', 'Remove unreachable blocks',
            'Control flow graph', 'Dominator tree',
        ):
            if label in ('Control flow graph', 'Dominator tree'):
                if ('cranelift_pass', 'Verify Cranelift IR') not in ancestors:
                    continue
                label += ' inside verification'
            row = result[label]
            row['calls'] += 1
            row['ms'] += (event['end_ns'] - event['start_ns']) / 1e6
    return dict(result)


def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument('observed', type=Path)
    args.add_argument('archived_trace', type=Path)
    args.add_argument('output', type=Path)
    cli = args.parse_args()
    current = read(cli.observed / 'summary.json')
    plan = cli.observed / 'plan.json'
    assert current['plan_sha256'] == digest(plan)
    arrays = []
    for i, pair in enumerate(current['trials'], 1):
        file = cli.observed / f'order-{i}/trace/native-pass-trace.json'
        assert digest(file) == pair['trace']['trace_sha256']
        arrays.append(sequence(read(file)))
    assert all([x[:2] for x in row] == [x[:2] for x in arrays[0]] for row in arrays)
    entries = [{'index': i + 1, 'pass': name, 'callers_from_inner_to_outer': callers,
                'per_trial_ms': [trial[i][2] / 1e6 for trial in arrays],
                'median_ms': median(trial[i][2] for trial in arrays) / 1e6}
               for i, (name, callers, _) in enumerate(arrays[0])]
    inputs = [row['trace']['root_ir'] for row in current['trials']]
    assert all(row == inputs[0] for row in inputs)
    before = inputs[0]
    archived = [x for x in read(cli.archived_trace) if x['thread'] == 'coordinator']
    assert len(archived) == 1
    record = {'current_source': read(plan)['source_pin'],
              'trace_plan_sha256': digest(plan),
              'root_calls': entries,
              'first_cleanup_report': before,
              'archived_verifier_traces': {'sha256': digest(cli.archived_trace), 'by_label': verify_internals(archived[0]['events'])},
              'limits': ['The current trace measures coordinator call boundaries and aggregate IR changes.',
                         'Unreachable and constant-phi timings include internal verifier calls.',
                         'Individual visited and changed work counts require a narrower trace.',
                         'Verifier-internal times come from an earlier capture and give a diagnostic ceiling.']}
    cli.output.write_text(json.dumps(record, indent=2) + '\n')
    print({name: value for name, value in record.items() if name not in ('root_calls', 'first_cleanup_report')}, flush=True)


if __name__ == '__main__':
    main()
