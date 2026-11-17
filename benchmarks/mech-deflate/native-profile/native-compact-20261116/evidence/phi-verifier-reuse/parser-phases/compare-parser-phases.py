"""Compare the current reported tokenization and parsing intervals on the M1."""
from importlib import import_module
from pathlib import Path
import re
from statistics import median
import sys

here = Path(__file__).resolve().parent
roadmap = here.parent
sys.path.insert(0, str(roadmap))
driver = import_module('measure-deployment-builds')


def main():
    root = here / 'parser-phases'
    root.mkdir()
    baseline = roadmap / 'phi-verifier'
    build = driver.read(baseline / 'matched/plan.json')
    warmup = driver.read(baseline / 'matched/warmup/roc/result.json')
    mech = build['variants']['mech-phi-verifier']
    roc = build['variants']['roc']
    assert all(driver.sha(row['source_command'][0]) == row['compiler_sha256'] for row in (mech, roc))
    host = driver.read(driver.SMALL / 'receipt.json')['commands']['link']
    origin_mech, = [arg for arg in host if arg.endswith('/program.o')]
    origin_roc, = [arg.removeprefix('--output=') for arg in roc['source_command'] if arg.startswith('--output=')]
    paths = {act: tuple(arg for arg in row['source_command'] if arg.endswith(('.roc', '.mec')))
             for act, row in [('mech', mech), ('roc', roc)]}
    assert all(len(path) == 1 for path in paths.values())
    assert '--jobs=1' in roc['source_command'] and '--no-cache' in roc['source_command']
    assert warmup['encoder_sha256'] == roc['expected_artifacts']['encoder']
    bindings = {
        'mech-phi-verifier': (mech['source_command'], mech['link_command'], origin_mech,
                              mech['native_flags'], mech['expected_artifacts']),
        'roc': ([*roc['source_command'][:2], '--timings', *roc['source_command'][2:]],
                roc['link_command'], origin_roc, roc['native_flags'], roc['expected_artifacts']),
    }
    orders = [('mech-phi-verifier', 'roc'), ('roc', 'mech-phi-verifier'),
              ('roc', 'mech-phi-verifier'), ('mech-phi-verifier', 'roc')]
    plan = {'orders': orders, 'source_sha256': {actor: driver.sha(p[0]) for actor, p in paths.items()},
            'bindings': bindings,
            'scope': {'mech': 'parse_canonical_document through strict clean check and syntax cast',
                      'roc': 'Roc --timings Parsing (summed), single compilation job and disabled cache'},
            'separate_from_source_through_link_timing': True}
    driver.save(root / 'plan.json', plan)

    def sample(actor, key):
        folder = root / key / actor
        record = driver.compile_and_link(actor, folder, *bindings[actor])
        if actor.startswith('mech-'):
            parsed, = [phase['seconds'] for phase in record['compiler_phases'] if phase['phase'] == 'parse']
            record['parse_reported_seconds'] = parsed
        else:
            display = (folder / 'source.log').read_text()
            parsed = re.search(r'^\s*Parsing \(summed\)\s+([\d_.]+)(ns|µs|ms|s)(?:,|\s|$)', display, re.M)
            assert parsed, display[:2000]
            value, unit = parsed.groups()
            record['parse_reported_seconds'] = float(value.replace('_', '')) * {'ns': 1e-9, 'µs': 1e-6, 'ms': 1e-3, 's': 1}[unit]
            record['displayed_parser_phase'] = parsed[0].strip()
        return record

    warmup = {actor: sample(actor, 'warmup') for actor in bindings}
    pairs = [{actor: sample(actor, f'order-{i}') for actor in order} for i, order in enumerate(orders, 1)]
    result = {actor: median(row[actor]['parse_reported_seconds'] for row in pairs) for actor in bindings}
    driver.save(root / 'summary.json', {'reported_parse_medians_s': result,
                                       'mech_over_roc_reported_intervals': result['mech-phi-verifier']/result['roc'],
                                       'warmup': warmup, 'trials': pairs})
    print(result, flush=True)


if __name__ == '__main__':
    main()
