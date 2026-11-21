from pathlib import Path
import json,os,subprocess,hashlib
D=Path(__file__).resolve().parent;S=D/'scoped-registers';read=lambda p:json.loads(p.read_text());sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();O=D/'source-replay';O.mkdir(exist_ok=False);records=[]
for label,binary,case in [('minimal',read(D/'minimal-integration/build.json')['compiler'],'selected-map'),('experimental-control',read(S/'build.json')['compiler'],'selected-map'),('experimental-unit',read(S/'build.json')['compiler'],'early-owner-output-root-preserved')]:
 recipe=read(S/case/'result.json');flags=recipe['flags'].copy();flags.pop('MECH_COMPACT_SOURCE_BLOCK_REPORT',None)
 for mode in ['1','checked']:
  folder=O/(label+'-'+mode);folder.mkdir();command=recipe['command'].copy();command[0]=binary;command[2]=str(folder/'frontend')
  with (folder/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':mode},stdout=log,stderr=log,check=True)
  actual={name:sha(folder/'frontend'/name) for name in recipe['artifacts']};assert actual==recipe['artifacts'];records.append({'label':label,'mode':mode,'compiler_sha256':sha(binary),'command':command,'flags':flags|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':mode},'artifacts':actual});print(label,mode,'all artifact identities reproduce',flush=True)
(O/'result.json').write_text(json.dumps(records,indent=2)+'\n')
