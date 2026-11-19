from pathlib import Path
from hashlib import file_digest
import json
import subprocess
import sys

root = Path(__file__).resolve().parent
worker = Path(sys.argv[1]).resolve()
metadata = Path(sys.argv[2]).resolve()
label = sys.argv[3]
fixture_path = Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/fixtures.json')
fixtures = json.loads(fixture_path.read_text())
assert len(fixtures)==81
receipt = root/(label+'-qualification.json')
log = root/(label+'-smokes.stderr')
assert not receipt.exists()
rows = []
command = [str(worker), str(metadata), str(fixture_path)]
with log.open('w') as error, subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.PIPE,
                                               stderr=error,text=True) as child:
    try:
        assert child.stdout.readline().strip() == 'SESSION_READY'
        for i, fixture in enumerate(fixtures):
            name = fixture['name']
            child.stdin.write(f'C\t{i}\t0\nNN')
            child.stdin.flush()
            assert child.stdout.readline().strip() == f'CASE_READY\t{name}'
            for sample in range(2):
                row = json.loads(child.stdout.readline())
                assert row['case'] == name
                for field in ('input_bytes','raw_bytes','prefix_bytes','capacity'):
                    assert row[field] == fixture[field], (name, field)
                for field in ('exact_reference','independent_inflation','input_alias_unchanged',
                              'borrowed_output_alias_unchanged'):
                    assert row[field], (name, field)
                del row['seconds']
                rows.append(row)
            assert child.stdout.readline().strip() == f'CASE_DONE\t{name}'
        child.stdin.write('Q\n')
        child.stdin.flush()
        assert child.wait() == 0
    finally:
        if child.poll() is None:
            child.terminate()
assert log.read_text().endswith('CLEANUP private_payload_bytes=0 live_owner_records=0\n')
assert len(rows)==162
def sha(path):
    with path.open('rb') as f:
        return file_digest(f,'sha256').hexdigest()
(root/(label+'-smokes.json')).write_text(json.dumps(rows,indent=2)+'\n')
receipt.write_text(json.dumps(dict(command=command,complete_smokes=len(rows),verified=True,
                                   worker_sha256=sha(worker),metadata_sha256=sha(metadata)),indent=2)+'\n')
print(label,len(rows),'exact linked calls')
