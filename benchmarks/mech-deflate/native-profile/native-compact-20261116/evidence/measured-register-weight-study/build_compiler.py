"""Build the compiler from this checkout; no prebuilt encoder input is used."""
import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(__doc__)
parser.add_argument('target',type=Path)
parser.add_argument('--dry-run',action='store_true')
args = parser.parse_args()
repo = Path(__file__).resolve().parents[6]
target = args.target.resolve()
profile = {'CARGO_BUILD_JOBS':'1','CARGO_PROFILE_RELEASE_OPT_LEVEL':'3',
           'CARGO_PROFILE_RELEASE_DEBUG':'false','CARGO_PROFILE_RELEASE_LTO':'false',
           'CARGO_PROFILE_RELEASE_CODEGEN_UNITS':'16','CARGO_INCREMENTAL':'0'}
command = ['cargo','+nightly-2026-03-03','build','--locked','--release','--config',
           'profile.release.package.libmimalloc-sys.opt-level=1','--manifest-path',
           str(repo/'benchmarks/mech-deflate/native-profile/Cargo.toml'),
           '--target-dir',str(target),'--bin','compile_profile']
assert (repo/'benchmarks/mech-deflate/native-profile/Cargo.toml').is_file()
if args.dry_run:
    print(json.dumps({'command':command,'environment':profile},indent=2))
    raise SystemExit(0)
target.mkdir(parents=True,exist_ok=True)
start = time.monotonic()
subprocess.run(command,cwd=repo,env=os.environ|profile,check=True)
compiler = target/'release/compile_profile'
record = {'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),
          'command':command,'environment':profile,'seconds':time.monotonic()-start,
          'compiler_sha256':sha256(compiler.read_bytes()).hexdigest(),
          'scope':'Rust compiler build with this target directory; separate from source-through-final-link codec timing.'}
(target/'compiler-build.json').write_text(json.dumps(record,indent=2)+'\n')
print(compiler)
