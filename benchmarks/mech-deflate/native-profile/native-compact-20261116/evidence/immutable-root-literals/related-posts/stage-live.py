"""Prepare the archived checked after-4 producer on the current review tree."""

from hashlib import sha256
from pathlib import Path
import gzip
import json
from io import BytesIO
import os
import re
import subprocess
import tarfile

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
VIEW=HERE.parent/'pinned-formatter/workspace'
BASE='ed99ade92836761bebd2f7357bf46817204433d4'
PACKAGE='benchmarks/mech-deflate/native-profile/native-compact-20261116'
ARCHIVE=PACKAGE+'/evidence/related-post-linked-binding/checked-after-4/source.patch.gz'

def git(*args,cwd=ROOT):
    return subprocess.check_output(['git',*map(str,args)],cwd=cwd)

def sha(data):
    return sha256(data).hexdigest()

def main():
    prior=json.loads((HERE/'patch-applicability.json').read_bytes())
    patch=gzip.decompress(git('show',f'{BASE}:{ARCHIVE}'))
    assert sha(patch)==prior['patch_sha256']
    expected=Path('.git')
    assert (VIEW/expected).is_dir() and Path(git('rev-parse','--show-toplevel',cwd=VIEW).decode().strip())==VIEW
    names=re.findall(rb'^diff --git a/(\S+) b/\1$',patch,flags=re.MULTILINE)
    assert len(names)==11
    inputs={}
    for filename in map(os.fsdecode,names):
        path=VIEW/filename
        existed=subprocess.run(['git','cat-file','-e',f'{BASE}:{filename}'],cwd=ROOT,stderr=subprocess.PIPE)
        if existed.returncode==0:
            source=git('show',f'{BASE}:{filename}')
            if path.exists():
                assert path.read_bytes()==source,path
            else:
                path.parent.mkdir(parents=True,exist_ok=True)
                path.write_bytes(source)
            inputs[filename]=sha(source)
        else:
            assert not path.exists(),path
    # Copy the maintained non-Rust source and package descriptions used by the
    # producer. Old binary snapshots and large benchmark fixtures stay separate.
    scopes=('benchmarks/related-post-gen/sources','hosts/gpu/src/native_artifact/templates')
    bundle=git('archive','--format=tar',BASE,*scopes)
    assert len(bundle)<8*1024*1024
    with tarfile.open(fileobj=BytesIO(bundle)) as stream:
        stream.extractall(VIEW,filter='data')
    git('update-ref','HEAD',BASE,cwd=VIEW)
    git('read-tree',BASE,cwd=VIEW)
    subprocess.run(['git','apply','--check','-'],input=patch,cwd=VIEW,check=True)
    subprocess.run(['git','apply','-'],input=patch,cwd=VIEW,check=True)
    paths={os.fsdecode(name):sha((VIEW/os.fsdecode(name)).read_bytes()) for name in names}
    audit=['bash','scripts/check-unsafe-boundaries.sh']
    audit_result=subprocess.run(audit,cwd=VIEW,capture_output=True)
    logs=HERE/'checked-after-4-audit.log'
    logs.write_bytes(audit_result.stdout+audit_result.stderr)
    assert 'hosts/after-4.rs' not in logs.read_text(),logs.read_text()
    command=['cargo','+nightly-2026-03-03','metadata','--offline','--locked','--no-deps','--format-version','1']
    meta=subprocess.check_output(command,cwd=VIEW)
    packages=json.loads(meta)['packages']
    gpu=next(p for p in packages if p['name']=='mech-gpu')
    target=next(t for t in gpu['targets'] if t['name']=='related_post_after_4')
    result={'preparation_base':BASE,'patch_sha256':sha(patch),'previous_paths':inputs,'overlay_files':paths,
            'metadata_command':command,'producer_target':target,'workspace':str(VIEW),
            'static_gate':{'command':audit,'status':audit_result.returncode,'log_sha256':sha(logs.read_bytes())}}
    (HERE/'staged.json').write_text(json.dumps(result,indent=2)+'\n')
    print(target['src_path'])
    print(logs.read_text()[-850:],flush=True)

if __name__=='__main__':
    main()
