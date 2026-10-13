"""Draft guarded ordinary stdlib and isolated application work proofs."""
import copy
import importlib.util
from pathlib import Path

HERE = Path(__file__).resolve().parent


def run(preparation, guard, harness, proposal, snapshot, build, compiler, libraries,
        environment, rows, pins, lock_fd, report, references):
    spec = importlib.util.spec_from_file_location('crystal_actual_trace_reconciliation', snapshot / 'source_trace.py')
    trace = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(trace)
    # The source snapshot has a different parent depth; keep only the exact
    # reviewed independent reconciler location, never a fallback/source search.
    trace.V = preparation.V
    result = dict(status='qualifying', reviews=[], pins=dict(pins),
                  native_benchmark_kernel=False, preparation_only=True, never_chart_eligible=True)
    destination = guard.OUT / 'work-proofs'
    destination.mkdir()
    source = snapshot / 'support/standard_primitives.cr'
    binary = build / 'bin/standard-primitives'
    cache = build / 'cache/standard-primitives'
    cache.mkdir()
    command = preparation.build_command(compiler, preparation.SDK, libraries, source, binary)
    preparation.owned_command(guard, harness, command, dict(environment, CRYSTAL_CACHE_DIR=str(cache)),
        destination / 'standard-primitives/build', lock_fd, 600, 'build')
    result['pins'] = preparation.merge(result['pins'], preparation.pin_tree(cache),
        {str(binary): preparation.sha(binary)}, preparation.native_images(guard, harness, binary,
            environment, destination / 'standard-primitives/images', lock_fd,
            [preparation.retained_release(), preparation.GMP, preparation.PCRE2]))
    # This is explicitly a generic stdlib proof, using the shared sampler's
    # pidigits dispatch only as transport for an independently checked stdout.
    row = dict(id='crystal-standard-primitives', key='pidigits:crystal-standard-primitives',
        benchmark='pidigits', language='crystal', source=str(source), source_sha256=preparation.sha(source),
        command=[str(binary)], runtime_environment={k: environment[k] for k in
            ('LC_ALL', 'CRYSTAL_WORKERS', 'DYLD_LIBRARY_PATH', 'OMP_NUM_THREADS',
             'OPENBLAS_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS')},
        measurement_kind='general-standard-library-primitive-proof', cwd=str(preparation.HERE))
    attempt, directory = preparation.probe(guard, harness, row, 0, None, b'PRIMITIVES_OK\x00\n', result['pins'],
        destination, lock_fd, report, 'actual-standard-apis')
    result['pins'] = preparation.merge(result['pins'],
        {str(p): preparation.sha(p) for p in directory.iterdir() if p.is_file()})
    result['reviews'].append(dict(status='passed', scope='standard-primitives',
        floor_and_truncated_division=True, immutable_bigint_lifetime=True,
        long_string_constructor=True, complete_private_map_handoff=True,
        actual_capped_context=True, regex_nonoverlap_and_order=True, binary_nul_stdout=True,
        native_benchmark_kernel=False, never_chart_eligible=True))
    for original in rows:
        benchmark = original['benchmark']
        if benchmark not in ('knucleotide', 'pidigits', 'binarytrees'):
            continue
        source_directory = destination / original['id']
        source_directory.mkdir()
        text, edits = trace.instrument_text(Path(original['source']).read_text(), benchmark, original['variant'])
        source = source_directory / 'work-trace.cr'
        source.write_text(text)
        preparation.new_json(source_directory / 'edits.json', dict(edits,
            scored_source=original['source'], scored_source_sha256=original['source_sha256']))
        binary = source_directory / 'work-trace'
        cache = build / 'cache' / (original['id'] + '-work-trace')
        cache.mkdir()
        preparation.owned_command(guard, harness,
            preparation.build_command(compiler, preparation.SDK, libraries, source, binary),
            dict(environment, CRYSTAL_CACHE_DIR=str(cache)), source_directory / 'build', lock_fd, 600, 'build')
        result['pins'] = preparation.merge(result['pins'], preparation.pin_tree(cache),
            {str(p): preparation.sha(p) for p in (source, binary, source_directory / 'edits.json')},
            preparation.native_images(guard, harness, binary, environment, source_directory / 'images',
                                      lock_fd, [preparation.retained_release(), preparation.GMP, preparation.PCRE2]))
        row = copy.deepcopy(original)
        row.update(id=original['id'] + '-work-trace', key=benchmark + ':' + original['id'] + '-work-trace',
            source=str(source), source_sha256=preparation.sha(source), binary=str(binary),
            binary_sha256=preparation.sha(binary), command=[str(binary), '{n}'],
            measurement_kind='actual-application-required-work-trace')
        if benchmark == 'knucleotide':
            cases = [(name, harness.WORK[benchmark][0], data) for name, data in preparation.inputs(benchmark, references)]
        elif benchmark == 'pidigits':
            cases = [('digits-97', 97, None), ('zero-digits', 0, None)]
        else:
            cases = [('allocated-n0', 0, None), ('allocated-n7', 7, None), ('allocated-n9', 9, None)]
        for name, n, data in cases:
            directory = destination / row['id'] / name
            trace_path = directory / 'actual.trace'
            row['runtime_environment'] = dict(original['runtime_environment'], MECH_CRYSTAL_WORK_TRACE=str(trace_path))
            expected = getattr(references, benchmark)(data if data is not None else n)
            def reconcile(actual_directory):
                assert actual_directory == directory and trace_path.is_file()
                payload = trace_path.read_bytes()
                stdout = (directory / 'actual.stdout').read_bytes()
                if benchmark == 'knucleotide':
                    return trace.reconcile_maps(payload, data, stdout, original['variant'], common_timing_mutex_fd=lock_fd)
                if benchmark == 'pidigits':
                    return trace.reconcile_pi(payload, stdout, n, original['variant'], common_timing_mutex_fd=lock_fd)
                return trace.reconcile_trees(payload, stdout, n, original['variant'], common_timing_mutex_fd=lock_fd)

            attempt, actual_directory = preparation.probe(guard, harness, row, n, data, expected,
                result['pins'], destination, lock_fd, report, name, trace=reconcile)
            review = attempt['source_work_proof']
            assert review['status'] == 'passed'
            review.update(profile=original['id'], case=name, attempt=attempt['attempt'],
                          scored_source_sha256=original['source_sha256'],
                          trace_source_sha256=preparation.sha(source), measured_source_bytes_unchanged=True)
            preparation.new_json(directory / 'work-review.json', review)
            result['reviews'].append(review)
            result['pins'] = preparation.merge(result['pins'],
                {str(p): preparation.sha(p) for p in directory.iterdir() if p.is_file()})
            guard.verify(result['pins'])
        assert preparation.sha(original['source']) == original['source_sha256']
    result['status'] = 'passed'
    return result
