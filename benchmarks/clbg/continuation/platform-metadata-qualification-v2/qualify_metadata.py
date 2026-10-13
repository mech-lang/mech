"""Reconcile preserved platform evidence; never execute a benchmark or runtime.

The caller owns the common timing mutex for this entire operation. Missing
historical backend evidence becomes an explicit unresolved annotation. Changed
report/source/sample identity is an integrity failure, never a guessed backend.
"""
import hashlib
import json
import math
from pathlib import Path
import statistics
import struct

LANGUAGES = {'java': 17, 'erlang': 14, 'csharp': 20, 'csharp-aot': 19,
             'fsharp': 15, 'fsharp-aot': 15}
JAVA_REPORTS = {'retime/results/apple-m3-ac-normal-20261008.json',
                'regexredux/results/apple-m3-ac-normal-20261009.json',
                'pidigits/results/apple-m3-ac-normal-20261009.json'}
ERLANG_BUILD = 'erlang-comparison/20261010/results/builds.json'
DOTNET_SUMMARY = 'remaining-languages-comparison/20261011/results/summary.json'
DOTNET_AUDIT = 'remaining-languages-comparison/20261011/source-audit-v2.json'
DOTNET_AOT = 'remaining-languages-comparison/20261011/dotnet_aot_builds.json'
BACKENDS = {'java': ('openjdk-jit', 'jit'), 'erlang': ('beam-jit', 'jit'),
            'csharp': ('dotnet-jit', 'jit'), 'fsharp': ('dotnet-jit', 'jit'),
            'csharp-aot': ('dotnet-native-aot', 'aot'),
            'fsharp-aot': ('dotnet-native-aot', 'aot')}


class UnresolvedBackend(ValueError):
    """A valid preserved measurement lacks enough historical backend evidence."""


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def backend_require(condition, reason):
    if not condition:
        raise UnresolvedBackend(reason)


def digest_value(value):
    return type(value) is str and len(value) == 64 and all(c in '0123456789abcdef' for c in value)


def command(value):
    return type(value) is list and value and all(type(part) is str and part for part in value)


def records(report):
    value = report['programs']
    require(type(value) in (list, dict), 'Invalid preserved program inventory')
    return list(value.items()) if isinstance(value, dict) else list(enumerate(value))


def select(report, displayed):
    found = [(index, row) for index, row in records(report)
             if row['id'] == displayed['id'] and
             row.get('benchmark', report.get('benchmark', displayed['benchmark'])) == displayed['benchmark'] and
             row['language'] == displayed['language']]
    require(len(found) == 1, 'Expected one exact preserved language/benchmark/id record')
    return found[0]


def field(index, suffix):
    # JSON pointer, rather than a dotted name with ambiguous dictionary keys.
    escaped = str(index).replace('~', '~0').replace('/', '~1')
    return '/programs/' + escaped + '/' + suffix


def validate_samples(displayed, saved):
    samples = saved['samples_seconds']
    require(type(samples) is list and len(samples) == 3 and
            all(type(value) in (int, float) and math.isfinite(value) and value > 0 for value in samples),
            'Expected three finite positive historical samples')
    require(displayed['samples'] == samples, 'Displayed samples differ from preserved samples')
    median = statistics.median(samples)
    require(type(saved['median_seconds']) in (int, float) and
            math.isclose(saved['median_seconds'], median, rel_tol=0, abs_tol=1e-10),
            'Preserved median differs from preserved samples')
    require(type(displayed['seconds']) in (int, float) and
            math.isclose(displayed['seconds'], median, rel_tol=0, abs_tol=1e-10),
            'Displayed elapsed time differs from preserved median')


def validate_inventory(data, expected):
    require(type(expected) is list and len(expected) == len(set(expected)) == 100,
            'Expected exact literal 100-ID inventory')
    selected = {key: row for key, row in data['programs'].items() if row['language'] in LANGUAGES}
    require(set(selected) == set(expected), 'Admitted platform program inventory changed')
    for language, count in LANGUAGES.items():
        require(sum(row['language'] == language for row in selected.values()) == count,
                'Admitted language count changed: ' + language)
    for key, row in selected.items():
        require(key == row['key'] == row['benchmark'] + ':' + row['id'], 'Program key/identity mismatch')
    return selected


