"""Retain the exact qualified checked root while changing the placed scorer."""

from pathlib import Path
import runpy

HERE = Path(__file__).resolve().parent
definitions = runpy.run_path(str(HERE / 'evaluate.py'), run_name='latch_evaluate')
pin_control, read, save, study = (definitions[key] for key in ('pin_control', 'read', 'save', 'study'))


def main():
    pin_control()
    label = 'latch-kept-root-layout'
    flags = study.driver.HORIZONTAL | study.driver.REVERSE | study.ARMS['latch-test']
    layout = HERE / 'latch-test-scorer-profile.json'
    study.driver.emit(label, flags, study.COMPILER, profile_path=layout)
    current = read(HERE / label / 'receipt.json')
    original = read(HERE / 'latch-test-layout/receipt.json')
    assert '_mech_compact_closed_checked' not in current['changed_symbols']
    name = study.driver.SCORER
    before, after = [row['layout']['symbols'][name] for row in (original, current)]
    assert (before['sha256'], before['extent'], before['relocations']) == (
        after['sha256'], after['extent'], after['relocations'])
    save(HERE / label / 'isolated-root-placement.json', {
        'same_placed_scorer': before['sha256'],
        'restores_reference_root': True,
        'reference_root_profile': current['flags']['MECH_COMPACT_ROOT_TAIL_PROFILE'],
    })


if __name__ == '__main__':
    main()
