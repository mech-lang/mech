"""Acquire the ordered reduction latch ablation with the 62-win floor."""

from importlib import import_module
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'ordered-vector-cursors'))
from build import read, save, sha
from evaluate import pin_control, study
from stage import BASE, Q

study.HERE = HERE
study.OUT = HERE / 'probe-compiler'
study.COMPILER = study.OUT / 'compile_profile'
study.ARMS = read(HERE / 'source.json')['modes']
for module in (study.driver, study.rebase, study.scheduler, study.count, study.fit):
    module.HERE = HERE


def time_runtime(role, kind):
    timing = import_module('timing')
    timing.time.policy_file = timing.time.packet / 'qualification-policy-v3.json'
    original_setup = timing.setup

    def setup(label, run_kind):
        plan = original_setup(label, run_kind)
        plan['policy_version'] = 3
        plan['objective'] = 'Restore the natural 8 KiB win and preserve the peer win set.'
        plan['driver'] = timing.time.ref(Path(__file__).resolve())
        folder, _ = timing.paths(label, run_kind)
        save(folder / 'plan.json', plan)
        (folder / 'plan.sha256').write_text(sha(folder / 'plan.json') + '\n')
        return plan

    timing.setup = setup
    sys.argv = [sys.argv[0], role, kind]
    timing.main()
    screen_file = HERE / role / kind / 'screen.json'
    screen = read(screen_file)
    replay = read(HERE / role / kind / f'{kind}-replay.json')
    for judged, outcome in zip(screen['runs'], replay['runs'], strict=True):
        raw = Q / outcome['id'] / 'summary.json'
        observed = {row['case']: row for row in read(raw)}
        judged['cases']['prefix-natural-4096'] = {
            role: observed['prefix-natural-4096']['paired_x_libdeflate'][role]
            for role in ('mech-control', 'mech-experiment')
        }
        if kind == 'full':
            judged['wins_mech_experiment'] = {peer: wins['mech-experiment']
                                            for peer, wins in judged['wins'].items()}
    save(screen_file, screen)
    print('Win targets', [{'order': row['order'], '4KiB': row['cases']['prefix-natural-4096'],
                           '8KiB': row['cases']['prefix-natural-8192']} for row in screen['runs']], flush=True)


def main():
    action, *roles = sys.argv[1:]
    pin_control()
    if action in ('emit', 'layout'):
        assert set(roles) <= set(study.ARMS)
        for role in roles:
            if action == 'emit':
                study.emit(role, study.ARMS[role])
            else:
                study.measure_counts(role)
                study.emit(role + '-layout', study.ARMS[role], study.profiles(role))
        if action == 'emit' and 'latch-control' in roles:
            old = read(HERE.parent / 'ordered-vector-cursors/vec-unlaid-control/receipt.json')
            new = read(HERE / 'latch-control/receipt.json')
            assert old['layout']['object_sha256'] == new['layout']['object_sha256']
    else:
        role, = roles
        if action in ('focus', 'full'):
            time_runtime(role, action)
        else:
            assert action == 'check'
            sys.argv = [sys.argv[0], role]
            import_module('qualify').main()


if __name__ == '__main__':
    main()
