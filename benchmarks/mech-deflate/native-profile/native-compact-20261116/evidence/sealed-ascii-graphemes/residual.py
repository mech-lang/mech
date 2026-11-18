"""Capture post-change parser stacks separately from uninstrumented timing."""
from pathlib import Path
from statistics import median
import subprocess
import sys

from stage import HERE, SAMPLE, CHECKOUT, PIN, read, save, sha

sys.path.insert(0, str(SAMPLE))
from sample_hotspots import parse_callgraph

OUT = HERE / 'residual'


def main():
    OUT.mkdir()
    standalone = read(HERE / 'measured/comparison.json')['exact_parse_report']
    parser = HERE / 'candidate/parser'
    candidate = read(HERE / 'candidate/build.json')
    assert sha(parser) == candidate['binary_sha256']
    lib = HERE / 'candidate/libmech_syntax.rlib'
    assert sha(lib) == candidate['library_sha256']
    original = read(SAMPLE / 'sampler-build.json')
    control = SAMPLE / 'sampler-probe'
    assert sha(control) == original['binary']
    instrument = OUT / 'sampler-probe'
    command = [f'mech_syntax={lib}' if arg.startswith('mech_syntax=') else
               str(instrument) if arg == str(control) else arg for arg in original['command']]
    with (OUT / 'link.log').open('wb') as log:
        subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
    source = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec'
    assert sha(source) == standalone['source_sha256']
    save(OUT / 'plan.json', {'baseline_source': PIN, 'candidate_scanner': read(HERE / 'candidate/source.json'),
         'source_sha256': sha(source), 'compiler': command, 'sampler_tool': '/usr/bin/sample',
         'stack_capture_interval_ms': 1, 'captures': 4, 'probe_sha256': sha(instrument),
         'exact_output': standalone})
    records = []
    for index in range(1, 5):
        prefix = OUT / f'capture-{index}'
        result = prefix.with_suffix('.json')
        stack = prefix.with_suffix('.txt')
        args = [str(instrument), str(source), str(result)]
        with prefix.with_suffix('.log').open('wb') as probe_log:
            with subprocess.Popen(args, stdout=probe_log, stderr=probe_log) as child:
                flags = ['/usr/bin/sample', str(child.pid), '2', '1', '-mayDie', '-file', str(stack)]
                with (OUT / f'sample-{index}.log').open('wb') as sample_log:
                    subprocess.run(flags, stdout=sample_log, stderr=sample_log, check=True, timeout=15)
                assert child.wait(timeout=20) == 0
        report = read(result)
        report.pop('parse_seconds')
        assert report == standalone
        total, family, leaves, local = parse_callgraph(stack.read_text().splitlines())
        assert total >= 550, (stack, total)
        records.append({'capture': index, 'probe_command': args, 'sample_command': flags,
                        'raw_sha256': sha(stack), 'parser_samples': total, 'family_samples': family,
                        'top_leaves': leaves.most_common(20),
                        'top_leaves_by_family': {name: items.most_common(12) for name, items in local.items()}})
        print(index, total, family.most_common(6), flush=True)
    families = {}
    for name in sorted({name for row in records for name in row['family_samples']}):
        fractions = [row['family_samples'].get(name, 0) / row['parser_samples'] for row in records]
        families[name] = {'median_fraction': median(fractions), 'per_capture': fractions}
    save(OUT / 'summary.json', {'plan_sha256': sha(OUT / 'plan.json'), 'captures': records,
                               'matched_control_families': read(SAMPLE / 'samples/report.json')['families'],
                               'families': dict(sorted(families.items(),
                                                       key=lambda row: -row[1]['median_fraction']))})


if __name__ == '__main__':
    main()
