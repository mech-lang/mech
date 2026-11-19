from pathlib import Path
import json,os,subprocess,hashlib,re
D=Path(__file__).resolve().parent/'scoped-registers';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();read=lambda p:json.loads(p.read_text());out=D/'compiler-profiles';out.mkdir(exist_ok=False);compiler=read(D/'build.json')['compiler'];roles={'current':'selected-map','early':'early-owner-output-root-preserved'};records=[]
for ordinal,role in enumerate(['current','early','early','current']):
 r=read(D/roles[role]/'result.json');folder=out/(str(ordinal)+'-'+role);folder.mkdir();flags=r['flags'].copy();flags.pop('MECH_COMPACT_SOURCE_BLOCK_REPORT',None);command=r['command'].copy();command[2]=str(folder/'frontend')
 with (folder/'compile.log').open('w') as log:
  child=subprocess.Popen(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  try:
   while True:
    line=child.stderr.readline();assert line,('exited before parse record',child.poll());log.write(line);log.flush()
    try:phase=json.loads(line).get('phase')
    except (ValueError,AttributeError):phase=None
    if phase=='parse':break
   sample=['/usr/bin/sample',str(child.pid),'4','1','-file',str(folder/'sample.txt')]
   with (folder/'sample.log').open('w') as stream:observed=subprocess.run(sample,stdout=stream,stderr=stream)
   stdout,stderr=child.communicate(timeout=30);log.write(stderr);assert child.returncode==0
  finally:
   if child.poll() is None:child.terminate();child.wait(timeout=10)
 actual={name:sha(folder/'frontend'/name) for name in r['artifacts']};assert actual==r['artifacts'];sample_text=(folder/'sample.txt').read_text();flat=sample_text.split('Sort by top of stack, same collapsed (when >= 5):')[-1].split('Binary Images:')[0];functions=[]
 for line in flat.splitlines():
  match=re.match(r'^\s+(.*?)\s+\(in (.*?)\)\s+(\d+)\s*$',line)
  if match:functions.append({'symbol':match[1],'image':match[2],'leaf_samples':int(match[3])})
 functions.sort(key=lambda x:-x['leaf_samples']);record={'role':role,'command':command,'flags':flags,'compiler_sha256':sha(compiler),'artifacts':actual,'sample_command':sample,'sample_exit_code':observed.returncode,'sample_sha256':sha(folder/'sample.txt'),'top_leaf_functions':functions,'scope':'Untimed sampling begins at the parse phase record, before source semantics and native planning. Semantics, artifact/encoding, all native compilation threads, waiting and process teardown may appear. Leaf counts and call ancestry are diagnostics, not exclusive elapsed phase timers. All emitted artifacts reproduce exactly.'};(folder/'result.json').write_text(json.dumps(record,indent=2)+'\n');records.append(record);print(role,'compiler profile captured, exact artifacts',flush=True)
(out/'summary.json').write_text(json.dumps({'captures':records},indent=2)+'\n')
