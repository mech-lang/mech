from pathlib import Path
import gzip,hashlib,json
P=Path(__file__).resolve().parent;m=json.loads((P/'manifest.json').read_text())
for name,v in m['files'].items():
 b=(P/name).read_bytes();assert len(b)==v['bytes'] and hashlib.sha256(b).hexdigest()==v['sha256']
 raw=gzip.decompress(b)if name.endswith('.gz')else b
 assert len(raw)==v['decoded_bytes'] and hashlib.sha256(raw).hexdigest()==v['decoded_sha256']
a=json.loads((P/'audit-summary.json').read_text());assert len(a['all13_normal_bundle_files_byte_identical'])==13 and all(a['all13_normal_bundle_files_byte_identical'].values())
assert a['optimized_clones']==51 and len(list((P/'ir').glob('*-before.clif.gz')))==51 and len(list((P/'ir').glob('*-optimized.clif.gz')))==51
n=json.loads(gzip.decompress((P/'IR251-noop-trial.json.gz').read_bytes()));assert not n['selected'] and not n['new_timing_cohort']
assert all(v['full_native_text_byte_identical']for v in n['normal_native_text_equivalence'].values())
assert len(n['focused_groups'])==8 and all(v['exit']==0 for v in n['focused_groups'].values())
assert n['actual_CI_inventory']['listed']==388 and not n['actual_CI_inventory']['executed']
print(json.dumps(dict(files=len(m['files']),normal_bundle_files_exact=13,optimized_clones=51,IR251_no_native_gain=True,selected_changed=False)))
