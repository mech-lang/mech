#!/usr/bin/env python3
"""Summarize macOS sample call trees, excluding non-execution benchmark work.

Pass a Rust symbol filter such as rustfilt with --demangler. Action categories
are disjoint outer-call attribution; self mechanisms form a separate view.
"""
import argparse
import collections
import gzip
import hashlib
import json
import pathlib
import re
import subprocess

def parse_nodes(text):
    graph = text.split('Call graph:\n', 1)[1].split('\nTotal number in stack', 1)[0].split('\nSort by top of stack', 1)[0]
    records = []
    stack = []
    for line in graph.splitlines():
        match = re.match('^([ +!|:\\t]*)(\\d+) (.*)$', line)
        if not match:
            continue
        prefix, count, rest = match.groups()
        name = re.split('\\s+\\(in ', rest, maxsplit=1)[0].strip()
        depth = len(prefix)
        while stack and records[stack[-1]]['depth'] >= depth:
            stack.pop()
        at = len(records)
        parent = stack[-1] if stack else None
        records.append({'depth': depth, 'count': int(count), 'name': name, 'parent': parent, 'children': []})
        if parent is not None:
            records[parent]['children'].append(at)
        stack.append(at)
    for n in records:
        n['self'] = n['count'] - sum((records[i]['count'] for i in n['children']))
        if n['self'] < 0:
            raise ValueError(('child count exceeds parent', n))
    return records

def action(stack):
    if any(('runtime::invocation::Invocation' in n and n.endswith('::visit') or '::visit_from_entry' in n or 'runtime::hooks::admit' in n for n in stack)):
        return 'Source visits and admission'
    if any((n.endswith(('::scalar_matrix_element_call', '::unsigned_matrix_element_call', '::scalar_matrix_element', '::unsigned_matrix_element', '::matrix_element_word')) for n in stack)):
        return 'Typed matrix scalar reads'
    if any((n.endswith('::collection_helper') or ('runtime::collections::State' in n and n.endswith('::helper')) or 'runtime::collections::row_blocks::' in n for n in stack)):
        return 'Comprehension collection bookkeeping'
    if any((n.endswith(('::canonical_library_call', '::invoke_current_cached')) for n in stack)):
        return 'Canonical library calls'
    if any(('runtime::buffer_frames::' in n or n.endswith(('::buffer_frame_action', '::buffer_frame_call', '::buffer_frame_helper')) for n in stack)):
        return 'Private frame transport'
    if any((n.endswith('::retire_tail_locals') for n in stack)):
        return 'Loop-local retirement'
    if any((n.endswith(('::unsigned_buffer_call', '::unsigned_matrix_replace_call', '::scalar_matrix_replace_call', '::scalar_matrix_replace'))
            or '::matrix_replace_call::<' in n or '::matrix_replace_with::<' in n for n in stack)):
        return 'Selected matrix updates'
    if any((n.endswith('::prepared_library_call') for n in stack)):
        return 'Other selected library dispatch'
    if any(('runtime::hooks::helper' in n for n in stack)):
        return 'Other helper dispatch'
    return 'Other execution and generated code'

def mechanism(leaf):
    low = leaf.lower()
    if any((t in low for t in ['malloc', 'realloc', '_xzm_free', '_free', '::alloc::', '__rdl_alloc', '__rust_alloc', '__rust_dealloc', 'raw_vec'])):
        return 'Allocation and deallocation'
    if any((t in low for t in ['memcpy', 'memmove', 'memset', 'bzero'])):
        return 'Byte copy and zeroing'
    if any((t in leaf for t in ['resident::budget::', 'memory_runtime::', 'NativeProjectionIndexContext', 'pthread_mutex', 'os_unfair_lock'])):
        return 'Resource bookkeeping and locks'
    if any((t in leaf for t in ['memory_plan::', 'memory_planner::', 'memory_contract::', 'schema::', 'dimension::', 'snapshot::validation::'])):
        return 'Memory planning, schema and snapshot validation'
    if leaf.startswith('???'):
        return 'Generated code, unresolved symbol'
    return 'Other runtime self time'

def analyze(records):
    roots = []
    for i, n in enumerate(records):
        if 'mech_gpu::native_artifact::program::runtime::execute' in n['name']:
            parent = n['parent']
            duplicate = False
            while parent is not None:
                if parent in roots:
                    duplicate = True
                    break
                parent = records[parent]['parent']
            if not duplicate:
                roots.append(i)
    total = sum((records[i]['count'] for i in roots))
    if not total:
        raise ValueError('no canonical execution samples')
    actions = collections.Counter()
    mechanisms = collections.Counter()
    inclusive = collections.Counter()
    own = collections.Counter()
    paths = collections.Counter()

    def visit(i, stack):
        n = records[i]
        stack = stack + [n['name']]
        if n['self']:
            amount = n['self']
            actions[action(stack)] += amount
            mechanisms[mechanism(n['name'])] += amount
            own[n['name']] += amount
            for symbol in dict.fromkeys(stack):
                inclusive[symbol] += amount
            paths[tuple(stack[-8:])] += amount
        for child in n['children']:
            visit(child, stack)
    for i in roots:
        visit(i, [])
    assert sum(actions.values()) == sum(mechanisms.values()) == total

    def rows(counts, limit=None):
        return [{'name': name, 'samples': n, 'percent': 100 * n / total} for name, n in sorted(counts.items(), key=lambda item: (-item[1], item[0]))[:limit]]
    all_samples = sum((n['count'] for n in records if n['parent'] is None))
    return {'all_thread_samples': all_samples, 'execution_samples': total, 'excluded_samples': all_samples - total, 'action_categories': rows(actions), 'self_time_mechanisms': rows(mechanisms), 'top_self_functions': rows(own, 25), 'top_inclusive_functions': rows(inclusive, 30), 'top_leaf_stacks': [{'samples': n, 'percent': 100 * n / total, 'stack': list(stack)} for stack, n in sorted(paths.items(), key=lambda item: (-item[1], item[0]))[:20]]}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--demangler', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('samples', type=pathlib.Path, nargs='+')
    args = parser.parse_args()
    output = {}
    for path in args.samples:
        payload = path.read_bytes()
        text = (gzip.decompress(payload) if path.suffix == '.gz' else payload).decode()
        records = parse_nodes(text)
        symbols = list(dict.fromkeys((n['name'] for n in records)))
        result = subprocess.run([str(args.demangler)], input='\n'.join(symbols) + '\n', text=True, capture_output=True, check=True)
        decoded = dict(zip(symbols, result.stdout.splitlines(), strict=True))
        for node in records:
            node['name'] = decoded[node['name']]
        row = analyze(records)
        row['sample_sha256'] = hashlib.sha256(text.encode()).hexdigest()
        output[path.name.removesuffix('.gz')] = row
        print(path.name, row['execution_samples'], 'execution samples')
        for category in row['action_categories']:
            print(f"  {category['percent']:6.2f}% {category['name']}")
    args.output.write_text(json.dumps(output, indent=2) + '\n')
if __name__ == '__main__':
    main()
