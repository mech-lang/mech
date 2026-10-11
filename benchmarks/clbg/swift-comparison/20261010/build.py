"""Compile all Swift candidates before timing; retain failed build diagnostics."""
import concurrent.futures, hashlib, json, shutil, subprocess
from datetime import datetime, timezone
from pathlib import Path
from fetch import metrics
D=Path(__file__).resolve().parent
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def build(row):
    row=row.copy();source=Path(row['source'])
    assert sha(row['original_source'])==row['original_source_sha256']
    compiler=shutil.which('swiftc')
    binary=D/'binaries'/row['id']
    cmd=[compiler,'-Ounchecked','-whole-module-optimization',str(source),'-o',str(binary)]
    support=[]
    if row['benchmark']=='pidigits':
        cmd+=['-I',str(D/'support/gmp'),'-L','/opt/homebrew/opt/gmp/lib','-lgmp']
        support=['support/gmp/module.modulemap']
    process=subprocess.run(cmd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    log=D/'results'/(row['id']+'.build.log');log.write_text(process.stdout)
    row.update(source_sha256=sha(source),source_metrics=metrics(source.read_text()),build_command=cmd,
               build_status='passed' if process.returncode==0 else 'failed',build_log=str(log),support_files=support)
    if process.returncode==0:row.update(binary=str(binary),binary_sha256=sha(binary))
    print('BUILD',row['id'],row['build_status'],flush=True)
    return row
def main():
    manifest=json.loads((D/'manifest.json').read_text())
    support=D/'support/gmp';support.mkdir(parents=True,exist_ok=True)
    (support/'module.modulemap').write_text('module gmp [system] {\n  header "/opt/homebrew/opt/gmp/include/gmp.h"\n  link "gmp"\n  export *\n}\n')
    compiler=Path(shutil.which('swiftc')).resolve()
    driver=Path(subprocess.check_output(['xcrun','--find','swiftc'],text=True).strip())
    frontend=Path(subprocess.check_output(['xcrun','--find','swift-frontend'],text=True).strip())
    corrections=json.loads((D/'corrections.json').read_text())
    assert corrections['manifest_sha256']==sha(D/'manifest.json')
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:rows=list(pool.map(build,manifest['programs']+corrections['programs']))
    report=dict(status='built',created_at=datetime.now(timezone.utc).isoformat(),manifest_sha256=sha(D/'manifest.json'),corrections_sha256=sha(D/'corrections.json'),
        compiler=dict(path=str(compiler),sha256=sha(compiler),version=subprocess.check_output([str(compiler),'--version'],text=True).strip(),
            driver_path=str(driver),driver_sha256=sha(driver),frontend_path=str(frontend),frontend_sha256=sha(frontend),
            target_info=json.loads(subprocess.check_output([str(compiler),'-print-target-info'],text=True))),
        flags=['-Ounchecked','-whole-module-optimization'],libraries={str(Path('/opt/homebrew/opt/gmp/lib/libgmp.10.dylib').resolve()):sha('/opt/homebrew/opt/gmp/lib/libgmp.10.dylib')},
        support_files={'support/gmp/module.modulemap':sha(support/'module.modulemap')},programs=rows)
    (D/'results/builds.json').write_text(json.dumps(report,indent=2)+'\n')
    print('BUILT',sum(r['build_status']=='passed' for r in rows),'of',len(rows),'sources')
if __name__=='__main__':main()
