from pathlib import Path
import re,json,hashlib,subprocess,time
P=Path('/private/tmp/mech-related-unused-source-functions-20261107-v3');P.mkdir(exist_ok=True)
base=Path('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4/build/source.mec');lines=base.read_text().splitlines(keepends=True)
functions={};occupied=set();i=0
while i<len(lines):
 match=re.match(r'^([a-z][A-Za-z0-9-]*|#[A-Za-z][A-Za-z0-9-]*)\([^\n]*=>',lines[i])
 if not match:i+=1;continue
 name=match[1];end=i+1
 while end<len(lines)and(not lines[end].strip()or lines[end][0].isspace()or lines[end].lstrip().startswith('//')or(name.startswith('#')and lines[end].startswith(name+'('))):end+=1
 assert name not in functions,name
 functions[name]=dict(start=i,end=end,text=''.join(lines[i:end]));occupied.update(range(i,end));i=end
clean=lambda s:re.sub(r'//[^\n]*','',s)
refs=lambda text:{n for n in functions if re.search(r'(?<![A-Za-z0-9_-])'+re.escape(n)+r'\s*\(',clean(text))}
root=''.join(line for i,line in enumerate(lines)if i not in occupied);reachable=refs(root);queue=list(reachable)
while queue:
 for name in refs(functions[queue.pop()]['text'])-reachable:reachable.add(name);queue.append(name)
removed=sorted(set(functions)-reachable);dead={i for name in removed for i in range(functions[name]['start'],functions[name]['end'])}
source=''.join(line for i,line in enumerate(lines)if i not in dead);(P/'source.mec').write_text(source)
report=dict(schema='mech.related-post-gen.unused-source-function-probe.v1',diagnostic_only=True,timing_claim=False,algorithm_unchanged_intent=True,source_sha256=hashlib.sha256(base.read_bytes()).hexdigest(),pruned_source_sha256=hashlib.sha256(source.encode()).hexdigest(),ordinary_and_FSM_functions=len(functions),removed_functions=removed,removed_lines=len(dead),conservative='Call-reference probe for ordinary/FSM declarations, avoiding same-named type aliases. Compilation must reject any undefined non-call function reference. Type declarations retained; no image bypass.')
(P/'source-analysis.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report),flush=True)
cmd=['/private/tmp/mech-related-native-check-census-20261107-v1/compiler',str(P/'source.mec'),str(P/'build')];t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True);(P/'emission.json').write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stdout[-3000:]+r.stderr[-3000:]
a=json.loads((base.parent/'metadata.json').read_text());b=json.loads((P/'build/metadata.json').read_text());print('native code/stack',[(j['code_bytes'],j['stack_bytes'])for j in [a,b]])
print('artifact bytes',[(p.stat().st_size)for p in [base.parent/'artifact.mecb',P/'build/artifact.mecb']]);print('metadata bytes',[(p.stat().st_size)for p in [base.parent/'native-metadata.bin',P/'build/native-metadata.bin']])
