from pathlib import Path
import json,hashlib,statistics
r=Path(__file__).resolve().parent
rows=[json.loads(l) for l in (r/'record-witness.stdout').read_text().splitlines()];assert len(rows)==128
assert sum(x['rejected'] for x in rows)==32
old=json.loads(Path('/private/tmp/mech-record-precharge-round16-20261029/primary/current-build.json').read_text())
assert all(hashlib.sha256(Path(v['path']).read_bytes()).hexdigest()==v['sha256'] for v in old['libraries'].values())
report={'compiler':'e982adbf6f7c060877bd253dbffe7b982dd9c67f','reviewed_head':'3615e260d96185ffbd3b7c57a9b7613742ac8207','driver_sha256':hashlib.sha256((r/'record-witness.rs').read_bytes()).hexdigest(),'binary_sha256':hashlib.sha256((r/'record-witness').read_bytes()).hexdigest(),'libraries':old['libraries'],'rows':len(rows),'successful_rows':96,'expected_rejections':32,'success_median_requests':statistics.median(x['allocation_requests'] for x in rows if not x['rejected']),'failure_median_requests':statistics.median(x['allocation_requests'] for x in rows if x['rejected']),'interpretation':'System/probe diagnostic, not primary; fresh owners and selectors, changing empty/rectangular shapes, original/decoded forms, unchanged publication, zero cache retention and final cleanup asserted.'}
(r/'baseline-receipt.json').write_text(json.dumps(report,indent=2)+'\n');print({k:v for k,v in report.items() if k not in ['libraries']})
