from pathlib import Path
import hashlib,json,subprocess,time

P=Path('/private/tmp/mech-related-selected-ranges-borrowed-json-runners-20261107-v1')
W=P/'escaped-field-checks';W.mkdir()
cases=[]
for shape in ['ordered','unordered','duplicates','empty']:
 posts=[]
 for i in range(9):
  tags=([f't{n}'for n in range(i%4+1)] if shape=='ordered' else [f't{(i+3)%5}',f't{i%5}'] if shape=='unordered' else ['same','same',f't{i%3}'] if shape=='duplicates' else [])
  posts.append(dict(_id=f'row-{i}-"\\\n-é',title=f'title-{i}\n"\\\tΩ😀',tags=tags))
 cases.append((shape,posts))
results=[]
for epoch,base in [(242,Path('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1/reuse-4')),
                   (252,Path('/private/tmp/mech-related-selected-range-integration-runners-20261107-v1/reuse-4'))]:
 for label,posts in cases:
  output_by_encoding={}
  for ascii_escape in [False,True]:
   case=W/f'{epoch}-{label}-{ascii_escape}';case.mkdir();input_path=case/'input.json';input_path.write_text(json.dumps(posts,ensure_ascii=ascii_escape))
   outputs=[]
   for host,exe in [('owned',base/'runner-aot'),('borrowed',P/str(epoch)/'runner-aot')]:
    target=case/(host+'.json');cmd=[str(exe),str(base/'build/source.mec'),str(input_path),str(target),'1'];start=time.monotonic();x=subprocess.run(cmd,capture_output=True,text=True)
    (case/(host+'-run.json')).write_text(json.dumps(dict(command=cmd,exit=x.returncode,seconds=time.monotonic()-start,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');assert x.returncode==0,x.stderr
    outputs.append(json.loads(target.read_text()))
   assert outputs[0]==outputs[1],(epoch,label,ascii_escape)
   output_by_encoding[ascii_escape]=outputs[1]
  assert output_by_encoding[False]==output_by_encoding[True]
  results.append(dict(compiler_epoch=epoch,case=label,owned_borrowed_full_JSON_equal=True,literal_unicode_escaped_unicode_equal=True,escapes_in_both_ID_title=True))
(W/'summary.json').write_text(json.dumps(dict(cases=results,fresh_processes=32,all_outputs_exact=True,scope='Host IO differential check against the unchanged owned-string adapter; ordinary Rust does not accept all escaped borrowed strings, so it is not used as this extended-case oracle. All normal31 edge cases separately match Rust.',timings_excluded=True),indent=2)+'\n')
print('32 extended escaped-field processes exact; both owned and borrowed-fallback semantics preserved')
