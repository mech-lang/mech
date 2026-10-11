#!/usr/bin/env python3
"""Install pinned portable ARM64 GNU R and its ordinary GMP package locally."""
from pathlib import Path
import urllib.request,hashlib,tarfile,subprocess,json
G=Path(__file__).resolve().parent
T=G/'.build/toolchains'
T.mkdir(parents=True,exist_ok=True)
PINS=json.loads((G/'r-install-pins.json').read_text())

def download(row,path):
 if not path.exists():path.write_bytes(urllib.request.urlopen(row['url']).read())
 assert hashlib.sha256(path.read_bytes()).hexdigest()==row['sha256'],str(path)
 return path

archive=download(PINS['runtime'],T/'R-4.6.1-macos-arm64.tar.gz')
runtime=T/'R-4.6.1/bin/Rscript'
if not all(p.exists() for p in [runtime,T/'R-4.6.1/bin/exec/R',T/'R-4.6.1/lib/libR.dylib']):
 skipped=[]
 with tarfile.open(archive) as tar:
  for member in tar:
   try:tar.extract(member,T,filter='data')
   except (tarfile.LinkOutsideDestinationError,tarfile.AbsoluteLinkError):
    # CRAN fontconfig aliases outside this portable tree are unused by this suite.
    assert member.name.startswith('R-4.6.1/fontconfig/fonts/conf.d/')
    skipped.append({'path':member.name,'link':member.linkname})
 (T/'fontconfig-links-skipped.json').write_text(json.dumps(skipped,indent=2)+'\n')
packages=T/'package-archives'
packages.mkdir(exist_ok=True)
binary=download(PINS['gmp_binary'],packages/'gmp_0.7-5.1.tgz')
download(PINS['gmp_source'],packages/'gmp_0.7-5.1.tar.gz')
library=T/'r-library'
library.mkdir(exist_ok=True)
if not (library/'gmp').exists():
 expression='install.packages('+json.dumps(str(binary))+',lib='+json.dumps(str(library))+',repos=NULL,type="binary")'
 subprocess.run([str(runtime),'--vanilla','-e',expression],check=True)
subprocess.run([str(runtime),'--vanilla','-e','cat(R.version.string,"\\n")'],check=True)
