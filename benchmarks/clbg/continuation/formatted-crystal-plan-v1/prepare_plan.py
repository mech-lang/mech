"""Prepare exact Crystal formatter plans under common; start no native process.

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
COHORT = CLBG / 'crystal-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BASELINE = CONTINUATION / 'jobs/current-chart-session-v2/candidate-data.json'
BASELINE_SHA = '27b1c792b253d3d465ca6f853e7f85c9d089cb87e3b161a634feb7b5c1f69c8e'
SUITE = COHORT / 'reconciliation/full-v1-suite-v2.json'
SUITE_SHA = '386356315c9fc5c680e99077fa70fe52db130c369fe74f7e0aa301e5f7ea6490'
LOC = COHORT / 'source-counts/corrected-qualified-overlay-v2/qualified-loc-overlay.json'
LOC_SHA = 'e22140622a5154a1e3b9512580eb469c8c078beda4a2ae9812bf429e51e089e2'
METADATA = CONTINUATION / 'publication-crystal-metadata-v2/qualified/actual-metadata-v2/metadata.json'
METADATA_SHA = '63b93da6f941ae4ea276d6e595d29b438ba64e315b29f461874a0edbedfa1a1f'
FIXTURES = FORMATTED / 'jobs/fixtures23-v1'
FIXTURE_SHA = '7567ddfb5a03df5ed4813dd7d87bfd48b8f300ef0a5be66c90504d377fca81b5'
FIXTURE_TERMINAL_SHA = '5b3da339f1f8e575d34742f119996b684b40e03e32365cad749e2a23a61b94bd'
FIXTURE_REVIEW = CONTINUATION / 'jobs/formatted-loc23-actual-review-v1/receipt.json'
PYTHON = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12')
VENDOR = Path('/private/tmp/mech-clbg-compact-libs/pygments')
VENDOR_RECEIPT = CLBG / 'loc-normalization/20261012/c3-proposal-v5/jobs/fixtures-v5/fixture-receipt.json'
VENDOR_RECEIPT_SHA = 'ef90317a26782bb8e6801ccfb125533ffc1352e77595d81840088972209263a5'
SNAPSHOT = CLBG / 'loc-normalization/20261012/jobs/boundary-audit-18/snapshot'
CRYSTAL_MASK = CLBG / 'loc-normalization/20261012/crystal-brace-repair-v2/jobs/fixtures-replay-v1/snapshot/foreign_crystal.py'
BENCHMARKS = ('binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
              'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux')


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision', 'source-sha256', 'shim-sha256', 'review-sha256', 'fixture-review-sha256'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    require(args.revision and all(char.isalnum() or char in '-_' for char in args.revision), 'Fresh revision required')
    require(sys.dont_write_bytecode, 'Use a fresh Python -E -S -B process')
    print('WAIT common mutex: prepare exact Crystal20 native formatter plans only', flush=True)
    with LOCK.open('a+') as common:
        fcntl.flock(common, fcntl.LOCK_EX)
        opened, named = os.fstat(common.fileno()), LOCK.stat()
        require((opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino), 'Common mutex inode differs')
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
            for path, value in ((Path(__file__), args.source_sha256), (HERE / 'physical_crystal.py', args.shim_sha256),
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
            baseline = read(BASELINE, BASELINE_SHA)
            suite = read(SUITE, SUITE_SHA)
            loc = read(LOC, LOC_SHA)
            metadata = read(METADATA, METADATA_SHA)
            require(suite['language'] == loc['language'] == metadata['language'] == 'crystal' and
                    set(suite['benchmarks']) == set(BENCHMARKS) and loc['status'] == 'qualified-exact-source-LOC' and
                    loc['profile_count'] == 20 and loc['complete_authored_source_bundles'] is True and loc['unresolved'] == [] and
                    metadata['status'] == 'reviewed-actual-native-language-metadata', 'Actual published Crystal20 evidence required')
            merge(suite['provenance']); merge(loc['pins']); merge(metadata['pins'])
            # Only this pure, fixture-bound module is executed in the builder.
            # It imports Python's copy module, with no external dependency.
            overlay_path = FORMATTED / 'overlay.py'
            raw = overlay_path.read_bytes()
            require(hashlib.sha256(raw).hexdigest() == pins[str(overlay_path)], 'Exact fixture-qualified overlay changed')
            module = types.ModuleType('prepared_crystal_overlay')
            module.__file__ = str(overlay_path)
            exec(compile(raw, str(overlay_path), 'exec'), module.__dict__)
            module.validate_exclusions(baseline)
            catalog = module.catalog(baseline)
            selected = {key: row for key, row in catalog.items() if row['language'] == 'crystal'}
            expected_ids = {'crystal-' + benchmark + '-' + form for benchmark in BENCHMARKS for form in ('compact', 'optimized')}
            require(len(selected) == 20 and {row['id'] for row in selected.values()} == expected_ids and
                    set(loc['compiler_bound_input_inventory']) == expected_ids, 'Full published20 source inventory required')
            modules = {}
            for name, path in (('physical_crystal', HERE / 'physical_crystal.py'), ('foreign_crystal', CRYSTAL_MASK),
                               ('foreign_core', SNAPSHOT / 'foreign_core.py'), ('canonical', SNAPSHOT / 'canonical.py')):
                require(str(path) in pins, 'Qualified lexical input is missing its actual pin')
                modules[name] = dict(path=str(path), sha256=pins[str(path)])
            # The qualified Crystal mask calls no Pygments lexer. Its retained
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
            compiler = None
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
                    require(source['sha256'] == original['sha256'] and Path(original['path']).suffix == '.cr',
                            'Crystal formatter requires every actual authored input to be Crystal')
                    path, _ = bind(original['path'], original['sha256'])
                    require(path.read_bytes().decode('utf-8') == source['code'], 'Published source differs from actual compiler input')
                    inputs.append(dict(published_file=public_file, path=str(path), sha256=original['sha256'],
                                       language='crystal', role='application' if index == 0 else 'application-helper'))
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
                image = inventory['actual_build_command'][0]
                require(compiler is None or compiler == image, 'Mixed formatter compiler distributions')
                compiler = image
                profiles[key] = dict(language='crystal', files=inputs, compiler_inventory=descriptor, measurement_evidence=evidence)
                identities[key] = actual
            runtime = metadata['measured_backend_template']['runtime_receipt']
            runtime_receipt = read(runtime['report'], runtime['report_sha256'])
            require(runtime['verified'] is True and runtime_receipt['status'] == 'verified-actual-Crystal-compiler-runtime-and-memory',
                    'Actual compiler runtime receipt required')
            expected_version = runtime_receipt['actual_compiler_version_stdout']
            version_receipt = read(runtime_receipt['actual_version_receipt']['path'], runtime_receipt['actual_version_receipt']['sha256'])
            require(version_receipt['command'] == [compiler, '--version'] and version_receipt['inherited_common_mutex'] is True and
                    type(version_receipt['exit_status']) is int and version_receipt['exit_status'] == version_receipt['final_exit_status'] == 0,
                    'Actual compiler/version route differs')
            version_stdout, _ = bind(runtime_receipt['actual_version_stdout']['path'], runtime_receipt['actual_version_stdout']['sha256'])
            require(version_stdout.read_bytes().decode('utf-8') == expected_version, 'Actual compiler version text differs')
            version_stderr = version_stdout.with_suffix('.stderr')
            bind(version_stderr, metadata['pins'][str(version_stderr)])
            require(version_stderr.read_bytes() == b'', 'Actual compiler version stderr differs')
            distribution = Path(compiler).parents[2]
            format_sources = [distribution / 'src/compiler/crystal/command/format.cr',
                              distribution / 'src/compiler/crystal/tools/formatter.cr']
            for path in format_sources: bind(path, metadata['pins'][str(path)])
            config = output / 'native-formatter-config.json'
            save(config, dict(language='crystal', version='1.21.1', argv=['tool','format','--no-color','{input}'],
                              output_mode='in-place', style='Exact pinned Crystal built-in formatter defaults',
                              explicit_file_only=True, ambient_config_files_read=False,
                              source_pins={str(path): pins[str(path)] for path in format_sources}))
            recipe = dict(status='qualified', provider='language-project',
                official_sources=['https://github.com/crystal-lang/crystal/blob/1.21.1/src/compiler/crystal/command/format.cr',
                                  'https://github.com/crystal-lang/crystal/blob/1.21.1/src/compiler/crystal/tools/formatter.cr'],
                tool=dict(path=compiler, sha256=pins[compiler], version=expected_version.splitlines()[0]),
                config=dict(path=str(config), sha256=pins[str(config)]), environment={'PATH':'/usr/bin:/bin', 'CRYSTAL_WORKERS':'1'},
                version_arguments=['--version'], expected_version_stdout=expected_version, expected_version_stderr='',
                output_mode='in-place', arguments=['tool','format','--no-color','{input}'], timeout_seconds=120)
            counter = dict(module='physical_crystal', module_path=str(HERE / 'physical_crystal.py'),
                           module_sha256=pins[str(HERE / 'physical_crystal.py')], function='mask_foreign', argument='crystal')
            plan = dict(status='root-reviewed-formatted-loc-plan', chart_admission=False, application_execution_authorized=False,
                physical_definition='nonblank-noncomment-physical-lines-v1', source_dataset=dict(path=str(BASELINE), sha256=BASELINE_SHA),
                fixture_qualification=dict(receipt_path=str(FIXTURES / 'fixture-receipt.json'), receipt_sha256=FIXTURE_SHA,
                                           terminal_path=str(FIXTURES / 'terminal.json'), terminal_sha256=FIXTURE_TERMINAL_SHA),
                profiles=profiles, measurement_identities=identities, formatters={'crystal':recipe},
                counters={'crystal':counter}, code_modules=modules)
            # Plan pins deliberately exclude either plan's own bytes. The
            # worker adds its separately reviewed input plan SHA before use.
            plan['pins'] = dict(pins)
            save(output / 'cohort-plan.json', plan)
            probe_keys = {key for key, row in selected.items() if row['id'] in
                          {'crystal-regexredux-compact', 'crystal-fannkuchredux-optimized'}}
            require(len(probe_keys) == 2, 'Exact copied-input probe representatives required')
            probe = copy.deepcopy(plan)
            probe['profiles'] = {key: value for key, value in profiles.items() if key in probe_keys}
            probe['measurement_identities'] = {key: value for key, value in identities.items() if key in probe_keys}
            save(output / 'probe-plan.json', probe)
            report.update(status='prepared-exact-Crystal20-plans-awaiting-root-digest-review', profiles=20,
                          probe_profiles=2, source_dataset_sha256=BASELINE_SHA,
                          cohort_plan=dict(path=str(output / 'cohort-plan.json'), sha256=pins[str(output / 'cohort-plan.json')]),
                          probe_plan=dict(path=str(output / 'probe-plan.json'), sha256=pins[str(output / 'probe-plan.json')]),
                          other_language_formatted_counts_admitted=0)
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