def java_backend(saved):
    cmd = saved.get('command')
    mode = saved.get('execution_mode')
    backend_require(command(cmd) and '-Xint' not in cmd and cmd[0] == '/opt/homebrew/opt/openjdk/bin/java' and
                    '-cp' in cmd and cmd.index('-cp') + 2 < len(cmd),
                    'Saved command does not establish the measured OpenJDK classpath launcher')
    backend_require(type(mode) is str and mode.startswith('OpenJDK JIT'),
                    'Saved row lacks explicit historical OpenJDK JIT execution evidence')
    compile_cmd = saved.get('compile_command')
    classpath = java_classpath(cmd)
    backend_require(command(compile_cmd) and compile_cmd[0] == '/opt/homebrew/opt/openjdk/bin/javac' and
                    '-d' in compile_cmd and compile_cmd.index('-d') + 1 < len(compile_cmd) and
                    compile_cmd[compile_cmd.index('-d') + 1] == classpath[0] and
                    (len(classpath) == 1 or '-cp' in compile_cmd and
                     compile_cmd.index('-cp') + 1 < len(compile_cmd) and
                     compile_cmd[compile_cmd.index('-cp') + 1] == cmd[cmd.index('-cp') + 1]),
                    'Saved Java compilation does not bind the measured classpath')
    return cmd


def java_classpath(cmd):
    backend_require(command(cmd) and cmd.count('-cp') == 1 and cmd.index('-cp') + 2 < len(cmd),
                    'Saved Java classpath argument is missing or ambiguous')
    entries = cmd[cmd.index('-cp') + 1].split(':')
    backend_require(entries and len(entries) == len(set(entries)) and
                    all(entry and Path(entry).is_absolute() for entry in entries),
                    'Saved macOS Java classpath contains empty, duplicate or relative entries')
    return entries


def compare_retime_java_lineage(saved, original, benchmark, workloads):
    # Compare the preserved templates, not a fabricated unchanged old command.
    # The retime report itself records the workload used for its new samples.
    compare_saved_fields(saved, original, ('compile_command', 'execution_mode', 'source', 'sha256'))
    require(type(workloads) is dict and benchmark in workloads, 'Saved retime workload missing')
    sizes = workloads[benchmark]
    require(type(sizes) is list and len(sizes) == 2 and
            all(type(value) is int and value > 0 for value in sizes), 'Invalid saved retime workload')
    before, after = original['command'], saved['command']
    java_backend(original); java_backend(saved)
    split = before.index('-cp') + 3
    prefix, arguments = before[:split], before[split:]
    if benchmark in ('knucleotide', 'revcomp'):
        require(arguments == [], 'Unexpected original stdin Java workload argument')
        expected = prefix
    else:
        require(arguments in ([], [str(sizes[1])], ['{n}']), 'Original Java command has an unrelated workload argument')
        expected = prefix + ['{n}']
    require(after == expected, 'Saved retime Java command differs beyond exact workload-template normalization')
    return dict(original_command=list(before), measured_command=list(after), full_workload_n=sizes[1],
        status='exact-saved-retime-workload-template-lineage',
        unchanged_fields=['compile_command', 'execution_mode', 'source', 'sha256'],
        old_command_relabelled_as_measured=False)


def dotnet_backend(saved, runtime, aot):
    cmd, build = saved.get('command'), saved.get('build_command')
    backend_require(command(cmd) and command(build) and len(build) >= 3 and saved.get('build_status') == 'passed',
                    'Missing successful saved .NET build and measured command')
    backend_require(build[0] == runtime.get('path') and runtime.get('asset', {}).get('rid') == 'osx-arm64',
                    'Saved .NET build is not bound to the preserved ARM64 SDK host')
    if aot:
        backend_require(saved.get('execution_mode') == 'aot' and cmd[0] == saved.get('binary') and
                        len(cmd) >= 1 and cmd[0] != runtime['path'] and 'publish' == build[1] and
                        '-r' in build and build.index('-r') + 1 < len(build) and build[build.index('-r') + 1] == 'osx-arm64' and
                        '--self-contained' in build and build.index('--self-contained') + 1 < len(build) and
                        build[build.index('--self-contained') + 1] == 'true',
                        'Saved NativeAOT publish and actual native executable do not agree')
    else:
        backend_require(saved.get('execution_mode') == 'jit' and len(cmd) >= 2 and
                        cmd[:2] == [runtime['path'], saved.get('binary')] and
                        saved.get('binary', '').endswith('.dll') and build[1] == 'build',
                        'Saved JIT command is not the preserved dotnet host plus application DLL')
    return cmd


def is_native_arm64(data):
    # Read existing Mach-O bytes, without file/otool or a version/native rerun.
    return len(data) >= 8 and data[:4] == b'\xcf\xfa\xed\xfe' and struct.unpack('<I', data[4:8])[0] == 0x0100000c


