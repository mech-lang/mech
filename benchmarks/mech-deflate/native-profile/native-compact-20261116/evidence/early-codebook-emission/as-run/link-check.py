"""Bind the actual early-unit object to fixed support and verify checked behavior."""
from pathlib import Path
from runpy import run_path
import json
ROOT=Path(__file__).resolve().parent
LAB=ROOT.parent
h=run_path(str(LAB.parent/'current-roadmap/checked-root-census/recheck-link.py'),run_name='early_unit_link_check')
reference=LAB/'preserved-registers/preserved/bound'
out=ROOT/'normal/linked'
workers=out/'worker-checks'
for role,artifact in [('maintained',reference),('early-codebook',ROOT/'normal/early-codebook/frontend')]:
 record=h['link_worker'](reference,artifact,workers/role)
 if role=='maintained':assert record['worker_sha256']==json.loads((reference/'runtime.json').read_text())['worker_sha256']
 h['check_worker'](workers/role,'early-source-unit-'+role)
 print(role,record['worker_sha256'],flush=True)
code=h['check_contracts'](out,('maintained','early-codebook'))
print('linked contract exit',code,flush=True)
raise SystemExit(code)
