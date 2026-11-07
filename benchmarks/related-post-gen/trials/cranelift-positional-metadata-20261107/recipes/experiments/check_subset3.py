#!/usr/bin/env python3
"""Exact original-Rust differential checks for the isolated subset-index source.

Executable times emitted by the adapter are diagnostic and are not comparisons.
Requires the original unchanged upstream Rust release executable and the linked
Mech adapter for related-subset3-linked.mec; creates only --out files.
"""
from pathlib import Path
import argparse,json,subprocess,hashlib,random
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--runner',type=Path,required=True)
parser.add_argument('--rust',type=Path,required=True)
parser.add_argument('--out',type=Path,required=True)
parser.add_argument('--source',type=Path,help='Variant source linked into --runner; default is the qualified subset3 source')
args=parser.parse_args()
root=Path(__file__).resolve().parents[1];out=args.out.resolve();out.mkdir(parents=True,exist_ok=True)
source=args.source.resolve() if args.source else root/'experiments/related-subset3-linked.mec';rows=[]
runner=args.runner.resolve()
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
rust=args.rust.resolve()
for name in ['first-10','first-100','first-500','unicode-ties','empty-tags']:
 posts=root/'fixtures'/('posts-'+name+'.json');expected=root/'fixtures'/('expected-'+name+'.json');actual=out/('actual-'+name+'.json')
 cmd=[str(runner),str(source),str(posts),str(actual),'1'];r=subprocess.run(cmd,capture_output=True,text=True,timeout=60)
 entry={'fixture':name,'exit':r.returncode,'command':cmd,'stdout':r.stdout,'stderr':r.stderr,'diagnostic_elapsed_excluded_from_benchmark':True}
 if r.returncode==0:
  data=json.loads(posts.read_text());got=json.loads(actual.read_text());want=json.loads(expected.read_text())['related_indices'];exact=got==[{'_id':p['_id'],'tags':p['tags'],'related':[data[i] for i in indices]} for p,indices in zip(data,want)];entry['exact_expected_output']=exact
  assert exact,(name,got[:1],want[:1])
 rows.append(entry);print(json.dumps({'fixture':name,'exit':r.returncode,'exact':entry.get('exact_expected_output')},ensure_ascii=False),flush=True)
 assert r.returncode==0,entry
cases={
'all-empty':[[] for _ in range(7)],
'one-post':[['alpha']],
'eight-tag-boundary':[['a','é','界','🛰','a-b','f','g','h'] for _ in range(9)],
'duplicate-utf8':[['é','é','界'] if i%2 else ['界','界','a'] for i in range(12)],
'wrapped-u8':[['same']*16 for _ in range(7)],
'wrap255':[['same']*n for n in [15,17,1,2,3,4,5]],
'wrap256':[['same']*n for n in [16,16,1,2,3,4,5]],
'wrap257':[['same']*n for n in [1,257,2,3,4,5,6]],
'long-nine-tags':[['tag'+str(k) for k in range(9)] for _ in range(9)],
'large-vocabulary':[['v'+str(i*3+k) for k in range(3)] for i in range(70)],
'mixed-full-sparse':[['a','é','界'] for _ in range(8)]+[['a'],['x'],[],['y']],
# Coverage proof needs five OTHER positive candidates, not five including self.
'coverage-five-with-two-tag-fifth':[['a','b','c'] for _ in range(5)]+[['a','b']],
'coverage-six-with-two-tag-extra':[['a','b','c'] for _ in range(6)]+[['a','b']],
'coverage-all-six':[['a','b','c'] for _ in range(6)],
'coverage-permuted-ties':[['c','a','b'],['b','c','a'],['a','c','b'],['c','b','a'],['b','a','c'],['a','b','c'],['a','b']],
'utf8-key-prefix':[['aé','a','aa','🛰'] if i%2 else ['aé','a','aa','é'] for i in range(10)],
'long-utf8-key-prefix':[['x'*65+'界','x'*65+'é','a','b'] for _ in range(8)],
'coverage-mixed-disjoint':[['a','b','c'] for _ in range(6)]+[['x','y','z'] for _ in range(5)]+[['x','y']],
}
rng=random.Random(712325)
for j in range(8):
 cases['random-'+str(j)]=[rng.sample(['t'+str(k) for k in range(14)],rng.randrange(0,9)) for i in range(rng.randrange(6,24))]
for name,tagsets in cases.items():
 case=out/'edges'/name; (case/'rust').mkdir(parents=True,exist_ok=True)
 data=[{'_id':name+'-'+str(i),'title':'Edge '+str(i),'tags':tags} for i,tags in enumerate(tagsets)]
 posts=case/'posts.json';posts.write_text(json.dumps(data,ensure_ascii=False)+'\n')
 rr=subprocess.run([str(rust)],cwd=case/'rust',capture_output=True,text=True,timeout=60);assert rr.returncode==0,(name,rr.stderr)
 actual=case/'actual.json';cmd=[str(runner),str(source),str(posts),str(actual),'1'];r=subprocess.run(cmd,capture_output=True,text=True,timeout=60)
 expected=case/'related_posts_rust.json';exact=r.returncode==0 and json.loads(actual.read_text())==json.loads(expected.read_text())
 entry={'fixture':name,'source_posts':str(posts),'source_posts_sha256':sha(posts),'expected':str(expected),'expected_sha256':sha(expected),'exit':r.returncode,'command':cmd,'stdout':r.stdout,'stderr':r.stderr,'exact_rust_output':exact,'diagnostic_elapsed_excluded_from_benchmark':True}
 rows.append(entry);print(json.dumps({'fixture':name,'exit':r.returncode,'exact':exact},ensure_ascii=False),flush=True)
 if not exact: print(r.stderr);(out/'edge-fixtures.json').write_text(json.dumps(rows,indent=2)+'\n');raise AssertionError(name)
(out/'edge-fixtures.json').write_text(json.dumps(rows,indent=2)+'\n')
print(json.dumps({'status':'passed','cases':len(rows),'timing_claim':False,'source_sha256':sha(source),'binary_sha256':sha(runner)}))
