"""Evaluate the two independent ordered vector cursor modes."""

from importlib import import_module
import sys

from stage import BASE, HERE, SWITCH
from build import CONTROL_PLAN, OUT, COMPILER, read, save, sha
from narrow import MODES, SOURCE

ROADMAP = HERE.parent
CONTROL = 'vec-unlaid-control'
WINDOWS = 'vec-window-cursors'
DEFERRED = 'vec-deferred-lanes'

# Reuse the pinned emit, layout and timing drivers to keep the acquisition
# boundary, data schema and fixture checks identical to the preceding study.
sys.path.insert(0, str(ROADMAP / 'ordered-scorer'))
study = import_module('study')
study.BASE, study.HERE, study.OUT, study.COMPILER = BASE, HERE, OUT, COMPILER
for module in (study.driver, study.rebase, study.scheduler, study.count, study.fit):
    module.HERE = HERE
study.driver.PIN = BASE
study.driver.PLAN = HERE / 'control-plan.json'
study.ARMS = {CONTROL: {}, WINDOWS: {SWITCH: '1'}, DEFERRED: {SWITCH: 'defer-lanes'},
              **{role: {SWITCH: mode} for role, mode in MODES.items()}}


def pin_control():
    out = study.driver.PLAN
    body = read(CONTROL_PLAN)
    body['actors']['mech-verified-flowgraph'] = body['actors']['mech-ascii-miss']
    if out.exists():
        assert read(out) == body
    else:
        save(out, body)
    assert sha(COMPILER) == read(OUT / 'result.json')['compiler_sha256']


def main():
    action, *roles = sys.argv[1:]
    pin_control()
    if action in ('emit', 'layout', 'count'):
        allowed = set(study.ARMS)
        assert set(roles) <= allowed, roles
        for role in roles:
            if role in MODES:
                study.OUT = SOURCE / 'probe-compiler'
                study.COMPILER = study.OUT / 'compile_profile'
            else:
                study.OUT, study.COMPILER = OUT, COMPILER
            if action == 'layout':
                study.measure_counts(role)
                path = study.profiles(role)
                study.emit(role + '-layout', study.ARMS[role], path)
            elif action == 'count':
                study.measure_counts(role)
            else:
                study.emit(role, study.ARMS[role])
        if action == 'emit' and CONTROL in roles:
            prior = ROADMAP / 'ordered-scorer/ord-control/receipt.json'
            known = read(prior)['layout']['object_sha256']
            actual = read(HERE / CONTROL / 'receipt.json')['layout']['object_sha256']
            assert actual == known, (known, actual)
    else:
        modules = {'focus': 'timing', 'full': 'timing', 'check': 'qualify', 'matched': 'matched'}
        role, = roles
        sys.argv = [sys.argv[0], role, action] if action in ('full', 'focus') else [sys.argv[0], role]
        import_module(modules[action]).main()


if __name__ == '__main__':
    main()