class PinnedReads:
    def __init__(self, clbg, pins):
        self.clbg, self.pins = clbg, pins
        self.provenance, self.json_cache = {}, {}

    def read(self, path, expected=None, *, backend=False):
        path = Path(path)
        try:
            data = path.read_bytes()
        except FileNotFoundError:
            if backend:
                raise UnresolvedBackend('Preserved backend artifact unavailable: ' + str(path))
            raise
        actual = hashlib.sha256(data).hexdigest()
        require(expected is None or digest_value(expected) and actual == expected,
                'Preserved byte pin changed: ' + str(path))
        require(str(path) not in self.pins or self.pins[str(path)] == actual, 'Conflicting input pin')
        self.pins[str(path)] = actual
        if path.is_relative_to(self.clbg):
            self.provenance[str(path.relative_to(self.clbg))] = actual
        return data

    def report(self, relative, expected=None):
        path = self.clbg / relative
        data = self.read(path, expected)
        if relative not in self.json_cache:
            self.json_cache[relative] = json.loads(data)
        return self.json_cache[relative]

    def unchanged(self):
        for path, expected in self.pins.items():
            require(hashlib.sha256(Path(path).read_bytes()).hexdigest() == expected,
                    'Input changed during platform qualification: ' + path)


def display_sources(displayed, saved, data, reads):
    require(displayed['files'] and len(displayed['files']) == len(set(displayed['files'])), 'Invalid source inventory')
    sources = [data['files'][identity] for identity in displayed['files']]
    require(all(digest_value(source['sha256']) for source in sources), 'Invalid displayed source pins')
    for source in sources:
        reads.read(reads.clbg / source['path'], source['sha256'])
    saved_source = saved.get('source')
    if type(saved_source) is str:
        require(len(sources) == 1 and str(reads.clbg / sources[0]['path']) == saved_source and
                sources[0]['sha256'] == saved.get('source_sha256'), 'Displayed source differs from saved source')
    elif type(saved_source) is dict and type(saved_source.get('sha256')) is dict:
        pinned = {displayed['benchmark'] + '/' + path: digest for path, digest in saved_source['sha256'].items()}
        require({source['path']: source['sha256'] for source in sources} == pinned,
                'Displayed Java bundle differs from saved source map')
    else:
        digest = saved_source.get('sha256') if type(saved_source) is dict else None
        require(len(sources) == 1 and digest == sources[0]['sha256'] == saved.get('sha256'),
                'Displayed Java source differs from preserved scalar pin')
    return [source['sha256'] for source in sources]


def compare_saved_fields(first, second, names):
    for name in names:
        require(first.get(name) == second.get(name), 'Preserved build/report field differs: ' + name)


