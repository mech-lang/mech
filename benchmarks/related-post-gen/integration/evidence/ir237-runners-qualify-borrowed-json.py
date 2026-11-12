from pathlib import Path
import copy,json,subprocess,time,hashlib
U=Path(__file__).parent;P=U/'borrowed-reuse-4';O=Path('/private/tmp/mech-related-owned-shape-metadata-opt-20261107-v1');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
def run(c,p):
 c=list(map(str,c));t=time.monotonic();r=subprocess.run(c,capture_output=True,text=True);p.write_text(json.dumps(dict(command=c,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr;print(p.name,'passed',flush=True);return r
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'runner.rs','--out',P/'runner-aot','--bundle',P/'build','--linked'],P/'host-build-run.json')
run(['python3',R/'audit-native-link.py','--object',P/'build/object.o','--executable',P/'runner-aot','--output',P/'native-link-audit.json'],P/'audit-run.json')
run(['python3',R/'experiments/check_subset3.py','--runner',P/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',P/'build/source.mec','--out',P/'edges'],P/'edge-run.json')
posts=json.loads((R/'fixtures/posts-first-10.json').read_text());indices=json.loads((R/'fixtures/expected-first-10.json').read_text())['related_indices']
for i,post in enumerate(posts):
 post['_id']=f'escaped "id" {i} 🛰';post['title']=f'Title\\slash\nline\t{i} 界'
path=P/'escaped-id-title-posts.json';path.write_text(json.dumps(posts,ensure_ascii=True)+'\n');output=P/'escaped-id-title-output.json'
run([P/'runner-aot',P/'build/source.mec',path,output,'1'],P/'escaped-id-title-run.json')
expected=[{'_id':post['_id'],'tags':post['tags'],'related':[posts[i]for i in row]}for post,row in zip(posts,indices)];assert json.loads(output.read_text())==expected
(P/'escaped-id-title-expected.json').write_text(json.dumps(expected,ensure_ascii=False)+'\n')
config=json.loads((U/'ablation-comparison-config.json').read_text());v=copy.deepcopy(next(x for x in config['implementations']if x['name']=='mech-ir237-reuse-4'));v.update(name='mech-ir237-reuse-borrowed-json-4',category='IR237_reuse_borrowed_ID_title_JSON',executable=str(P/'runner-aot'),receipt=str(P/'runner-aot.build.json'));config['implementations'].append(v);assert len(config['implementations'])==12
config['matrix_rationale']+=' A borrowed-ID/title adapter is a separately labeled I/O ablation, including retained JSON input and owned escaped-string fallback.'
(U/'ablation-comparison-config.json').write_text(json.dumps(config,indent=2)+'\n')
run(['python3',R/'build-metrics.py','--config',U/'ablation-comparison-config.json','--out',U/'ablation-metrics','--variant','mech-ir237-reuse-borrowed-json-4'],P/'metrics-build-run.json')
print('Borrowed adapter:31 existing edge fixtures, escaped ID/title output, six ownership/error cases and instrumented runner complete',flush=True)
