"""Measure whether vector cursors and a bottom test share one spill."""

from pathlib import Path
import runpy

HERE = Path(__file__).resolve().parent
defs = runpy.run_path(str(HERE / 'evaluate.py'), run_name='latch_evaluate')
pin_control, read, save, study = (defs[key] for key in ('pin_control', 'read', 'save', 'study'))
ROLE = 'latch-both-cursors'


def main():
    pin_control()
    flags = study.ARMS['latch-test'] | {'MECH_ORDERED_VECTOR_WINDOW_CURSORS': '1'}
    study.emit(ROLE, flags)
    study.measure_counts(ROLE)
    captured = read(HERE / ROLE / 'instruction-counts/counts.json')
    compare = read(HERE / 'latch-test/instruction-counts/counts.json')
    delta = {}
    for name, row in captured['cases'].items():
        old = compare['cases'][name]['counts']
        counts = row['counts']
        delta[name] = {key: counts.get(key, 0) - old.get(key, 0) for key in
            ('instructions', 'branches', 'stack_loads', 'stack_stores', 'payload_loads')}
    save(HERE / ROLE / 'changes-from-latch.json', delta)
    print(delta, flush=True)
    if delta['prefix-natural-8192']['instructions'] >= 0:
        return
    layout = study.profiles(ROLE)
    placed = ROLE + '-layout'
    study.driver.emit(placed, study.driver.HORIZONTAL | study.driver.REVERSE | flags,
                      study.COMPILER, profile_path=layout)
    assert '_mech_compact_closed_checked' not in read(HERE / placed / 'receipt.json')['changed_symbols']


if __name__ == '__main__':
    main()