def qualify(clbg, data, extractions, expected, pins):
    selected = validate_inventory(data, expected)
    candidates = [row for extraction in extractions for row in extraction['records']]
    require(len(candidates) == len({row['key'] for row in candidates}) == 100 and
            {row['key'] for row in candidates} == set(selected), 'Candidate extraction inventory differs')
    require(all(row.get('verified') is False and row.get('matched_preserved_records') == 1 for row in candidates),
            'Expected unchanged unqualified one-record candidate extractions')
    candidate_by_key = {row['key']: row for row in candidates}
    reads = PinnedReads(clbg, pins)
    annotations, unresolved = {}, []
    for key in sorted(selected):
        displayed, candidate = selected[key], candidate_by_key[key]
        relative = displayed['report']
        require(candidate['report'] == relative and
                candidate['expected_report_sha256'] == data['provenance'][relative], 'Candidate report binding changed')
        report = reads.report(relative, data['provenance'][relative])
        index, saved = select(report, displayed)
        validate_samples(displayed, saved)
        source_digests = display_sources(displayed, saved, data, reads)
        require(candidate['display_samples'] == candidate['preserved_samples'] == saved['samples_seconds'] and
                candidate['stored_samples_agree'] is True and candidate['saved_command'] == saved['command'],
                'Candidate samples/command differ from actual saved measurement')
        require(candidate['source_files'] == [dict(path=data['files'][identity]['path'],
                sha256=data['files'][identity]['sha256']) for identity in displayed['files']],
                'Candidate display-source bundle changed')
        language = displayed['language']
        backend, execution = BACKENDS[language]
        require(candidate['proposed_backend'] == backend, 'Candidate backend proposal changed')
        runtime_relative, runtime_field, runtime_value = relative, field(index, 'execution_mode'), None
        runtime_encoding = 'literal-saved-string'
        extra = dict(saved_runtime_environment=saved.get('runtime_environment', {}))
        try:
            if language == 'java':
                require(relative in JAVA_REPORTS, 'Unexpected Java report')
                cmd = java_backend(saved)
                require(candidate['saved_compile_command'] == saved['compile_command'] and
                        candidate['saved_execution_mode'] == saved['execution_mode'], 'Candidate Java build/mode changed')
                runtime_value, label = saved['execution_mode'], 'OpenJDK JVM JIT'
                original = saved.get('original_report')
                if original:
                    original_pin = report['original_reports'][original]['sha256']
                    original_report = reads.report(original, original_pin)
                    _, original_row = select(original_report, displayed)
                    lineage = compare_retime_java_lineage(saved, original_row, displayed['benchmark'], report['protocol']['workloads'])
                    extra['preserved_original_report'] = dict(report=original, sha256=original_pin,
                        role='command/source lineage only; original runtime version is not attributed to new retime samples',
                        command_lineage=lineage)
                assets = saved.get('command_assets')
                if assets is not None:
                    require(type(assets) is list and assets and len(assets) == len(set(assets)), 'Invalid saved Java command-asset inventory')
                    require(all(path in report['asset_hashes'] for path in assets), 'Unpinned historical Java command asset')
                    for entry in java_classpath(cmd):
                        cp = Path(entry)
                        if cp.suffix == '.jar':
                            backend_require(entry in assets, 'Historical Java classpath JAR has no saved command-asset pin: ' + entry)
                        else:
                            saved_classes = {path for path in assets if Path(path).is_relative_to(cp) and path.endswith('.class')}
                            backend_require(saved_classes and {str(path) for path in cp.rglob('*.class')} == saved_classes,
                                            'Historical Java classpath artifact inventory is unavailable or changed: ' + entry)
                    for path in assets:
                        reads.read(path, report['asset_hashes'][path], backend=True)
                    extra['historical_command_asset_pins'] = {path: report['asset_hashes'][path] for path in assets}
                version = report.get('toolchains', {}).get('java')
                if type(version) is str and version.startswith('openjdk '):
                    extra['historical_runtime_version'] = version
                    runtime_field, runtime_value = '/toolchains/java', version
                    label += ' · ' + version.splitlines()[0]
                # Retime inherited commands/source metadata but produced new samples.
                # An old report version is provenance, not a version for the retime.
            elif language == 'erlang':
                built = reads.report(ERLANG_BUILD, data['provenance'][ERLANG_BUILD])
                details, runtimes = built['runtime_details'], built['runtimes']
                _, built_row = select(built, displayed)
                compare_saved_fields(saved, built_row, ('source', 'source_sha256', 'binary', 'binary_sha256', 'command', 'build_command', 'build_status'))
                require(report['runtimes'] == runtimes, 'Erlang timing/build runtime receipts differ')
                backend_require(details.get('jit') == 'jit' and details.get('schedulers') == 8 and
                                details.get('arch', '').startswith('aarch64-apple-darwin') and
                                command(saved['command']) and saved['command'][0] == runtimes['launcher']['path'] and
                                saved['command'][1:3] == ['+S', '8:8'], 'Saved Erlang receipt does not establish native BEAM JIT')
                backend_require(saved['build_status'] == 'passed' and saved['build_command'][0] == runtimes['compiler']['path'] and
                                '-pa' in saved['command'] and saved['command'][saved['command'].index('-pa') + 1] == str(Path(saved['binary']).parent),
                                'Erlang saved compilation/module lookup does not match the measured BEAM artifact')
                for runtime in runtimes.values():
                    runtime_bytes = reads.read(runtime['path'], runtime['sha256'], backend=True)
                    if runtime is runtimes['engine']:
                        backend_require(is_native_arm64(runtime_bytes), 'Preserved BEAM engine is not native ARM64 Mach-O')
                reads.read(saved['binary'], saved['binary_sha256'], backend=True)
                reads.read(saved['build_log'], backend=True)
                require(displayed['runtime_sha256'] == runtimes['engine']['sha256'], 'Displayed BEAM engine pin differs')
                runtime_relative, runtime_field = ERLANG_BUILD, '/runtime_details'
                runtime_value = json.dumps(details, sort_keys=True)
                runtime_encoding = 'json-serialized-preserved-value'
                label, cmd = 'Erlang/OTP ' + details['release'] + ' BEAM JIT · native ARM64', saved['command']
                extra.update(runtime_artifacts=runtimes, compiled_artifact=dict(path=saved['binary'], sha256=saved['binary_sha256']))
            else:
                summary = reads.report(DOTNET_SUMMARY, data['provenance'][DOTNET_SUMMARY])
                audit = reads.report(DOTNET_AUDIT, data['provenance'][DOTNET_AUDIT])
                runtime = summary['runtimes'][language]
                require(report['runtimes'][language] == audit['runtimes'][language] == runtime, 'Saved .NET host receipts disagree')
                _, audited = select(audit, displayed)
                compare_saved_fields(saved, audited, ('source', 'source_sha256', 'binary', 'binary_sha256', 'command', 'build_command', 'build_status', 'execution_mode', 'artifact_pins'))
                cmd = dotnet_backend(saved, runtime, language.endswith('-aot'))
                host = reads.read(runtime['path'], runtime['sha256'], backend=True)
                backend_require(is_native_arm64(host), 'Preserved .NET SDK host is not native ARM64 Mach-O')
                require(displayed['runtime_sha256'] == runtime['sha256'], 'Displayed .NET SDK host pin differs')
                artifact = reads.read(saved['binary'], saved['binary_sha256'], backend=True)
                for path, digest in saved['artifact_pins'].items():
                    reads.read(path, digest, backend=True)
                reads.read(saved['build_log'], backend=True)
                aot = language.endswith('-aot')
                if aot:
                    publish = reads.report(DOTNET_AOT)
                    _, published = select(publish, displayed)
                    compare_saved_fields(saved, published, ('source', 'source_sha256', 'binary', 'binary_sha256', 'command', 'build_command', 'build_status', 'execution_mode', 'artifact_pins'))
                    backend_require(is_native_arm64(artifact), 'Preserved NativeAOT scored binary is not native ARM64 Mach-O')
                    project = Path(saved['build_command'][2])
                    project_bytes = reads.read(project, backend=True)
                    backend_require(b'<PublishAot>true</PublishAot>' in project_bytes,
                                    'Preserved publish project lacks explicit NativeAOT property')
                    extra['observed_project'] = dict(path=str(project), sha256=reads.pins[str(project)],
                        role='additional present-day consistency check; no historical project digest was stored; '
                             'backend binding uses the saved publish receipt plus exact executed native binary pin')
                    runtime_relative, runtime_field = DOTNET_AOT, field(select(publish, displayed)[0], 'build_command')
                    runtime_value = json.dumps(published['build_command'])
                else:
                    backend_require(artifact[:2] == b'MZ', 'Preserved .NET JIT application is not a managed PE DLL')
                    runtime_relative, runtime_field = DOTNET_SUMMARY, '/runtimes/' + language
                    runtime_value = json.dumps(runtime, sort_keys=True)
                family = 'C#' if language.startswith('csharp') else 'F#'
                runtime_encoding = 'json-serialized-preserved-value'
                label = '.NET ' + family + (' NativeAOT · native ARM64' if aot else ' JIT')
                extra.update(sdk_build_provenance=dict(path=runtime['path'], sha256=runtime['sha256'], version=runtime['version'],
                    role='preserved SDK version; no runtime version inferred'),
                    compiled_artifact=dict(path=saved['binary'], sha256=saved['binary_sha256']),
                    saved_build_command=saved['build_command'])
            runtime_pin = reads.provenance.get(runtime_relative, data['provenance'].get(runtime_relative))
            require(digest_value(runtime_pin), 'Runtime receipt pin unavailable')
            annotations[key] = dict(key=backend, label=label, execution=execution, verified=True,
                evidence=dict(verified=True, report=relative, report_sha256=data['provenance'][relative],
                    source_sha256=source_digests, command=cmd,
                    runtime_receipt=dict(verified=True, report=runtime_relative, report_sha256=runtime_pin,
                        field=runtime_field, value=runtime_value, value_encoding=runtime_encoding), **extra))
        except UnresolvedBackend as error:
            annotations[key] = dict(key=backend, execution=execution, verified=False,
                evidence=dict(verified=False, report=relative, report_sha256=data['provenance'][relative],
                    source_sha256=source_digests, command=saved['command']), unresolved_reason=str(error))
            unresolved.append(dict(key=key, reason=str(error)))
    reads.unchanged()
    return dict(status='qualified-preserved-platform-metadata' if not unresolved else 'reconciled-with-unresolved-platform-metadata',
        expected_program_count=100, actual_program_count=len(annotations),
        qualified_program_count=sum(row['verified'] is True for row in annotations.values()),
        unresolved_programs=unresolved, annotations=annotations, provenance=reads.provenance,
        pins=pins, dataset_mutated=False, sources_mutated=False, timings_mutated=False,
        native_processes=0, runtime_version_reruns=0, chart_admission=False)
