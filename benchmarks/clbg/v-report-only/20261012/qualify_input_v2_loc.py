"""Apply the executed immutable V LOC API; keep every review diagnostic."""
import fcntl
import hashlib
import json
from pathlib import Path
import sys
from datetime import datetime,timezone

HERE=Path(__file__).resolve().parent
LOC=HERE.parents[1]/'loc-normalization/20261012'
FIXTURE=LOC/'jobs/vlang-fixtures-02/receipt.json'
SOURCES=HERE/'revisions/input-v2/profiles.json'
OUTPUT=HERE/'revisions/input-v2/loc-proposal.json'
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def main():
    print('WAIT common mutex: V input-v2 source LOC proposals',flush=True)
    with open('/tmp/mech-clbg-timing.lock','a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        assert not OUTPUT.exists(),'No overwrite of prior source-count evidence'
        receipt=json.loads(FIXTURE.read_text())
        assert receipt['status']=='passed' and receipt['tests_run']==17
        for path,digest in receipt['frozen_module_pins'].items():assert sha(path)==digest
        assert sys.version.split()[0]==receipt['python_version'].split()[0]
        sys.path.insert(0,str(Path(receipt['pygments_module']).parent.parent))
        assert sha(receipt['pygments_module'])==receipt['pygments_module_sha256']
        snapshot=FIXTURE.parent/'snapshot'
        sys.path.insert(0,str(snapshot))
        from vlang import normalize_vlang
        document=json.loads(SOURCES.read_text())
        input_fixture=HERE/'input-v2-fixtures.json'
        ownership=json.loads(input_fixture.read_text())
        assert ownership['status']=='passed' and ownership['tests_run']==8
        rows=document['profiles'] if isinstance(document,dict) else document
        assert len(rows)==20
        pins=dict(receipt['frozen_module_pins'])
        pins.update(ownership['pins'])
        pins[str(input_fixture)]=sha(input_fixture)
        pins.update({str(FIXTURE):sha(FIXTURE),str(SOURCES):sha(SOURCES),str(Path(__file__).resolve()):sha(__file__)})
        proposals=[]
        for row in rows:
            source=(HERE/row['source']).resolve(strict=True)
            assert source.is_relative_to(HERE/'revisions/input-v2/sources')
            digest=sha(source)
            assert digest==ownership['pins'][str(source)]
            assert row['source_sha256'] is None or digest==row['source_sha256']
            pins[str(source)]=digest
            proposals.append(dict(id=row['id'],benchmark=row['benchmark'],variant=row['variant'],source=str(source),
                source_sha256=digest,**normalize_vlang(source.read_text(),'vlang')))
        for path,digest in pins.items():assert sha(path)==digest
        OUTPUT.write_text(json.dumps(dict(status='source-proposals-awaiting-semantic-review',created_at=datetime.now(timezone.utc).isoformat(),
            profiles=proposals,pins=pins,fixture=str(FIXTURE),report_only=True,never_chart_eligible=True,
            parser_pass_is_not_semantic_certification=True),indent=2)+'\n')
        print(json.dumps(dict(output=str(OUTPUT),sha256=sha(OUTPUT),profiles=len(proposals),
            needs_review=sum(bool(row['diagnostics']) for row in proposals))),flush=True)
if __name__=='__main__':main()
