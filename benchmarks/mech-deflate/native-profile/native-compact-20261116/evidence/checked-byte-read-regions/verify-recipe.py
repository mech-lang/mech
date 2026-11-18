"""Test the optional read-outline source rebuild in a short-lived checkout subdirectory."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

from screen import sha, read, write

here = Path(__file__).resolve().parent
q = here.parents[1]
checkout = q / 'native-review-pr/checkout'
packet = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')
real = checkout / packet
sample = here / 'screen/general-byte-read/compiled'
compiler = here.parent / 'phi-verifier/candidate/compile_profile'


def main():
    target = here.parent / 'definition-compiler/target'
    expected = read(here / 'recipe-bindings.json')
    assert expected['variant'] == 'shared-byte-read-layout'
    with TemporaryDirectory(dir=real.parent, prefix='read-outline-recipe-') as tmp:
        home = Path(tmp)
        for name, digest in expected['sources'].items():
            new = here / 'recipe-overlay' / name
            assert sha(new) == digest
            shutil.copyfile(new, home / new.name)
        for name in ('program.mec', 'profiles'):
            (home / name).symlink_to(real / name, target_is_directory=name == 'profiles')
        out = here / 'optional-recipe-rebuild'
        args = [sys.executable, '-B', home / 'rebuild.py', out, '--variant', expected['variant'],
                '--compiler-opt-level', '3', '--direct-source', '--preselect-root-inlining',
                '--skip-refused-general-ssa', '--target-dir', target]
        with (here / 'optional-recipe.log').open('w') as log:
            subprocess.run(args, env=os.environ | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                           stderr=subprocess.STDOUT, check=True)
        assert sha(target / 'release/compile_profile') == sha(compiler)
        record = read(out / 'rebuild.json')
        assert record['full_integrated_source_identity'] and record['object_text_bytes'] == 71420
        exact = {'program.mecb': sha(out / 'frontend/program.mecb'), 'program.o': sha(out / 'native/program.o'),
                 'metadata.json': sha(out / 'native/metadata.json')}
        assert exact == {name: sha(sample / name) for name in exact}
        write(here / 'optional-recipe.json', {'command': list(map(str, args)), 'compiler': sha(compiler),
                                              'bindings': expected, 'artifacts': exact})
        print(exact, flush=True)


if __name__ == '__main__':
    main()
