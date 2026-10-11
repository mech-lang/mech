"""Check complete small outputs before admitting any .NET program."""
import json
from dotnet_fetch import D,sha
from metrics import normalize
from probe import validate_small,save

def main():
    report=json.loads((D/'dotnet_jit_builds.json').read_text())
    path=D/'results/dotnet-small.json'
    small=json.loads(path.read_text()) if path.exists() else dict(status='running',programs={})
    for row in report['programs']:
        if row['build_status']!='passed':continue
        if sha(row['source'])!=row['source_sha256'] or sha(row['binary'])!=row['binary_sha256']:
            raise AssertionError('Changed build input '+row['id'])
        old=small['programs'].get(row['key'])
        if old and old['source_sha256']==row['source_sha256'] and old['binary_sha256']==row['binary_sha256']:continue
        check=dict(source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'])
        try:
            check.update(status='passed',validation=validate_small(row))
            print('SMALL PASS',row['id'],flush=True)
        except Exception as e:
            check.update(status='failed',error=str(e))
            print('SMALL FAIL',row['id'],str(e)[-750:],flush=True)
        small['programs'][row['key']]=check;save(path,small)
    small['status']='reviewed';save(path,small)

if __name__=='__main__':main()
