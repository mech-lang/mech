"""Time the source-generated read outline through the common deployment link."""
from importlib import import_module
from pathlib import Path
from statistics import median
import sys

here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
driver = import_module('measure-deployment-builds')
control, candidate, roc, lib = ('mech-control', 'mech-read-outline', 'roc', 'libdeflate')
actors = (control, candidate, roc, lib)


def main():
    folder = here / 'matched'
    folder.mkdir()
    build = here.parent / 'phi-verifier'
    prebuilt = driver.read(build / 'matched/plan.json')
    smaller = driver.read(here / 'screen/general-byte-read/result.json')
    mach = driver.read(build / 'candidate/rebuild/rebuild.json')
    compiler = build / 'candidate/compile_profile'
    assert driver.sha(compiler) == prebuilt['variants']['mech-phi-verifier']['compiler_sha256']
    fixed_record = prebuilt['variants']['mech-phi-verifier']
    active_flags = mach['native_flags']
    layout_key = 'MECH_COMPACT_PHASE_BLOCK_LAYOUT'
    previous_flags = fixed_record['native_flags']
    assert {k: v for k,v in previous_flags.items() if k != layout_key} == {k: v for k,v in active_flags.items() if k != layout_key}
    profile_fields = ('order', 'literal_offset', 'object_tail')
    def profiles(text):
        definitions = (item.split('@', 1) for item in text.split(';'))
        rows = ((symbol, driver.read(Path(path))) for symbol, path in definitions)
        return {symbol: {name: row.get(name) for name in profile_fields} for symbol,row in rows}
    assert profiles(previous_flags[layout_key]) == profiles(active_flags[layout_key])
    assert fixed_record['expected_artifacts']['encoder'] != smaller['identity']['program.o']
    assert fixed_record['expected_artifacts']['bytecode'] == smaller['identity']['program.mecb']
    baseline = driver.read(driver.MATCHED / 'plan.json')
    stock = driver.read(driver.STOCK / 'build.json')
    small_host = driver.read(driver.SMALL / 'receipt.json')['commands']['link']
    object_pin, = [arg for arg in small_host if arg.endswith('/program.o')]
    wanted = smaller['identity']
    candidate_artifacts = {'encoder': wanted['program.o'], 'bytecode': wanted['program.mecb'],
                           'metadata': wanted['metadata.json']}
    assert driver.sha(fixed_record['source_command'][1]) == baseline['mech_source_sha256']
    choices = {
        control: (fixed_record['source_command'], small_host, object_pin, active_flags,
                  fixed_record['expected_artifacts']),
        candidate: (fixed_record['source_command'], small_host, object_pin, smaller['flags'],
                    candidate_artifacts),
    }
    for peer, ext in ((roc, 'a'), (lib, 'o')):
        frozen = driver.MATCHED / 'warmup' / peer
        source = dict(driver.read(frozen / 'commands.json'))['compile']
        object_file = str(frozen / f'encoder.{ext}')
        choices[peer] = source, stock['commands'][f'link-{peer}'], object_file, {}, {'encoder': driver.sha(object_file)}
    assert driver.sha(choices[roc][0][-1]) == baseline['roc_source_sha256']
    assert driver.sha(choices[roc][0][0]) == baseline['roc_compiler_sha256']
    lib_source, = [arg for arg in choices[lib][0] if arg.endswith('/deflate_compress.c')]
    assert driver.sha(lib_source) == baseline['c_source_sha256']
    order = [[actors[(i + s) % 4] for i in (0, 1, 3, 2)] for s in range(4)] * 2
    frozen = {}
    for actor, (src, link, obj, flags, pins) in choices.items():
        link = [option for option in link if not option.startswith('-Wl,-map,')]
        choices[actor] = src, link, obj, flags, pins
        frozen[actor] = {'source_command': src, 'compiler_sha256': driver.sha(src[0]), 'link_command': link,
                         'fixed_link_inputs': {arg: driver.sha(arg) for arg in link[:link.index('-o')]
                                               if arg != obj and arg.endswith(('.o', '.a', '.rlib'))},
                         'native_flags': flags, 'expected_artifacts': pins}
    plan = {'order': order, 'warmup': actors, 'variants': frozen,
            'mech_source_sha256': baseline['mech_source_sha256'], 'same_compiler_and_deployment_for_mech': True}
    driver.save(folder / 'plan.json', plan)

    def sample(actor, run):
        return driver.compile_and_link(actor, folder / run / actor, *choices[actor])

    warmup = {actor: sample(actor, 'warmup') for actor in actors}
    trials = [{actor: sample(actor, f'order-{index}') for actor in actors_in_run}
              for index, actors_in_run in enumerate(order, 1)]
    for row in trials:
        for actor in actors:
            assert row[actor]['generated_artifacts'] == warmup[actor]['generated_artifacts']
            assert row[actor]['stripped_executable'] == warmup[actor]['stripped_executable']
    medians = {actor: {phase: median(row[actor]['seconds'][phase] for row in trials)
                       for phase in warmup[actor]['seconds']} for actor in actors}
    phases = {actor: {phase['phase']: median(next(item['seconds'] for item in row[actor]['compiler_phases']
                                                 if item['phase'] == phase['phase']) for row in trials)
                      for phase in warmup[actor]['compiler_phases']}
              for actor in (control, candidate)}
    ratios = {actor: median(row[candidate]['seconds']['source_through_link'] /
                            row[actor]['seconds']['source_through_link'] for row in trials)
              for actor in (control, roc, lib)}
    driver.save(folder / 'comparison.json', {'plan_sha256': driver.sha(folder / 'plan.json'), 'medians_seconds': medians,
                'compiler_phase_medians_seconds': phases, 'candidate_paired_build_x': ratios,
                'stripped_executables': {actor: warmup[actor]['stripped_executable'] for actor in actors},
                'warmups': warmup, 'trials': trials})
    print(medians, ratios, flush=True)


if __name__ == '__main__':
    main()
