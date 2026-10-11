#!/usr/bin/env python3
"""Complete the published-fastest scripting cohort at the full CLBG workloads."""
import argparse
import copy
import importlib.util
import json
import platform
import statistics
import subprocess
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
SPEC = importlib.util.spec_from_file_location('compact_measurement', CLBG / 'compact/run.py')
common = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(common)
LANGUAGES = ('python', 'ruby', 'perl', 'lua', 'luajit', 'php', 'javascript')
PREVIOUS = CLBG / 'retime/results/apple-m3-ac-normal-20261008.json'
MANIFEST = CLBG / 'compact/manifest.json'
POWER = {'source': 'AC Power', 'low_power_mode': 0}


def published_seconds(row):
    text = row['published_time'].replace('\xa0', ' ')
    return float(text.split()[0].replace(',', '')) * (60 if 'min' in text else 1)


def save(path, report):
    report['updated_at'] = datetime.now(timezone.utc).isoformat()
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(report, indent=2) + '\n')
    temporary.replace(path)


def loc_source(pin):
    return {key: pin['source'][key] for key in
            ('physical_lines', 'nonblank_noncomment_lines')}


def candidate(pin, language):
    source = CLBG / 'compact' / pin['path']
    if common.sha(source) != pin['sha256']:
        raise ValueError('Source changed: ' + pin['id'])
    flags = []
    if language == 'ruby':
        flags = ['--yjit', '-W0', '-r', str(HERE / 'linux_cpuinfo.rb')]
    elif language == 'perl':
        flags = ['-I' + str(HERE), '-MCLBGLinuxCPUInfo']
    elif language == 'php':
        flags = ['-n', '-d', 'memory_limit=-1', '-d', 'opcache.enable_cli=1',
                 '-d', 'opcache.jit_buffer_size=64M', '-d', 'opcache.jit=tracing',
                 '-d', 'short_open_tag=1']
    launcher = [str(HERE / 'python_fork.py')] if language == 'python' else []
    command = [common.RUNTIMES[language], *flags, *launcher, str(source)]
    if pin['benchmark'] not in ('knucleotide', 'revcomp'):
        command.append('{n}')
    algorithm = common.classify(pin)
    if pin['benchmark'] == 'binarytrees' and language == 'php' and 'return [null, null]' in source.read_text():
        algorithm = dict(strict=False, reason='PHP leaves use constant array literals, which may share copy-on-write backing storage. Allocation parity with ordinary-node leaves is not assumed; recursive construction and full output are validated.')
    return dict(id=pin['id'] + ('-luajit' if language == 'luajit' else ''),
                upstream_id=pin['id'], benchmark=pin['benchmark'], language=language,
                variant=pin['variant'], url=pin['url'], path=str(source),
                sha256=pin['sha256'], source=loc_source(pin), command=command,
                published_time=pin['published_time'], samples_seconds=[],
                sample_validation=[], runtime_environment={
                    'CLBG_CPUINFO_FILE': str(HERE / 'cpuinfo.txt')},
                algorithm=algorithm, endpoint_scope='fastest measured in selected listed cohort')


