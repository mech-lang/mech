#!/usr/bin/env python3
"""Compare recorded outputs from identical native and browser Mech sources."""
import hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];output=ROOT/'audit/v04/evidence'
log=(output/'type-inspection-native-final.log').read_text()
native=json.loads(next(line.split('=',1)[1] for line in log.splitlines() if line.startswith('NATIVE_TYPE_FIXTURES=')))
browser=json.loads((output/'types-browser.json').read_text());pairs=[]
for item in native:
 matched=next(row for row in browser['results'] if row['result']['source']==item['source'])
 a=[value['scalar_text'] for value in item['values']];b=[value['scalar_text'] for value in matched['result']['values']]
 pairs.append({'source':item['source'],'native':a,'browser':b,'equal':a==b,'artifact_bytes_native':item['artifact_bytes'],'artifact_bytes_browser':matched['result']['artifact_bytes']})
binary=Path(re.search(r'Running unittests .*\(([^)]+)\)',log).group(1))
passed=browser['status']=='passed' and all(row['equal'] for row in pairs) and '3 passed; 0 failed' in log
record={'id':'E-TYPES-NATIVE-BROWSER','claim':'Identical scalar sources publish exact expected values through native and WASM resident execution','contract':'docs/design/grammar-audit/fixed-integer-intervals.md; canonical source contracts','source_commit':'c4777b7015fe8ff47fdfa48d18606c49aace7d97','source_state':'baseline with recorded audit corrections and optional inspection adapters','artifact_identity':{'native_binary':str(binary),'native_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'wasm_manifest':'audit/v04/site/artifact.json','wasm_sha256':browser['metadata']['loaded_wasm_sha256']},'command':'CARGO_TARGET_DIR=/private/tmp/mech-v04-target-architecture cargo test -p mech-wasm --locked --offline --no-default-features --features browser_project,browser_compute,u8,u64,u128,syntax_inspection,type_inspection type_inspection::tests -- --nocapture','comparison_command':'python3 audit/v04/compare_type_targets.py','evidence_level':'native-and-browser-execution','outcome':'passed' if passed else 'failed','native_test_result':'3 passed; 104 filtered out; other integration targets had 0 selected tests','pairs':pairs,'results':['audit/v04/evidence/type-inspection-native-final.log','audit/v04/evidence/types-browser.json']}
(output/'types-native-browser-comparison.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({'outcome':record['outcome'],'pairs':len(pairs)}));raise SystemExit(0 if passed else 1)
