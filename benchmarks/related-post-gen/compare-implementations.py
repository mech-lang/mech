#!/usr/bin/env python3
"""Compare Mech with the fastest verified external implementation in its cohort."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import re
import shutil
import statistics
import subprocess
import time


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def cohort_orders(variants, samples, mode='alternating', seed=None):
    """Predeclare every round; only a local seeded generator changes order."""
    if mode not in ('alternating', 'randomized'):
        raise ValueError('Unknown cohort ordering')
    if (mode == 'randomized') != (seed is not None):
        raise ValueError('A seed is required exactly when ordering is randomized')
    generator = random.Random(seed) if mode == 'randomized' else None
    orders = []
    for sample in range(samples):
        order = list(variants)
        if generator is not None:
            generator.shuffle(order)
        elif sample % 2:
            order.reverse()
        orders.append(order)
    return orders


def retain_output_copy(source, destination, objects):
    """Share immutable evidence copies; a producer never writes into the store."""
    digest = sha(source)
    blob = objects / (digest + '.json')
    if blob.exists():
        assert sha(blob) == digest
    else:
        shutil.copy2(source, blob)
        assert sha(blob) == digest
        blob.chmod(0o444)
    destination.unlink(missing_ok=True)
    os.link(blob, destination)


def add_timing_comparisons(report):
    """Select references by declared timer scope, never by a variant's name."""
    variants = report['implementations']
    summary = report['summary']
    external = [v['name'] for v in variants if v['kind'] != 'mech']
    boundaries = {
        'unspecified', 'post_json_including_setup',
        'processing_excluding_admission', 'preinitialized_workers',
        'processing_excluding_input_preparation',
    }
    for variant in variants:
        name = variant['name']
        summary[name].pop('median_post_json_seconds', None)
        summary[name].pop('post_json_over_fastest_matched_native', None)
        boundary = variant.get('calculation_boundary', 'unspecified')
        if boundary not in boundaries:
            raise ValueError(f'Unknown calculation boundary for {name}: {boundary}')
        rows = [r for r in report['samples'] if r['variant'] == name]
        summary[name]['median_whole_process_seconds'] = statistics.median(
            r['wall_seconds'] for r in rows)
        summary[name]['calculation_boundary'] = boundary
        if boundary == 'post_json_including_setup':
            if not variant.get('processing_boundary'):
                raise ValueError(f'{name} lacks a supported, documented post-JSON timer')
            # Mech must supply one continuous measured interval. Adding its
            # admission and processing timers would miss allocation/setup in
            # between, even when their medians are combined per invocation.
            summary[name]['median_post_json_seconds'] = (
                statistics.median(r['post_json_seconds'] for r in rows)
                if variant['kind'] == 'mech' else summary[name]['median_seconds'])

    whole_name = min(external, key=lambda name: summary[name]['median_whole_process_seconds'])
    whole = summary[whole_name]['median_whole_process_seconds']
    report['whole_process_reference'] = {
        'variant': whole_name, 'median_seconds': whole,
        'scope': 'Fastest external fresh process, launch through exit, including all setup, JSON IO, computation and teardown.',
    }
    matched = [v['name'] for v in variants if v['kind'] != 'mech'
               and v.get('calculation_boundary') == 'post_json_including_setup']
    report['matched_native_reference'] = None
    if matched:
        name = min(matched, key=lambda name: summary[name]['median_post_json_seconds'])
        report['matched_native_reference'] = {
            'variant': name, 'median_seconds': summary[name]['median_post_json_seconds'],
            'scope': 'Fastest explicitly documented post-JSON timer including input preparation and worker creation/run/join. Artifact restoration and JSON IO are tracked in whole-process time.',
            'eligible_variants': matched,
        }

    # Retain the historical raw processing ratios for old consumers, but do
    # not present their potentially unequal preparation boundaries as primary.
    raw_name = min(external, key=lambda name: summary[name]['median_seconds'])
    raw = summary[raw_name]['median_seconds']
    rust = summary[report['reference']]['median_seconds']
    report['fastest_reference'] = {
        'variant': raw_name, 'median_seconds': raw,
        'scope': 'Historical raw processing timer convention; preparation and worker lifetimes may differ. This is not the primary comparison.',
    }
    for entry in summary.values():
        entry['over_fastest'] = entry['median_seconds'] / raw
        entry['over_rust'] = entry['median_seconds'] / rust
        entry['whole_process_over_fastest'] = entry['median_whole_process_seconds'] / whole
        if 'median_processing_plus_admission_seconds' in entry:
            entry['processing_plus_admission_over_fastest'] = entry['median_processing_plus_admission_seconds'] / raw
            entry['processing_plus_admission_over_rust'] = entry['median_processing_plus_admission_seconds'] / rust
        if matched and 'median_post_json_seconds' in entry:
            entry['post_json_over_fastest_matched_native'] = (
                entry['median_post_json_seconds'] / report['matched_native_reference']['median_seconds'])
    report['timing_policy_version'] = 3
    report['primary_metric'] = 'whole_process_over_fastest'
    report['primary_reference'] = report['whole_process_reference']
    report['secondary_metric'] = (
        'post_json_over_fastest_matched_native' if matched else None)
    report['primary_metric_definition'] = (
        'Lower is faster. The headline ratio always measures a complete fresh process, launch through exit, including all preparation, restoration, allocation, JSON IO and teardown. '
        'Secondary post-JSON ratios require explicit compatible boundary declarations and one continuous interval including Mech admission and intervening setup. '
        'Unknown or preinitialized processing timers never qualify as matched references. '
        'Neither an excluded startup cost nor a favorable partial interval establishes complete-task parity.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--samples', type=int, default=7)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--timeout', type=float, default=600)
    parser.add_argument('--resources', action='store_true', help='Record macOS whole-process resource counters separately from processing-window measurements.')
    parser.add_argument('--deduplicate-outputs', action='store_true', help='Retain immutable content-addressed output copies after each timed process exits.')
    parser.add_argument('--order', choices=['alternating', 'randomized'], default='alternating', help='Predeclare alternating or seeded randomized variant order.')
    parser.add_argument('--seed', type=int, help='Required with --order randomized; the full generated order is retained.')
    args = parser.parse_args()
    config = json.loads(args.config.read_text())
    if args.resources and platform.system() != 'Darwin':
        raise SystemExit('--resources currently requires macOS /usr/bin/time')
    variants = config['implementations']
    names = [v['name'] for v in variants]
    assert len(set(names)) == len(names) and config['reference'] in names
    try:
        orders = cohort_orders(variants, args.samples, args.order, args.seed)
    except ValueError as error:
        parser.error(str(error))
    input_path = args.input.resolve()
    args.work.mkdir(parents=True, exist_ok=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    objects = args.work / 'output-objects'
    if args.deduplicate_outputs:
        objects.mkdir(exist_ok=True)
    shutil.copy2(input_path, args.work / 'posts.json')
    report = {
        'schema': 'mech.related-post-gen.implementations-comparison.v1',
        'machine': config['machine'], 'platform': platform.platform(),
        'protocol': 'Fresh processes; reverse order on alternate rounds; every invocation computes all posts; full parsed JSON equality against unchanged standard Rust. All initial runs retained; no outlier removal.',
        'limitations': [
            'Shared development machine; exploratory timings.',
            'Processing timers exclude JSON IO. Mech input admission is recorded separately; upstream input decoding is outside their timers.',
            'Mech accounting and checked arithmetic remain enabled. Cleanup and worker-pool lifetime follow each adapter; consult the per-variant timing boundary.',
            'Single-core standard, specialized algorithms and concurrent implementations are labeled separately.',
            'Parent wall_seconds covers fresh subprocess launch through exit, including restoration, JSON IO, admission and cleanup; it includes common subprocess-launch overhead.',
        ],
        'timing_boundaries': config.get('timing_boundaries'),
        'reference': config['reference'],
        'input': {'path': str(input_path), 'sha256': sha(input_path), 'posts': len(json.loads(input_path.read_text()))},
        'implementations': [], 'samples': [], 'summary': {},
    }
    report['execution_order'] = {
        'mode': args.order, 'seed': args.seed,
        'python_version': platform.python_version(),
        'rounds': [[variant['name'] for variant in order] for order in orders],
    }
    if args.order == 'randomized':
        report['protocol'] = report['protocol'].replace(
            'reverse order on alternate rounds',
            'seeded randomized order, predeclared and retained for every round')
    if args.deduplicate_outputs:
        report['output_retention'] = 'After process timing ends, identical output files share read-only content-addressed copies. Producer paths never alias the retained store. Measured commands, outputs and timing boundaries are unchanged.'
    if args.resources:
        report['latency_qualification'] = config.get('latency_qualification', False)
        report['authoritative_latency_comparison'] = config.get('authoritative_latency_comparison')
        report['resource_reference'] = config.get('resource_reference')
        report['whole_process_resource_reference'] = config.get('whole_process_resource_reference') or config.get('resource_reference')
        report['resource_boundaries'] = {
            'process_resources': 'Whole child process, including startup, compiler restoration, admission, processing, JSON IO and cleanup. macOS /usr/bin/time -lp; memory values are bytes.',
            'measurement': 'Instrumented calculation window declared by each adapter measurement_boundary. Hardware counters aggregate process threads. Allocator counters wrap the original Rust allocator across all workers; requested heap excludes allocator internals, OS stacks and mmap.',
            'latency': 'Instrumentation changes host code and adds accounting overhead. Use the separately pinned authoritative latency cohort for performance claims.',
        }
    for variant in variants:
        arguments = variant.get('arguments', [])
        assert isinstance(arguments, list) and all(isinstance(argument, str) for argument in arguments)
        assert not arguments or variant['kind'] != 'mech', 'Mech adapters use the pinned source/input/output protocol'
        executable = Path(variant['executable']).resolve()
        variant['executable'] = str(executable)
        pin = {**variant, 'executable_sha256': sha(executable), 'executable_file_bytes': executable.stat().st_size}
        if 'source' in variant:
            pin['source_sha256'] = sha(variant['source'])
        if 'receipt' in variant:
            pin['receipt_sha256'] = sha(variant['receipt'])
        if variant['kind'] == 'mech':
            build = json.loads(Path(variant['receipt']).read_text())
            assert build['executable_sha256'] == pin['executable_sha256']
            assert build['bundle']['source.mec']['sha256'] == pin['source_sha256']
            for item in build['bundle'].values():
                assert sha(item['path']) == item['sha256']
        report['implementations'].append(pin)

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    for sample, order in enumerate(orders):
        outputs, rows = {}, []
        for variant in order:
            name = variant['name']
            executable = Path(variant['executable'])
            pin = next(p for p in report['implementations'] if p['name'] == name)
            assert sha(executable) == pin['executable_sha256']
            output = (args.work / f'{name}-{sample}.json').resolve()
            if args.deduplicate_outputs and output.exists():
                raise SystemExit(f'Refusing to overwrite retained output: {output}')
            if variant['kind'] == 'mech':
                command = [str(executable), str(Path(variant['source']).resolve()), str(input_path), str(output), '1']
                cwd = executable.parent
            else:
                command = [str(executable), *variant.get('arguments', [])]
                cwd = args.work / name
                cwd.mkdir(exist_ok=True)
                upstream_output = args.work / variant['output']
                upstream_output.unlink(missing_ok=True)
            start = time.monotonic()
            resource_path = args.work / f'{name}-{sample}.resources.txt'
            executed_command = ['/usr/bin/time', '-lp', '-o', str(resource_path.resolve()), *command] if args.resources else command
            result = subprocess.run(executed_command, cwd=cwd, capture_output=True, text=True, timeout=args.timeout)
            row = {'sample': sample, 'variant': name, 'command': command, 'exit': result.returncode, 'wall_seconds': time.monotonic() - start, 'stdout': result.stdout, 'stderr': result.stderr}
            if args.resources:
                row['resource_command'] = executed_command
                raw = resource_path.read_text()
                process = {}
                for line in raw.splitlines():
                    match = re.fullmatch(r'(real|user|sys)\s+([0-9.]+)', line.strip())
                    if match:
                        process[{'real':'wall_seconds','user':'user_cpu_seconds','sys':'system_cpu_seconds'}[match[1]]] = float(match[2])
                    match = re.fullmatch(r'\s*(\d+)\s+(.+?)\s*', line)
                    if match:
                        key = match[2].replace(' ', '_')
                        key = {'maximum_resident_set_size':'peak_resident_bytes', 'peak_memory_footprint':'peak_physical_footprint_bytes', 'instructions_retired':'instructions_retired', 'cycles_elapsed':'cycles'}.get(key, key)
                        process[key] = int(match[1])
                row['process_resources'] = process
                row['process_resources_raw'] = str(resource_path.resolve())
            report['samples'].append(row)
            if result.returncode:
                save()
                raise SystemExit(f'{name} failed; partial evidence saved')
            if variant['kind'] == 'mech':
                measurement = json.loads(next(line for line in result.stdout.splitlines() if line.startswith('{')))
                measurement['invocation_sample'] = measurement.pop('sample')
                row.update(measurement)
            else:
                match = re.search(r'Processing time .*:\s*([0-9.]+)\s*(ns|µs|us|ms|s)', result.stdout)
                if not match:
                    save()
                    raise SystemExit(f'Unrecognized timing from {name}: {result.stdout}')
                row['seconds'] = float(match[1]) * {'ns': 1e-9, 'µs': 1e-6, 'us': 1e-6, 'ms': 1e-3, 's': 1}[match[2]]
                measurements = [json.loads(line.removeprefix('PROCESS_METRICS ')) for line in result.stdout.splitlines() if line.startswith('PROCESS_METRICS ')]
                if measurements:
                    assert len(measurements) == 1
                    data = measurements[0]
                    row['measurement'] = data.get('processing', data)
                    if 'preparation' in data:
                        row['preparation_measurement'] = data['preparation']
                if args.deduplicate_outputs:
                    retain_output_copy(upstream_output, output, objects)
                else:
                    shutil.copy2(upstream_output, output)
            if args.deduplicate_outputs and variant['kind'] == 'mech':
                retain_output_copy(output, output, objects)
            row['output_sha256'] = sha(output)
            if 'measurement' in row:
                assert row['measurement']['allocator_lifetime_accounting_underflows'] == 0
                if args.resources:
                    assert row['measurement']['hardware']['instructions_retired'] > 0
            outputs[name] = json.loads(output.read_text())
            rows.append(row)
            save()
            print(json.dumps({k: row[k] for k in ('sample', 'variant', 'seconds', 'exit')}), flush=True)
        for row in rows:
            row['exact_rust_output'] = outputs[row['variant']] == outputs[config['reference']]
        save()
        if not all(r['exact_rust_output'] for r in rows):
            raise SystemExit('Output mismatch; full outputs retained')
    for variant in variants:
        rows = [r for r in report['samples'] if r['variant'] == variant['name']]
        report['summary'][variant['name']] = {
            'median_seconds': statistics.median(r['seconds'] for r in rows),
            'samples': len(rows), 'all_exact_rust': all(r['exact_rust_output'] for r in rows),
            'category': variant['category'],
        }
        if variant['kind'] == 'mech' and all('processing_plus_admission_seconds' in r for r in rows):
            report['summary'][variant['name']]['median_processing_plus_admission_seconds'] = statistics.median(r['processing_plus_admission_seconds'] for r in rows)
        if args.resources:
            def medians(items):
                keys = set.intersection(*(set(item) for item in items))
                result = {}
                for key in sorted(keys):
                    values = [item[key] for item in items]
                    if all(isinstance(value, dict) for value in values):
                        result[key] = medians(values)
                    elif all(type(value) in (int, float) for value in values):
                        result[key] = statistics.median(values)
                return result
            report['summary'][variant['name']]['process_resources_median'] = medians([r['process_resources'] for r in rows])
            if all('measurement' in r for r in rows):
                window = ('post_json' if variant.get('measurement_boundary') == 'post_json_including_setup'
                          else 'processing')
                report['summary'][variant['name']]['measurement_boundary'] = variant.get('measurement_boundary', 'processing')
                report['summary'][variant['name']][window+'_measurement_median'] = medians([r['measurement'] for r in rows])
            stages = {}
            for stage in ['restore', 'json', 'admission', 'preparation']:
                key = stage+'_measurement'
                if all(key in r for r in rows):
                    stages[stage] = medians([r[key] for r in rows])
            if stages:
                report['summary'][variant['name']]['stage_measurements_median'] = stages
    add_timing_comparisons(report)
    if args.resources and config.get('resource_reference'):
        baseline = report['summary'][config['resource_reference']]
        whole_baseline = report['summary'][config.get('whole_process_resource_reference') or config['resource_reference']]
        def ratios(values, reference):
            result = {}
            for key in values.keys() & reference.keys():
                if isinstance(values[key], dict) and isinstance(reference[key], dict):
                    result[key] = ratios(values[key], reference[key])
                elif type(values[key]) in (int, float) and type(reference[key]) in (int, float) and reference[key] != 0:
                    result[key] = values[key] / reference[key]
            return result
        for entry in report['summary'].values():
            entry['resource_ratios_to_fastest_rust'] = {
                'whole_process': ratios(entry['process_resources_median'], whole_baseline['process_resources_median']),
            }
            for window in ['processing', 'post_json']:
                key = window+'_measurement_median'
                if key in entry:
                    data = entry[key]
                    if key in baseline:
                        entry['resource_ratios_to_fastest_rust'][window] = ratios(data, baseline[key])
                    entry[window+'_instructions_per_post'] = data['hardware']['instructions_retired'] / report['input']['posts']
                    entry[window+'_cycles_per_post'] = data['hardware']['cycles'] / report['input']['posts']
    save()
    print(json.dumps(report['summary'], indent=2))


if __name__ == '__main__':
    main()
