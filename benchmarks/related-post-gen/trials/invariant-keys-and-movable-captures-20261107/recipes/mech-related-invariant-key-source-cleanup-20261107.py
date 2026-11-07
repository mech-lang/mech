from pathlib import Path
import re,json,hashlib,subprocess,difflib
P=Path('/private/tmp/mech-related-invariant-key-source-cleanup-20261107-v1');P.mkdir(exist_ok=True)
B=Path('/private/tmp/mech-related-invariant-key-source-runners-20261107-v1/242-reads-4/build');s=(B/'source.mec').read_text();defs={}
for m in re.finditer(r'^([A-Za-z][A-Za-z0-9_-]*)\([^\n]*\)\s*=>[^\n]*:=\s*$',s,re.M):
 name=m.group(1);assert name not in defs;end=s.index('.\n',m.end())+2;defs[name]=(m.start(),end,s[m.start():end])
assert len(defs)>30
calls=lambda text:{name for name in defs if re.search(r'(?<![\w/-])'+re.escape(name)+r'\s*\(',re.sub(r'//[^\n]*','',text))}
root=s
for start,end,_ in sorted(defs.values(),reverse=True):root=root[:start]+root[end:]
live=calls(root)
while True:
 following=live|set().union(*(calls(defs[name][2])for name in live))
 if following==live:break
 live=following
removed=sorted(set(defs)-live);out=s
for name in sorted(removed,key=lambda n:defs[n][0],reverse=True):
 a,b,_=defs[name];out=out[:a]+out[b:]
(P/'source.mec').write_text(out);(P/'source.patch').write_text(''.join(difflib.unified_diff(s.splitlines(True),out.splitlines(True),fromfile='hoisted-ids-4.mec',tofile='reachable-functions-4.mec')))
r=subprocess.run(['/private/tmp/mech-related-conjoined-window-integration-opt-20261107-v1/compiler',str(P/'source.mec'),str(P/'build')],capture_output=True,text=True);assert r.returncode==0,r.stdout+r.stderr
(P/'compile-run.json').write_text(json.dumps(dict(exit=r.returncode,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
a=json.loads((B/'metadata.json').read_text());b=json.loads((P/'build/metadata.json').read_text())
report=dict(original_functions=len(defs),reachable_functions=len(live),removed_functions=removed,original_source_bytes=len(s.encode()),new_source_bytes=len(out.encode()),original_native_bytes=a['code_bytes'],new_native_bytes=b['code_bytes'],original_native_metadata_bytes=(B/'native-metadata.bin').stat().st_size,new_native_metadata_bytes=(P/'build/native-metadata.bin').stat().st_size,original_descriptor_bytes=(B/'descriptor.json').stat().st_size,new_descriptor_bytes=(P/'build/descriptor.json').stat().st_size,qualified=False,selected=False)
(P/'comparison.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
