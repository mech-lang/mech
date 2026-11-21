from pathlib import Path
from hashlib import sha256
import argparse,json,os,subprocess
parser=argparse.ArgumentParser(__doc__);parser.add_argument('compiler',type=Path);parser.add_argument('output',type=Path);parser.add_argument('--role',choices=['linear-4096-profiled', 'linear-65536-profiled', 'sqrt-65536-profiled', 'linear-65536-guided', 'sqrt-65536-guided'],default='linear-65536-profiled');parser.add_argument('--checked',action='store_true');args=parser.parse_args();here=Path(__file__).resolve().parent;out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);r=json.loads((here/'recipes.json').read_text())['roles'][args.role];sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();source=(here/r['source_file']).read_bytes();assert sha256(source).hexdigest()==r['source_sha256'];(out/'program.mec').write_bytes(source);flags=r['flags'].copy();entries={}
for profile in r['profiles']:
 path=here/profile['file'];assert sha(path)==profile['sha256'];entries.setdefault(profile['control'],[]).append(profile['symbol']+'@'+str(path))
flags|={k:';'.join(v) for k,v in entries.items()}
if r['sidecar']:flags['MECH_INSPECT_NUMERIC_SITE_BLOCKS']=str(out/'475-vector.json')
if r['raw_trace']:
 flags['MECH_INSPECT_PHASE_REGISTER_PROFILE']=str(out/'mech_compact_numeric_475_vector.vcode');flags['MECH_INSPECT_OPTIMIZED_PHASE_CLIF']=str(out/'mech_compact_numeric_475_vector.clif')
if args.checked:flags['MECH_COMPACT_PAIR_REGISTER_SPILLS']='checked'
cmd=[str(args.compiler.resolve()),str(out/'program.mec'),str(out/'frontend'),'compact']
with (out/'compile.log').open('w') as log:subprocess.run(cmd,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
artifacts={name:sha(out/'frontend'/name) for name in r['artifacts']};assert artifacts==r['artifacts'];result={'role':args.role,'checked':args.checked,'all_three_exact':True,'artifacts':artifacts,'source_sha256':sha(out/'program.mec'),'compiler_sha256':sha(args.compiler),'command':cmd,'flags':flags};(out/'reproduction.json').write_text(json.dumps(result,indent=2)+'\n');print(args.role,args.checked,'all three exact')
