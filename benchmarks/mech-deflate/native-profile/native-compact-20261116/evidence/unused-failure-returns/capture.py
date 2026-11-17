"""Bind the source-only ablation and measure its emitted artifacts."""
from importlib import import_module
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from build import FLAG, HEAD, HERE, sha, write

sys.path.insert(0, str(HERE.parent))
driver = import_module('measure-deployment-builds')
BASE = HERE.parent / 'statement-compiler/matched/plan.json'
ORIGINAL = 'mech-statement-replay'
ARTIFACTS = {'program.mecb': 'bytecode', 'program.o': 'encoder', 'metadata.json': 'metadata'}


def main():
    compiler = HERE / 'candidate/compile_profile'
    assert sha(compiler) == driver.read(HERE / 'candidate/build.json')['compiler_sha256']
    plan = driver.read(BASE)
    pinned = plan['variants'][ORIGINAL]
    source = Path(pinned['source_command'][1])
    assert sha(source) == plan['source_sha256']['mech']
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    out = HERE / 'capture'
    out.mkdir()
    metadata = {}
    records = {}
    for name, extra in [('materialize', {}), ('skip-unused', {FLAG: '1'})]:
        work = out / name
        work.mkdir()
        command = [str(compiler), str(source), str(work / 'compiled'), 'compact']
        flags = pinned['native_flags'] | extra
        log = work / 'source.log'
        with log.open('wb') as output:
            subprocess.run(command, cwd=HERE, env=env | flags | {'PATH': '/usr/bin:/bin'},
                           stdout=output, stderr=subprocess.STDOUT, check=True)
        outputs = {field: sha(work / 'compiled' / name) for name, field in ARTIFACTS.items()}
        for f in ('bytecode', 'encoder'):
            assert outputs[f] == pinned['expected_artifacts'][f], (name, f, outputs[f])
        metadata[name] = driver.read(work / 'compiled/metadata.json')
        hits = re.findall(r'^compact unused failure returns skipped: (\d+)$', log.read_text(), re.M)
        assert (len(hits) == 1 and int(hits[0]) > 0) if extra else not hits
        records[name] = {'command': command, 'artifacts': outputs, 'discarded_returns': int(hits[0]) if hits else 0,
                         'compiler_phases': driver.read(work / 'compiled' / 'compile_phases.json')
                         if (work / 'compiled' / 'compile_phases.json').is_file() else None}
    old, new = metadata.values()
    write(out / 'result.json', {'source_pin': HEAD, 'compiler_sha256': sha(compiler),
                               'parent_plan_sha256': sha(BASE), 'ablation_flag': FLAG,
                               'expected_artifacts': pinned['expected_artifacts'], 'actors': records,
                               'metadata_difference_fields': sorted(key for key in old.keys() | new.keys()
                                                                   if old.get(key) != new.get(key))})
    print({name: (x['artifacts'], x['discarded_returns']) for name, x in records.items()}, flush=True)


if __name__ == '__main__':
    main()
