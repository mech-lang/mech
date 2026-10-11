"""Build an untimed generic nested-cell identity diagnostic with existing CLT."""
import hashlib,json,subprocess
from pathlib import Path
D=Path(__file__).resolve().parent
compiler=Path('/Library/Developer/CommandLineTools/usr/bin/clang++')
matlab=Path('/Applications/MATLAB_R2026b.app')
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
command=[str(compiler),'-std=c++17','-DMX_HAS_INTERLEAVED_COMPLEX=1','-O2','-fPIC','-isysroot','/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk','-bundle','-I'+str(matlab/'extern/include'),'-L'+str(matlab/'bin/maca64'),'-Wl,-rpath,'+str(matlab/'bin/maca64'),'allocation_identity.cpp','-lmx','-lmex','-lmat','-o','allocation_identity.mexmaca64']
subprocess.run(command,cwd=D/'evidence',check=True)
record=dict(status='built',command=command,compiler_sha256=sha(compiler),source_sha256=sha(D/'evidence/allocation_identity.cpp'),binary_sha256=sha(D/'evidence/allocation_identity.mexmaca64'),note='Untimed generic identity inventory only. MATLAB benchmark sources do not call this diagnostic. Uses the existing Apple Command Line Tools directly, with its own SDK; does not change Xcode settings or accept its license.')
(D/'evidence/probe-build.json').write_text(json.dumps(record,indent=2)+'\n')
print('Built untimed generic cell identity diagnostic.')