def prepare(args):
    pins = json.loads(MANIFEST.read_text())
    previous = json.loads(PREVIOUS.read_text())
    assert previous['status'] == 'complete'
    (HERE / 'cpuinfo.txt').write_text(''.join(f'processor\t: {i}\n\n' for i in range(previous['hardware']['logical_cpus'])))
    versions = {}
    for language, executable in common.RUNTIMES.items():
        flags = ['-v'] if language in ('perl', 'lua', 'luajit') else ['--version']
        process = subprocess.run([executable, *flags], capture_output=True, text=True, check=True)
        if common.sha(executable) != previous['toolchains'][language]['sha256']:
            raise ValueError('Runtime differs from reusable AC cohort: ' + language)
        versions[language] = dict(path=executable, sha256=common.sha(executable),
                                  version=(process.stdout + process.stderr).strip())
    pin_by_id = {p['id']: p for p in pins['programs']}
    programs = []
    for old in previous['programs']:
        if old['language'] not in LANGUAGES:
            continue
        pin = pin_by_id[old.get('upstream_id', old['id'].removesuffix('-luajit'))]
        row = candidate(pin, old['language'])
        if len(old['samples_seconds']) < args.samples:
            raise ValueError('Insufficient preserved samples: ' + old['id'])
        if previous['benchmark_power_profiles'][old['benchmark']] != POWER:
            raise ValueError('Preserved measurement has wrong power profile')
        # These are the exact preserved invocations, not invocations using the new launcher.
        row.update(command=copy.deepcopy(old['command']),
                   samples_seconds=copy.deepcopy(old['samples_seconds']),
                   sample_validation=copy.deepcopy(old['sample_validation']),
                   median_seconds=old['median_seconds'],
                   runtime_environment=copy.deepcopy(old.get('runtime_environment', {})),
                   algorithm=copy.deepcopy(old.get('algorithm', row['algorithm'])),
                   reused_measurement=dict(report=str(PREVIOUS.relative_to(CLBG)),
                       sha256=common.sha(PREVIOUS), id=old['id'], power_profile=POWER))
        assert all(v.get('reference_match') for v in row['sample_validation'])
        programs.append(row)
    report = dict(schema_version=1, created_at=datetime.now(timezone.utc).isoformat(),
                  status='prepared', hardware=previous['hardware'], toolchains=versions,
                  inputs=dict(manifest_sha256=common.sha(MANIFEST),
                              previous_report_sha256=common.sha(PREVIOUS),
                              python_launcher_sha256=common.sha(HERE / 'python_fork.py'),
                              portability_assets={p: common.sha(HERE / p) for p in
                                  ('linux_cpuinfo.rb', 'CLBGLinuxCPUInfo.pm', 'cpuinfo.txt')}),
                  protocol=dict(workloads=common.WORK, samples=args.samples, seed=args.seed,
                      timeout_seconds=args.timeout, power_profile=POWER,
                      selection='Previously measured compact entries plus fastest published time and all ties in the October 8 pinned tables. Rounded minute times retain every tie. LuaJIT tests every unchanged listed Lua source for local compatibility and speed.',
                      fastest_claim='Fastest measured among this selected cohort, not an exhaustive local sweep of every scripting source. Published-fastest variants are explicitly covered or excluded with reasons.',
                      source_metric='Nonblank, noncomment physical LOC of the complete unchanged listed program. General runtime libraries excluded.',
                      timed_region='Sequential fresh process through exit, including interpreter/JIT startup, workers and complete stdout to a regular file. Output validation is outside timing.',
                      python_runtime='New parallel Python invocations explicitly use the fork multiprocessing start policy assumed by the upstream Linux programs. The general launcher is included in process timing.',
                      linux_cpu_probe='New Ruby and Perl invocations redirect only /proc/cpuinfo reads to a pinned eight-logical-CPU fixture on macOS. These general portability preloads are timed; complete benchmark sources are unchanged.',
                      normalization='Raw process medians on the same Apple M3 with AC power and Low Power Mode off. Earlier compact medians are preserved. Each new benchmark batch includes three fresh samples of a preserved native control to audit drift; no rescaling.',
                      environment={k: common.ENV[k] for k in ('RAYON_NUM_THREADS', 'JULIA_NUM_THREADS', 'OMP_NUM_THREADS', 'OMP_DYNAMIC', 'CHPL_RT_NUM_THREADS_PER_LOCALE', 'GOMAXPROCS', 'OPENBLAS_NUM_THREADS', 'PYTHONHASHSEED')},
                      compiler_guard='Before samples wait for no competing compiler >=5% CPU. Inspect compiler executable names and power at both boundaries and every 10 seconds; discard affected samples.'),
                  programs=programs, controls=[], excluded=[], selection_audit=[],
                  completed_benchmarks=[], discarded_samples=[])
    selected = {(r['benchmark'], r['id']) for r in programs}
    for benchmark in common.WORK:
        for language in LANGUAGES:
            listed = sorted([p for p in pins['programs'] if p['benchmark'] == benchmark
                        and p['language'] == ('lua' if language == 'luajit' else language)],
                        key=published_seconds)
            fastest_time = published_seconds(listed[0])
            fastest = [p for p in listed if published_seconds(p) == fastest_time]
            wanted = listed if language == 'luajit' else fastest
            audit = dict(benchmark=benchmark, language=language, listed_candidates=len(listed),
                         published_fastest_ids=[p['id'] for p in fastest],
                         selected_ids=[], excluded_ids=[])
            report['selection_audit'].append(audit)
            # If a published-fastest source is incompatible, test the next tier too.
            if language != 'luajit':
                wanted = listed
            supported_fastest = False
            accepted_time = None
            for pin in wanted:
                if language != 'luajit' and supported_fastest and published_seconds(pin) != accepted_time:
                    break
                row = candidate(pin, language)
                audit['selected_ids'].append(row['id'])
                if (benchmark, row['id']) in selected:
                    supported_fastest = True
                    accepted_time = published_seconds(pin)
                    continue
                common.wait_for_environment(POWER)
                try:
                    result = common.execute(row, False, 60)
                    row['correctness_small'] = result['validation']
                    row['small_validation_seconds'] = result['seconds']
                    programs.append(row)
                    selected.add((benchmark, row['id']))
                    supported_fastest = True
                    accepted_time = published_seconds(pin)
                    print('SMALL PASS', row['id'], f"{result['seconds']:.3f}s", flush=True)
                except Exception as error:
                    report['excluded'].append(dict(id=row['id'], benchmark=benchmark,
                        language=language, stage='small validation', reason=str(error),
                        source_sha256=pin['sha256'], loc=row['source']['nonblank_noncomment_lines']))
                    audit['excluded_ids'].append(row['id'])
                    print('EXCLUDED', row['id'], str(error), flush=True)
        old_controls = [r for r in previous['programs'] if r['benchmark'] == benchmark
                        and r['language'] in ('c', 'cpp', 'rust')]
        old = min(old_controls, key=lambda r: r['median_seconds'])
        control = dict(id=old['id'], benchmark=benchmark, language=old['language'],
                       command=copy.deepcopy(old['command']),
                       runtime_environment=copy.deepcopy(old.get('runtime_environment', {})),
                       samples_seconds=[], sample_validation=[],
                       preserved_median_seconds=old['median_seconds'],
                       executable_sha256=common.sha(old['command'][0]))
        report['controls'].append(control)
    save(args.output, report)
    print('PREPARED', len(programs), 'entries;', sum(not r.get('reused_measurement') for r in programs), 'new full measurements', flush=True)


