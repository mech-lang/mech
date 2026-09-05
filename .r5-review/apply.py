from pathlib import Path
import base64
import hashlib
import os
import subprocess
import sys
import zlib

root = Path(sys.argv[1]).resolve()
here = Path(__file__).resolve().parent
base = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
assert base == os.environ['BASE'], (base, os.environ['BASE'])
encoded = ''.join((here / f'patch.{i}').read_text().strip() for i in range(4))
patch = zlib.decompress(base64.b64decode(encoded, validate=True))
assert len(patch) == 83007
assert hashlib.sha256(patch).hexdigest() == '0b2a11d4d00ff4e290fb589fcbc0579b5beeb1fefe22f65a80c1f570608ebc09'
subprocess.run(['git', '-C', str(root), 'apply', '--check', '-'], input=patch, check=True)
subprocess.run(['git', '-C', str(root), 'apply', '-'], input=patch, check=True)
print('Applied exact locally validated R5 patch: ' + hashlib.sha256(patch).hexdigest())
subprocess.run(['git', '-C', str(root), 'diff', '--stat'], check=True)
