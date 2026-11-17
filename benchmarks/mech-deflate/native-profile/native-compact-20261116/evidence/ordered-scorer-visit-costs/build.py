"""Build the probe compiler and reproduce the maintained artifacts with the knobs off."""

import json
import os
from pathlib import Path
from shutil import copy2
import subprocess
import sys

from prepare import BASE, CHECKOUT, HERE, Q, RECEIPT
from census import digest

ROADMAP = HERE.parent
TARGET = ROADMAP / 'definition-compiler/target'
PACKET = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'
OUT = HERE / 'probe-compiler'
COMPILER = OUT / 'compile_profile'
ACTOR_PLAN = ROADMAP / 'flowgraph-verification/production/matched-repeat/plan.json'


def main():
    OUT.mkdir(exist_ok=True)
    source = json.loads(RECEIPT.read_bytes())
    assert source['base'] == BASE
    assert all(digest(CHECKOUT / name) == value for name, value in source['source_sha256'].items())
    command = [sys.executable, '-B', PACKET / 'rebuild.py', OUT / 'disabled',
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    clean = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    with (OUT / 'build.log').open('wb') as log:
        subprocess.run(command, env=clean | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    copy2(TARGET / 'release/compile_profile', COMPILER)
    expected = json.loads(ACTOR_PLAN.read_bytes())['actors']['mech-verified-flowgraph']['artifact_sha256']
    locations = {'encoder': 'native/program.o', 'metadata': 'native/metadata.json',
                 'bytecode': 'frontend/program.mecb'}
    produced = {name: digest(OUT / 'disabled' / path) for name, path in locations.items()}
    assert produced == expected, (produced, expected)
    receipt = source | {'compiler_sha256': digest(COMPILER), 'command': list(map(str, command)),
                        'disabled_artifacts': produced, 'patch_sha256': digest(HERE / 'scorer.patch')}
    (OUT / 'result.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt), flush=True)


if __name__ == '__main__':
    main()
