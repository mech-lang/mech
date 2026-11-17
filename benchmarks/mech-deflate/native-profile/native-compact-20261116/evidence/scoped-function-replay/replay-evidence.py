"""Check the archived compiler and standalone parser comparisons."""
from collections import Counter
import gzip
from hashlib import sha256
import json
from pathlib import Path
from statistics import median
import sys


def read(path):
    data = path.read_bytes()
    return json.loads(gzip.decompress(data) if path.suffix == '.gz' else data)


def replay(scope):
    plan_path = scope / 'source-through-link-plan.json'
    plan = read(plan_path)
    builds = read(scope / 'source-through-link-observations.json.gz')
    parser = read(scope / 'parser-observations.json.gz')
    stats = read(scope / 'summary.json')
    checks = read(scope / 'worker.json')
    assert sha256(plan_path.read_bytes()).hexdigest() == builds['plan_sha256']
    assert len(builds['trials']) == len(plan['order']) == 8
    control, candidate, roc, lib = ('mech-control', 'mech-function-replay', 'roc', 'libdeflate')
    roles = (control, candidate, roc, lib)
    assert all(Counter(column) == Counter({actor: 2 for actor in roles}) for column in zip(*plan['order']))
    assert plan['variants'][control]['compiler_sha256'] != plan['variants'][candidate]['compiler_sha256']
    assert plan['variants'][control]['expected_artifacts'] == plan['variants'][candidate]['expected_artifacts']
    assert checks['output_sha256']['program.o'] == plan['variants'][candidate]['expected_artifacts']['encoder']
    assert checks['output_sha256']['metadata.json'] == plan['variants'][candidate]['expected_artifacts']['metadata']
    assert checks['worker_sha256'] == stats['worker_sha256']
    assert checks['exact_linked_calls'] == 162
    for order, trial in zip(plan['order'], builds['trials'], strict=True):
        assert list(trial) == order
        assert trial[control]['stripped_executable'] == trial[candidate]['stripped_executable']
        for actor, row in trial.items():
            assert row['generated_artifacts'] == plan['variants'][actor]['expected_artifacts']
            assert row['stripped_executable'] == builds['warmups'][actor]['stripped_executable']
    key = 'source_through_link'
    ratios = {peer: median(row[candidate]['seconds'][key] / row[peer]['seconds'][key] for row in builds['trials'])
              for peer in (control, roc, lib)}
    medians = {actor: {stage: median(trial[actor]['seconds'][stage] for trial in builds['trials'])
                       for stage in ('source', 'link', 'strip', key, 'source_through_strip')}
               for actor in roles}
    parser_roles = ('control', 'nested-disabled', 'candidate', 'capture-only')
    assert len(parser['rows']) == 8
    med_parser = {actor: median(sample[actor]['observable']['parse_seconds'] for sample in parser['rows'])
                  for actor in parser_roles}
    assert med_parser == parser['median_s'] == stats['standalone_parser']['median_s']
    assert medians == builds['medians_seconds'] == stats['source_through_deployment']
    assert ratios == builds['candidate_paired_build_x'] == stats['source_through_link_paired_ratio']
    print(json.dumps({'source_through_link_seconds': {actor: medians[actor][key] for actor in roles},
                      'paired_build_ratios': ratios, 'standalone_parser_seconds': med_parser,
                      'exact_linked_calls': checks['exact_linked_calls']}, indent=2))


if __name__ == '__main__':
    replay(Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent)
