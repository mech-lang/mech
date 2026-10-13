"""Prepare exact Nim formatter plans under common; start no native process.

Root reviews and freezes these sources before supplying their literal digests.
Root must then review the generated actual plan/digest before running measure.
"""
import argparse
import fcntl
import os
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
CONTINUATION = HERE.parent
CLBG = CONTINUATION.parent
FORMATTED = CLBG / 'loc-normalization/20261013/formatted-loc-v1'
COHORT = CLBG / 'nim-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BASELINE = CONTINUATION / 'combined-chart-integration-v3/jobs/current-nim-format-platform-v1/candidate-data.json'
BASELINE_SHA = '8b85d35f9ff8c267171c9d91f3467ba765bf019ac438c64ed2071898a7656378'
SUITE = COHORT / 'reconciliation/full-v6-suite-v2.json'
SUITE_SHA = 'c4dff4ea00db0f380257708cfb0aea50b3a3eaecd1c5ce0f11f6cc780561f3f7'
LOC = COHORT / 'source-counts/helper-owners-v2/jobs/exact-source20-loc-v2/qualified-loc-overlay.json'
LOC_SHA = '55ef7197d8caa62a5087cf93380e524d8a0fac11d2d82a731e44675e3d288398'
METADATA = CONTINUATION / 'publication-nim-metadata-v2/qualified/actual-metadata-v2/metadata.json'
METADATA_SHA = '98efdbdfcdaacbf22c0644e4225b39e29b68ca94db4d392a3cdd436376ce3722'
FIXTURES = FORMATTED / 'jobs/fixtures23-v1'
FIXTURE_SHA = '7567ddfb5a03df5ed4813dd7d87bfd48b8f300ef0a5be66c90504d377fca81b5'
FIXTURE_TERMINAL_SHA = '5b3da339f1f8e575d34742f119996b684b40e03e32365cad749e2a23a61b94bd'
FIXTURE_REVIEW = CONTINUATION / 'jobs/formatted-loc23-actual-review-v1/receipt.json'
PYTHON = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12')
VENDOR = Path('/private/tmp/mech-clbg-compact-libs/pygments')
VENDOR_RECEIPT = CLBG / 'loc-normalization/20261012/c3-proposal-v5/jobs/fixtures-v5/fixture-receipt.json'
VENDOR_RECEIPT_SHA = 'ef90317a26782bb8e6801ccfb125533ffc1352e77595d81840088972209263a5'
SNAPSHOT = CLBG / 'loc-normalization/20261012/jobs/boundary-audit-18/snapshot'
NIM_MASK = SNAPSHOT / 'foreign_nim.py'
NIM_FUTURE_MASK = SNAPSHOT / 'foreign_future_mask.py'
BASELINE_REVIEW = CONTINUATION / 'jobs/combined-integration-v3-closed-review-v1/receipt.json'
METADATA_REVIEW = CONTINUATION / 'jobs/nim-metadata-v2-closed-review-v1/receipt.json'
DISTRIBUTION = COHORT / '.build/private-v3/nim/nim-2.2.12'
NIMPRETTY = DISTRIBUTION / 'bin/nimpretty'
NIMPRETTY_SHA = 'cdf07fca977cfcfbeceb0189317657280ea89b11bed7220524bfef66db862cd5'
NIM_COMMIT = '8e8fbf60693418dc95bb0d762fd660231d08a583'
BENCHMARKS = ('binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
              'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux')


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision', 'source-sha256', 'shim-sha256', 'review-sha256', 'fixture-review-sha256',
                 'baseline-review-sha256', 'metadata-review-sha256'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    require(args.revision and all(char.isalnum() or char in '-_' for char in args.revision), 'Fresh revision required')
    require(sys.dont_write_bytecode, 'Use a fresh Python -E -S -B process')
    print('WAIT common mutex: prepare exact Nim20 native formatter plans only', flush=True)
    with LOCK.open('a+') as common:
        fcntl.flock(common, fcntl.LOCK_EX)
        opened, named = os.fstat(common.fileno()), LOCK.stat()
        require((opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino) and opened.st_ino == 79859134, 'Common mutex inode differs')
        import copy
        import hashlib
        import json
        import traceback
        import types

        output = HERE / 'jobs' / args.revision
        require(output.resolve().is_relative_to(HERE.resolve()) and not output.exists(), 'Output must be fresh and contained')
        output.mkdir(parents=True)
        pins = {}
        body_error = None
        report = dict(status='started-plan-preparation', actual_application_processes=0,
                      actual_formatter_processes=0, actual_fixture_reexecutions=0, chart_admission=False)

        def digest(value):
            return type(value) is str and len(value) == 64 and all(char in '0123456789abcdef' for char in value)

        def bind(path, expected=None):
            path = Path(path).resolve(strict=True)
            value = hashlib.sha256(path.read_bytes()).hexdigest()
            require(expected is None or digest(expected) and value == expected, 'Changed actual input: ' + str(path))
            require(str(path) not in pins or pins[str(path)] == value, 'Conflicting actual pin')
            pins[str(path)] = value
            return path, value

        def read(path, expected):
            path, _ = bind(path, expected)
            return json.loads(path.read_bytes())

        def merge(values):
            require(type(values) is dict and values, 'Actual input pin closure required')
            for path, value in values.items():
                require(Path(path).is_absolute() and digest(value), 'Malformed actual pin')
                bind(path, value)

        def save(path, value):
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('x') as stream:
                json.dump(value, stream, indent=2)
                stream.write('\n')
            path.chmod(0o444)
            bind(path)

        try:
            for path, value in ((Path(__file__), args.source_sha256), (HERE / 'physical_nim.py', args.shim_sha256),
                                (HERE / 'REVIEW.md', args.review_sha256)):
                require(not path.stat().st_mode & 0o222, 'Reviewed builder inputs must be frozen')
                bind(path, value)
            require(Path(sys.executable).resolve() == PYTHON.resolve(), 'Use the actual qualified bundled Python image')
            fixture = read(FIXTURES / 'fixture-receipt.json', FIXTURE_SHA)
            terminal = read(FIXTURES / 'terminal.json', FIXTURE_TERMINAL_SHA)
            review = read(FIXTURE_REVIEW, args.fixture_review_sha256)
            require(fixture['status'] == terminal['status'] == 'passed-formatted-loc-fixtures' and
                    terminal['body_error'] is None and terminal['integrity_errors'] == [] and
                    terminal['available_bytes_not_fully_pinned'] is False and terminal['inherited_common_mutex'] is True,
                    'Actual formatted fixture closure is not qualified')
            require(type(fixture['tests_run']) is int and fixture['tests_run'] == fixture['discovered_tests'] == 23 and
                    len(review['exact_started_ids']) == len(set(review['exact_started_ids'])) == 23 and
                    set(fixture['actual_test_ids']) == set(review['exact_started_ids']) and
                    len(set(fixture['actual_test_ids'])) == 23 and
                    set(fixture['actual_test_ids']) == set(fixture['expected_test_ids']) == set(fixture['discovered_test_ids']),
                    'Exact actual23 identities required')
            require(review['status'] == 'actual-formatted23-fixtures-reviewed' and review['tests_run'] == 23 and
                    review['receipt_sha256'] == FIXTURE_SHA and review['terminal_sha256'] == FIXTURE_TERMINAL_SHA and
                    review['source_snapshot_pins_verified'] == 52 and review['formatter_counts_admitted'] == 0,
                    'Root actual fixture review differs')
            require(all(fixture[key] == [] for key in ('failures','errors','skipped','expected_failures','unexpected_successes')) and
                    fixture['actual_native_subjects'] == fixture['actual_formatter_processes'] == fixture['actual_signals_sent'] == 0,
                    'Fixture phase performed unexpected work')
            require(terminal['receipt'] == str(FIXTURES / 'fixture-receipt.json') and terminal['receipt_sha256'] == FIXTURE_SHA,
                    'Fixture terminal names another receipt')
            merge(terminal['pins']); merge(review['pins'])
            bind(PYTHON.resolve(), terminal['pins'][str(PYTHON.resolve())])
            baseline_review = read(BASELINE_REVIEW, args.baseline_review_sha256)
            metadata_review = read(METADATA_REVIEW, args.metadata_review_sha256)
            require(baseline_review['status'] == 'passed-closed-combined-format-platform-candidate-review' and
                    baseline_review['verified_pins'] == 46991 and
                    baseline_review['candidate'] == dict(path=str(BASELINE), sha256=BASELINE_SHA) and
                    baseline_review['prior_programs_sources_timings_and_canonical_coordinates_preserved'] is True and
                    baseline_review['canonical_baselines_preserved'] is True, 'Actual published baseline closure differs')
            require(metadata_review['status'] == 'root-reviewed-qualified-Nim20-runtime-metadata' and
                    metadata_review['verified_pins'] == 38829 and metadata_review['profile_count'] == 20 and
                    metadata_review['metadata_sha256'] == METADATA_SHA and metadata_review['source_counts_unchanged'] is True and
                    metadata_review['native_processes'] == metadata_review['benchmark_runs'] == 0,
                    'Actual Nim metadata closure differs')
            baseline = read(BASELINE, BASELINE_SHA)
            suite = read(SUITE, SUITE_SHA)
            loc = read(LOC, LOC_SHA)
            metadata = read(METADATA, METADATA_SHA)
            require(suite['language'] == loc['language'] == metadata['language'] == 'nim' and
                    set(suite['benchmarks']) == set(BENCHMARKS) and loc['status'] == 'qualified-exact-source-LOC' and
                    loc['profile_count'] == 20 and loc['complete_authored_source_bundles'] is True and loc['unresolved'] == [] and
                    metadata['status'] == 'reviewed-actual-native-language-metadata', 'Actual published Nim20 evidence required')
            merge(suite['provenance']); merge(loc['pins']); merge(metadata['pins'])
            # Only this pure, fixture-bound module is executed in the builder.
            # It imports Python's copy module, with no external dependency.
            overlay_path = FORMATTED / 'overlay.py'
            raw = overlay_path.read_bytes()
            require(hashlib.sha256(raw).hexdigest() == pins[str(overlay_path)], 'Exact fixture-qualified overlay changed')
            module = types.ModuleType('prepared_nim_overlay')
            module.__file__ = str(overlay_path)
            exec(compile(raw, str(overlay_path), 'exec'), module.__dict__)
            module.validate_exclusions(baseline)
            catalog = module.catalog(baseline)
            selected = {key: row for key, row in catalog.items() if row['language'] == 'nim'}
            expected_ids = {'nim-' + benchmark + '-' + form for benchmark in BENCHMARKS for form in ('compact', 'optimized')}
            require(len(selected) == 20 and {row['id'] for row in selected.values()} == expected_ids and
                    set(loc['compiler_bound_input_inventory']) == expected_ids, 'Full published20 source inventory required')
            modules = {}
            for name, path in (('physical_nim', HERE / 'physical_nim.py'), ('foreign_nim', NIM_MASK), ('foreign_future_mask', NIM_FUTURE_MASK),
                               ('foreign_core', SNAPSHOT / 'foreign_core.py'), ('canonical', SNAPSHOT / 'canonical.py')):
                require(str(path) in pins, 'Qualified lexical input is missing its actual pin')
                modules[name] = dict(path=str(path), sha256=pins[str(path)])
            # The qualified Nim mask calls no Pygments lexer. Its retained
            # foreign_core import nevertheless imports Pygments infrastructure.
            # Copy/pin that complete vendor source closure so ambient packages
            # cannot supply any of those modules in the later fresh worker.
            require(pins[str(VENDOR / '__init__.py')] == 'ff7513f3a4c6a47b96f057a4759f2e2e2744647d4d86670b88ecb628a34f0ade',
                    'Actual qualified Pygments package identity differs')
            vendor_receipt = read(VENDOR_RECEIPT, VENDOR_RECEIPT_SHA)
            require(vendor_receipt['status'] == 'passed-awaiting-source-review' and
                    vendor_receipt['pins_before'] == vendor_receipt['pins_after'] and
                    vendor_receipt['pin_reconciliation_passed'] is True,
                    'Retained generic vendor dependency closure differs')
            vendor_pins = {Path(path): value for path, value in vendor_receipt['pins_before'].items()
                           if Path(path).is_relative_to(VENDOR) and Path(path).suffix == '.py'}
            vendor_paths = sorted(VENDOR.rglob('*.py'))
            require(vendor_paths and set(vendor_paths) == set(vendor_pins) and
                    all(not path.is_symlink() for path in vendor_paths),
                    'Vendor source must match the complete actual recorded nonsymlink Python inventory')
            for path in vendor_paths:
                source, value = bind(path, vendor_pins[path])
                target = output / 'imports/pygments' / source.relative_to(VENDOR)
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(source.read_bytes()); target.chmod(0o444)
                bind(target, value)
                parts = source.relative_to(VENDOR.parent).with_suffix('').parts
                name = '.'.join(parts[:-1] if parts[-1] == '__init__' else parts)
                modules[name] = dict(path=str(target), sha256=value, root=str(output / 'imports'))
            profiles, identities = {}, {}
            compiler_roots, authored_inputs = set(), []
            for key, published in selected.items():
                benchmark, identifier = published['benchmark'], published['id']
                measured = suite['benchmarks'][benchmark]['profiles'][identifier]
                actual = module.identity(baseline, key)
                require(measured['id'] == identifier and measured['key'] == key and
                        actual['seconds'] == measured['median_seconds'] and actual['samples'] == measured['samples_seconds'] and
                        actual['command'] == measured['command'] and actual['binary_sha256'] == measured['binary_sha256'],
                        'Published measured profile changed')
                reference = loc['compiler_bound_input_inventory'][identifier]
                inventory = read(reference['evidence_path'], reference['evidence_sha256'])
                require(inventory['files'] == reference['files'], 'Actual compiler input bundle differs')
                files = published['files']
                require(len(files) == len(inventory['files']), 'Whole authored bundle required; no main-only fallback')
                inputs = []
                for index, (public_file, original) in enumerate(zip(files, inventory['files'])):
                    source = module.source_identity(baseline, public_file)
                    require(source['sha256'] == original['sha256'] and Path(original['path']).suffix == '.nim',
                            'Nim formatter requires every actual authored input to be Nim')
                    path, _ = bind(original['path'], original['sha256'])
                    require(path.read_bytes().decode('utf-8') == source['code'], 'Published source differs from actual compiler input')
                    inputs.append(dict(published_file=public_file, path=str(path), sha256=original['sha256'],
                                       language='nim', role='application' if index == 0 else 'application-helper'))
                descriptor = dict(evidence_path=reference['evidence_path'], evidence_sha256=reference['evidence_sha256'],
                                  binding_field='', binding=inventory)
                module.validate_bundle(baseline, key, inputs, inventory)
                fields = dict(profile_id='/id', benchmark='/benchmark', source_sha256='/source_sha256',
                              binary_sha256='/binary_sha256', command='/command', samples='/samples_seconds',
                              command_image_sha256='/binary_sha256')
                pointer = '/benchmarks/' + benchmark + '/profiles/' + identifier
                evidence = dict(path=str(SUITE), sha256=SUITE_SHA, binding_field=pointer,
                                binding=copy.deepcopy(measured), fields=fields)
                require(module.measured_execution(dict(pins=pins, receipts={str(SUITE): suite}), baseline, key, inputs,
                    inventory, evidence, {str(BASELINE), reference['evidence_path']}), 'Independent recorded executable image not bound')
                original_ref = inventory['original_compiler_input_evidence']
                original_inventory = read(original_ref['path'], original_ref['sha256'])
                require(inventory['profile'] == original_inventory['id'] == identifier and
                        inventory['independently_retained_compiler_input_evidence'] == original_inventory['processing_evidence'] and
                        {(item['path'], item['sha256']) for item in original_inventory['source_bundle']} ==
                        {(item['path'], item['sha256']) for item in inventory['files']},
                        'Original actual Nim Processing/source bundle differs')
                image = original_inventory['original_build_command'][0]
                bind(image, metadata['pins'][image])
                compiler_roots.add(image)
                authored_inputs.extend(inputs)
                profiles[key] = dict(language='nim', files=inputs, compiler_inventory=descriptor, measurement_evidence=evidence)
                identities[key] = actual
            require(len(authored_inputs) == 21 and len({item['path'] for item in authored_inputs}) == 21,
                    'All20 mains plus the exact counted GMP helper required')
            helpers = [item for item in authored_inputs if item['role'] == 'application-helper']
            require(len(helpers) == 1 and helpers[0]['path'] == str(COHORT / 'qualification/private-v3/source-snapshot/support/gmp_ops.nim') and
                    helpers[0]['sha256'] == 'adad6363f378372c1f54b8382193a70c8e142565589bb2511ba6567838f8b3e4',
                    'Missing or changed authored GMP helper')
            runtime = metadata['measured_backend_template']['runtime_receipt']
            runtime_receipt = read(runtime['report'], runtime['report_sha256'])
            require(runtime['verified'] is True and runtime['report_sha256'] == metadata_review['runtime_receipt_sha256'] and
                    runtime_receipt['status'] == 'verified-actual-Nim-C-Clang-ORC-compiler-runtime-and-memory' and
                    set(runtime_receipt['original_compiler_proofs']) == compiler_roots and len(compiler_roots) == 3,
                    'Actual original three compiler roots differ')
            for image, proof in runtime_receipt['original_compiler_proofs'].items():
                require(proof['compiler'] == image and proof['compiler_sha256'] == pins[image],
                        'Original Nim compiler image pin differs')
                version = read(proof['receipt'], proof['receipt_sha256'])
                require(version['command'] == [image, '--version'] and version['inherited_common_mutex'] is True and
                        type(version['exit_status']) is int and version['exit_status'] == version['final_exit_status'] == 0,
                        'Original actual Nim version route differs')
                actual_stdout, _ = bind(proof['stdout'], proof['stdout_sha256'])
                require(actual_stdout.read_text() == runtime_receipt['actual_nim_version_stdout'] and
                        NIM_COMMIT in runtime_receipt['actual_nim_version_stdout'], 'Original actual compiler version differs')
            # Select the official standalone tool from the preserved distribution.
            # Its pin was independently recorded, not minted from current bytes.
            bind(NIMPRETTY, NIMPRETTY_SHA)
            require(metadata['pins'][str(NIMPRETTY)] == loc['pins'][str(NIMPRETTY)] == NIMPRETTY_SHA,
                    'Native formatter lacks its independently recorded image pin')
            format_sources = [DISTRIBUTION / relative for relative in
                ('nimpretty/nimpretty.nim', 'compiler/layouter.nim', 'compiler/parser.nim',
                 'compiler/lexer.nim', 'compiler/syntaxes.nim')]
            for path in format_sources: bind(path, metadata['pins'][str(path)])
            require(pins[str(format_sources[0])] == 'ef9b8a1f6f05a3cdbb10d7b026d01b10f2e93b6a8299fdce9621948a22914ed9',
                    'Official Nimpretty source differs')
            arguments = ['--indent:0', '--maxLineLen:80', '--out:{output}', '{input}']
            config = output / 'native-formatter-config.json'
            save(config, dict(language='nim', distribution='Nim2.2.12', tool='nimpretty', version='0.2',
                              argv=arguments, output_mode='file', indent='official default autodetection',
                              max_line_length=80, explicit_file_only=True, ambient_config_files_read=False,
                              separate_required_output=True, source_pins={str(path): pins[str(path)] for path in format_sources}))
            recipe = dict(status='qualified', provider='language-project',
                official_sources=['https://github.com/nim-lang/Nim/blob/' + NIM_COMMIT + '/' + str(path.relative_to(DISTRIBUTION))
                                  for path in format_sources],
                tool=dict(path=str(NIMPRETTY), sha256=NIMPRETTY_SHA, version='0.2'),
                config=dict(path=str(config), sha256=pins[str(config)]), environment={'PATH':'/usr/bin:/bin'},
                version_arguments=['--version'], expected_version_stdout='0.2\n', expected_version_stderr='',
                output_mode='file', arguments=arguments, timeout_seconds=120)
            counter = dict(module='physical_nim', module_path=str(HERE / 'physical_nim.py'),
                           module_sha256=pins[str(HERE / 'physical_nim.py')], function='mask_foreign', argument='nim')
            plan = dict(status='root-reviewed-formatted-loc-plan', chart_admission=False, application_execution_authorized=False,
                physical_definition='nonblank-noncomment-physical-lines-v1', source_dataset=dict(path=str(BASELINE), sha256=BASELINE_SHA),
                fixture_qualification=dict(receipt_path=str(FIXTURES / 'fixture-receipt.json'), receipt_sha256=FIXTURE_SHA,
                                           terminal_path=str(FIXTURES / 'terminal.json'), terminal_sha256=FIXTURE_TERMINAL_SHA),
                profiles=profiles, measurement_identities=identities, formatters={'nim':recipe},
                counters={'nim':counter}, code_modules=modules)
            # Plan pins deliberately exclude either plan's own bytes. The
            # worker adds its separately reviewed input plan SHA before use.
            plan['pins'] = dict(pins)
            save(output / 'cohort-plan.json', plan)
            report.update(status='prepared-exact-Nim20-plan-awaiting-root-digest-review', profiles=20,
                          authored_file_receipts_required=21, counted_GMP_helper_included=True,
                          source_dataset_sha256=BASELINE_SHA,
                          cohort_plan=dict(path=str(output / 'cohort-plan.json'), sha256=pins[str(output / 'cohort-plan.json')]),
                          actual_formatter_version_observation_pending=True, other_language_formatted_counts_admitted=0)
        except BaseException as failure:
            body_error = dict(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
            report['status'] = 'failed-plan-preparation'
            with (output / 'exception.log').open('x') as stream:
                traceback.print_exc(file=stream)
            raise
        finally:
            errors = []
            for path in sorted(output.rglob('*')):
                if path.is_file():
                    try: bind(path, pins.get(str(path.resolve())))
                    except BaseException as failure: errors.append(dict(path=str(path), error=str(failure)))
            for path, value in list(pins.items()):
                try: bind(path, value)
                except BaseException as failure: errors.append(dict(path=path, error=str(failure)))
            report.update(body_error=body_error, integrity_errors=errors, available_bytes_not_fully_pinned=bool(errors),
                          pins=dict(pins), inherited_common_mutex=True)
            if errors: report['status'] = 'failed-terminal-integrity'
            save(output / 'terminal.json', report)
            if errors: raise RuntimeError('Plan preparation integrity failed; partial evidence retained')


if __name__ == '__main__':
    main()
