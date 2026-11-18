"""Run the full pinned syntax unit suite against the candidate source."""

from pathlib import Path
import os
import re
import shutil
import subprocess

from stage import CHECKOUT, HERE, ROADMAP, SOURCE, read, sha, write


def main():
    here = HERE / 'syntax-validation'
    here.mkdir()
    src = here / 'source'
    paths = [
        Path('src/syntax/src'), Path('src/syntax/tests'), Path('src/core/tests/support/allocation_probe.rs'),
        Path('docs/design/grammar-audit'), Path('tests/fixtures/syntax-source-boundary'),
    ]
    for path in paths:
        if (CHECKOUT / path).is_dir():
            shutil.copytree(CHECKOUT / path, src / path)
        else:
            (src / path).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(CHECKOUT / path, src / path)
    shutil.copyfile(HERE / 'source' / SOURCE, src / SOURCE)
    assert sha(src / SOURCE) == read(HERE / 'source.json')['candidate_sha256']
    recipe = ROADMAP / 'ascii-graphemes/candidate/tests.json'
    previous = read(recipe)
    arguments = previous['build'].copy()
    executable = here / 'syntax-tests'
    input_lib, = [arg for arg in arguments if arg.endswith('/src/syntax/src/lib.rs')]
    output_slot = arguments.index('-o') + 1
    arguments[arguments.index(input_lib)] = str(src / 'src/syntax/src/lib.rs')
    arguments[output_slot] = str(executable)
    arguments = ['metadata=retained-child-traversal-test' if arg.startswith('metadata=') else arg
                 for arg in arguments]
    extern = {arg.split('=', 1)[0]: Path(arg.split('=', 1)[1])
              for flag, arg in zip(arguments, arguments[1:]) if flag == '--extern'}
    assert extern and all(path.is_file() for path in extern.values())
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    env.update(CARGO_MANIFEST_DIR=str(src / 'src/syntax'), CARGO_PKG_VERSION='0.3.5')
    with (here / 'build.log').open('w') as log:
        subprocess.run(arguments, env=env, cwd=src, stdout=log, stderr=subprocess.STDOUT, check=True)
    with (here / 'tests.log').open('w') as log:
        subprocess.run([executable], env=env, cwd=src, stdout=log, stderr=subprocess.STDOUT, check=True)
    result = (here / 'tests.log').read_text()
    match = re.search(r'test result: ok\. (\d+) passed; 0 failed;', result)
    assert match and int(match.group(1)) >= 538
    assert 'owned_traversal_keeps_order_after_the_parent_is_dropped ... ok' in result
    write(here / 'result.json', {'recipe_sha256': sha(recipe), 'command': arguments,
          'source_sha256': sha(src / SOURCE), 'passed': int(match.group(1)),
          'external_libraries': {name: sha(path) for name, path in extern.items()},
          'tests_log_sha256': sha(here / 'tests.log'), 'test_binary_sha256': sha(executable)})
    print(match[0], flush=True)


if __name__ == '__main__':
    main()
