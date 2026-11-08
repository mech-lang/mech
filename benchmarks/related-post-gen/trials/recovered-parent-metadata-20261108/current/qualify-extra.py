from pathlib import Path
import json,os,subprocess,time,hashlib
P=Path(__file__).parent;c=json.loads((P/'latency-config.json').read_text());names=['mech-ir282-borrowed-parent-4', 'mech-ir282-both-rows-borrowed-parent-4'];variants={v['name']:v for v in c['implementations']};refs=[]
for case,vocab,reverse in [('sorted64',64,False),('unordered64',64,True),('fallback65',65,False)]:
 root=P/'extra-edges'/case;root.mkdir(parents=True,exist_ok=True);posts=[{'_id':f'p{i:02}','title':f'Post {i}','tags':sorted([f't{(i+j)%vocab:02}'for j in range(8)],reverse=reverse)}for i in range(vocab)];inp=root/'posts.json';inp.write_text(json.dumps(posts)+'\n');oracle=root/'oracle';oracle.mkdir(exist_ok=True)
 r=subprocess.run(['/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust'],cwd=oracle,capture_output=True,text=True);assert r.returncode==0,r.stderr;expected=json.loads((root/'related_posts_rust.json').read_text())
 for name in names:
  v=variants[name];work=root/name;work.mkdir(exist_ok=True)
  if v['kind']=='mech':out=work/'output.json';cmd=[v['executable'],v['source'],str(inp),str(out),'1']
  else:out=root/v['output'];cmd=[v['executable']]
  t=time.monotonic();r=subprocess.run(cmd,cwd=work,capture_output=True,text=True);q=dict(case=case,variant=name,command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,all_timing_fields_ineligible=True);assert r.returncode==0,r.stderr;assert json.loads(out.read_text())==expected;q.update(exact_Rust_output=True,output_sha256=hashlib.sha256(out.read_bytes()).hexdigest());(work/'run.json').write_text(json.dumps(q,indent=2)+'\n');refs.append(q);print(case,name,'exact',flush=True)
(P/'extra-qualification.json').write_text(json.dumps(dict(extra_edge_outputs_exact=3,prior_qualification_outputs_exact=32,variants=names,total_extra_outputs_exact=len(refs),total_qualification_outputs_exact=35*len(names),timing_claim=False),indent=2)+'\n')
