from pathlib import Path
import hashlib,importlib.util,json
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');spec=importlib.util.spec_from_file_location('audit',R/'audit-native-link.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
old=Path('/private/tmp/mech-related-native-boolean-implication-freshness-runners-20261106-v4');new=Path('/private/tmp/mech-related-local-facts-integration-runners-20261107-v1');out={}
def functions(bundle):
 b,sections,symbols=m.macho(bundle/'object.o');symbols={s['name']:s for s in symbols};meta=json.loads((bundle/'metadata.json').read_text());rows={}
 for row in meta['compiled_numeric_functions']:
  sym=symbols['_'+row['symbol']];sec=sections[sym['section']-1];at=sec['offset']+sym['value']-sec['address'];raw=b[at:at+row['code_bytes']];assert len(raw)==row['code_bytes'];rows[row['header']]=raw
 return rows
for label in ['after-4','after-8','compact-4','compact-8','reuse-4']:
 a,b=functions(old/label/'build'),functions(new/label/'build');assert a.keys()==b.keys();changed=[k for k in a if a[k]!=b[k]];query_candidates=[k for k in a if len(a[k])==28448];assert len(query_candidates)==1;query=query_candidates[0];assert len(changed)==8 and a[query]==b[query] and len(a[query])==28448
 out[label]=dict(functions_compared=len(a),changed_headers=changed,query_header=query,query_bytes=28448,query_byte_identical=True,query_sha256=hashlib.sha256(a[query]).hexdigest(),native_text_saved_bytes=sum(len(a[k])-len(b[k])for k in a),functions={str(k):dict(before_bytes=len(a[k]),after_bytes=len(b[k]),before_sha256=hashlib.sha256(a[k]).hexdigest(),after_sha256=hashlib.sha256(b[k]).hexdigest())for k in a})
report=dict(schema='mech.related-post-gen.per-function-text-correspondence.v1',before_compiler='cbe5b47839b4fe2300fe16ebc1fcacfcfa03f98b',after_compiler='67bdc1a3ec9e28accb2fd968f4292ca73d81b542',functions_compared=sum(x['functions_compared']for x in out.values()),all_five_query_functions_byte_identical=True,variants=out)
p=R/'results/native-local-facts-function-text-correspondence-20261107.json';p.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(status='verified',functions=report['functions_compared'],query_bodies_byte_identical=5)))