def measure(args):
    report = json.loads(args.output.read_text())
    assert report['inputs']['manifest_sha256'] == common.sha(MANIFEST)
    assert report['inputs']['previous_report_sha256'] == common.sha(PREVIOUS)
    assert report['inputs']['python_launcher_sha256'] == common.sha(HERE / 'python_fork.py')
    for asset, digest in report['inputs']['portability_assets'].items():
        assert common.sha(HERE / asset) == digest
    for runtime in report['toolchains'].values():
        assert common.sha(runtime['path']) == runtime['sha256']
    for row in report['programs']:
        assert common.sha(row['path']) == row['sha256']
    args.required_power = POWER
    for benchmark in args.benchmarks.split(','):
        if benchmark in report['completed_benchmarks']:
            continue
        rows = [r for r in report['programs'] if r['benchmark'] == benchmark
                and not r.get('reused_measurement') and not r.get('excluded_full')]
        control = next(r for r in report['controls'] if r['benchmark'] == benchmark)
        assert common.sha(control['command'][0]) == control['executable_sha256']
        report['status'] = 'running'
        save(args.output, report)
        print('BEGIN', benchmark, len(rows), 'new entries', flush=True)
        # Every timed sample validates the full output. The small run serves as warmup;
        # avoid a redundant fourth full execution of minute-long interpreters.
        def pre_sample(row):
            report['active_sample'] = dict(benchmark=benchmark, id=row['id'])
            save(args.output, report)
            print('RUN', row['id'], len(row['samples_seconds']) + 1, flush=True)
        args.pre_sample = pre_sample
        while True:
            try:
                orders = common.measure_batch(benchmark, rows, control, args, report,
                                              lambda: save(args.output, report))
                break
            except (RuntimeError, ValueError) as error:
                active = report['active_sample']['id']
                if active == control['id']:
                    raise
                failed = next(r for r in rows if r['id'] == active)
                failed['excluded_full'] = True
                report['excluded'].append(dict(id=active, benchmark=benchmark,
                    language=failed['language'], stage='full validation', reason=str(error),
                    source_sha256=failed['sha256'], loc=failed['source']['nonblank_noncomment_lines']))
                rows.remove(failed)
                save(args.output, report)
                print('EXCLUDED FULL', active, str(error), flush=True)
        for row in [control, *rows]:
            row['median_seconds'] = statistics.median(row['samples_seconds'])
            row['min_seconds'] = min(row['samples_seconds'])
            row['max_seconds'] = max(row['samples_seconds'])
        control['drift_ratio'] = control['median_seconds'] / control['preserved_median_seconds']
        report.setdefault('sample_order', {})[benchmark] = orders
        report['completed_benchmarks'].append(benchmark)
        save(args.output, report)
        print('DONE', benchmark, 'control drift', f"{control['drift_ratio']:.3f}x", flush=True)
    report['status'] = 'complete' if len(report['completed_benchmarks']) == len(common.WORK) else 'partial'
    save(args.output, report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare', action='store_true')
    parser.add_argument('--output', type=Path, default=HERE / 'results/apple-m3-ac-normal-20261009.json')
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--timeout', type=int, default=1800)
    parser.add_argument('--seed', type=int, default=20261009)
    parser.add_argument('--benchmarks', default='revcomp,knucleotide,fasta,binarytrees,spectralnorm,nbody,mandelbrot,fannkuchredux')
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if args.samples < 3:
        parser.error('At least three samples required')
    if args.prepare:
        if args.output.exists():
            parser.error('Refusing to replace a report; resume without --prepare')
        prepare(args)
    else:
        measure(args)


if __name__ == '__main__':
    main()
