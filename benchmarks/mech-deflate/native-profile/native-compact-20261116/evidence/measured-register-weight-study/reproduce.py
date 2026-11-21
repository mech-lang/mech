"""Rebuild a separately pinned ordinary-Mech source using measured register spill weights on Apple ARM64."""
import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
from generate import generate


def digest(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('compiler', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--role', choices=['hybrid-linear','hybrid-sqrt'], default='hybrid-linear')
    parser.add_argument('--checked', action='store_true')
    args = parser.parse_args()
    here, out = Path(__file__).resolve().parent, args.output.resolve()
    recipe = json.loads((here/'recipes.json').read_text())['roles'][args.role]
    base = here/recipe['base_source']
    assert digest(base) == recipe['base_source_sha256']
    source = generate(base.read_text())
    assert sha256(source.encode()).hexdigest() == recipe['source_sha256']
    assert (here/recipe['source_file']).read_text() == source
    out.mkdir(parents=True, exist_ok=False)
    (out/'program.mec').write_text(source)
    profiles = {}
    for profile in recipe['profiles']:
        path = here/profile['file']
        assert digest(path) == profile['sha256']
        profiles.setdefault(profile['control'], []).append(profile['symbol']+'@'+str(path))
    flags = recipe['flags'] | {control:';'.join(entries) for control,entries in profiles.items()}
    if args.checked:
        flags['MECH_COMPACT_PAIR_REGISTER_SPILLS'] = 'checked'
    command = [str(args.compiler.resolve()), str(out/'program.mec'), str(out/'frontend'), 'compact']
    with (out/'compile.log').open('w') as log:
        subprocess.run(command, env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,
                       stdout=log, stderr=log, check=True)
    artifacts = {name:digest(out/'frontend'/name) for name in recipe['artifacts']}
    assert artifacts == recipe['artifacts'], (args.role, artifacts)
    result = {'role':args.role, 'checked':args.checked, 'command':command, 'flags':flags,
              'compiler_sha256':digest(args.compiler), 'source_sha256':digest(out/'program.mec'),
              'artifacts':artifacts, 'all_three_exact':True}
    (out/'reproduction.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('command','flags')}, indent=2))


if __name__ == '__main__':
    main()
