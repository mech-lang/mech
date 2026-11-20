"""Reproduce a pinned full-codec source alternative, without LLVM."""
import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
from generate_codec import generate
from build import text_size


def digest(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('compiler', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--role', choices=['source-control','all-buckets','hybrid-u64','hybrid-u32'], default='hybrid-u32')
    parser.add_argument('--checked', action='store_true')
    args = parser.parse_args()
    here, out = Path(__file__).resolve().parent, args.output.resolve()
    recipe = json.loads((here/'recipes.json').read_text())['roles'][args.role]
    original = here.parents[1]/'program.mec'
    assert digest(original) == '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
    source = original.read_text() if recipe['generator_role'] == 'control' else generate(original.read_text(), recipe['generator_role'])
    assert sha256(source.encode()).hexdigest() == recipe['source_sha256']
    out.mkdir(parents=True, exist_ok=False)
    (out/'program.mec').write_text(source)
    flags = dict(recipe['flags'])
    profiles = []
    for value in recipe['profiles']:
        path = here/value['file']
        assert digest(path) == value['sha256']
        profiles.append(value['symbol']+'@'+str(path))
    if profiles:
        flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'] = ';'.join(profiles)
    if args.checked:
        flags['MECH_COMPACT_PAIR_REGISTER_SPILLS'] = 'checked'
    environment = {k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
    command = [str(args.compiler.resolve()),str(out/'program.mec'),str(out/'frontend'),'compact']
    with (out/'compile.log').open('w') as log:
        subprocess.run(command,env=environment|flags,stdout=log,stderr=subprocess.STDOUT,check=True)
    artifacts = {name:digest(out/'frontend'/name) for name in recipe['artifacts']}
    assert artifacts == recipe['artifacts'], (args.role,artifacts,recipe['artifacts'])
    size = text_size(out/'frontend/program.o')
    assert size == recipe['text_bytes']
    result = {'role':args.role,'checked':args.checked,'command':command,'flags':flags,
        'compiler_sha256':digest(args.compiler),'reference_compiler_sha256':recipe['compiler_sha256'],
        'source_sha256':digest(out/'program.mec'),'artifacts':artifacts,'all_three_exact':True,
        'encoder_text_bytes':size,'encoder_text_x_Roc':size/57696,'encoder_text_x_C':size/37080,
        'scope':'Actual current-compiler source-to-object replay. Complete executable build times and runtime qualification are separate archived campaigns.'}
    (out/'reproduction.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('flags','command')},indent=2))


if __name__ == '__main__':
    main()
