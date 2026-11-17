"""Compare the failure-return omission against the same compiler with omission off."""
from importlib import import_module
from itertools import permutations
from pathlib import Path
from statistics import median
import sys

from build import FLAG, HERE, sha, write

sys.path.insert(0, str(HERE.parent))
driver = import_module('measure-deployment-builds')
SOURCE = HERE.parent / 'statement-compiler/matched/plan.json'
ROLES = ('mech-current', 'mech-materialize', 'mech-skip-unused')


def main():
    source_plan = driver.read(SOURCE)
    acq = driver.read(HERE / 'capture/result.json')
    published = source_plan['variants']['mech-statement-replay']
    candidate = acq['actors']['skip-unused']
    assert acq['parent_plan_sha256'] == sha(SOURCE)
    assert not acq['metadata_difference_fields'] and candidate['discarded_returns'] == 5236
    assert all(row['artifacts'] == published['expected_artifacts'] for row in acq['actors'].values())
    source = published['source_command']
    assert sha(Path(source[0])) == published['compiler_sha256']
    experiment = HERE / 'candidate/compile_profile'
    assert sha(experiment) == acq['compiler_sha256'] != published['compiler_sha256']
    assert sha(Path(source[1])) == source_plan['source_sha256']['mech']
    host = published['link_command']
    obj, = [arg for arg in host if arg.endswith('/program.o')]
    for name, expected in published['fixed_link_inputs'].items():
        assert sha(Path(name)) == expected
    options = published['native_flags']
    pins = published['expected_artifacts']
    arms = dict(zip(ROLES, ((source, options), ([str(experiment), *source[1:]], options),
                            ([str(experiment), *source[1:]], options | {FLAG: '1'})), strict=True))
    output = HERE / 'matched'
    output.mkdir()
    order = list(permutations(ROLES))
    write(output / 'plan.json', {'roles': ROLES, 'warmups': ROLES, 'orders': order, 'actors': arms,
                                 'reference_sha256': sha(SOURCE), 'fixed_link_inputs': published['fixed_link_inputs'],
                                 'artifact_sha256': pins, 'link': host})

    def sample(actor, label):
        command, flags = arms[actor]
        result = driver.compile_and_link(actor, output / label / actor, command, host, obj, flags, pins)
        assert result['generated_artifacts'] == pins
        return result

    warmups = {actor: sample(actor, 'warmup') for actor in ROLES}
    trials = [{actor: sample(actor, f'order-{i}') for actor in run} for i, run in enumerate(order, 1)]
    for row in [warmups, *trials]:
        assert len({actor['stripped_executable']['sha256'] for actor in row.values()}) == 1

    def get(row, actor, stage):
        if stage == 'native':
            val, = [p['seconds'] for p in row[actor]['compiler_phases'] if p['phase'] == 'native']
            return val
        return row[actor]['seconds'][stage]

    stages = ('native', 'source_through_link', 'source_through_strip')
    result = {'plan_sha256': sha(output / 'plan.json'), 'warmups': warmups, 'trials': trials,
              'medians': {stage: {actor: median(get(row, actor, stage) for row in trials) for actor in ROLES}
                          for stage in stages},
              'ratios_to_materialize': {
                  stage: {actor: median(get(row, actor, stage) / get(row, ROLES[1], stage) for row in trials)
                          for actor in (ROLES[0], ROLES[2])} for stage in stages}}
    write(output / 'comparison.json', result)
    print(result['medians'], result['ratios_to_materialize'], flush=True)


if __name__ == '__main__':
    main()
