"""Execute the production literal-setup pass and its Apple AArch64 ABI oracle."""
from pathlib import Path
from hashlib import sha256
import argparse,json,os,platform,subprocess,sys,time

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--compiler-build-log',required=True,type=Path,help='Cargo --message-format=json output from a compile_profile build')
parser.add_argument('--output',required=True,type=Path,help='A new output directory')
parser.add_argument('--source-root',type=Path)
args=parser.parse_args()
if sys.platform!='darwin' or platform.machine()!='arm64':parser.error('This oracle executes Apple AArch64 machine code')
here=Path(__file__).resolve().parent
root=args.source_root
if root is None:root=next((p for p in here.parents if (p/'hosts/gpu/src/native_artifact/program/compact/allocation_stubs.rs').exists()),None)
if root is None:parser.error('Cannot locate the compiler source; supply --source-root')
root=root.resolve();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
source=root/'hosts/gpu/src/native_artifact/program/compact/allocation_stubs.rs'
(out/'generator.rs').write_text((here/'generator.rs').read_text().replace('__ALLOCATION_SETUP_SOURCE__',str(source)))
(out/'oracle.rs').write_text((here/'oracle.rs').read_text())
runtime=root/'hosts/gpu/src/native_artifact/compact_runtime/mod.rs'
(out/'runtime.rs').write_text('pub mod native_artifact { #[path='+json.dumps(str(runtime))+'] pub mod compact_runtime; }\n')
targets={}
for line in args.compiler_build_log.read_text().splitlines():
 try:record=json.loads(line)
 except json.JSONDecodeError:continue
 if record.get('reason')=='compiler-artifact':
  files=[p for p in record.get('filenames',[]) if p.endswith('.rlib')]
  if files:targets[record['target']['name']]=Path(files[0])
required=['cranelift_codegen','cranelift_frontend','cranelift_module','cranelift_object','cranelift_native']
for name in required:
 if name not in targets or not targets[name].exists():parser.error('Missing compiler dependency '+name+'; rebuild compile_profile and retain its Cargo JSON output')
env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')};env['MECH_INLINE_OWNER_RECORDS']='12'
commands=[]
def run(name,cmd):
 start=time.monotonic()
 with (out/(name+'.log')).open('w') as log:p=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
 commands.append({'name':name,'command':cmd,'seconds':time.monotonic()-start,'exit_code':p.returncode})
 (out/'commands.json').write_text(json.dumps(commands,indent=2)+'\n')
 if p.returncode:raise RuntimeError(name+' failed; see '+str(out/(name+'.log')))
rust=['rustc','+nightly-2026-03-03','--edition=2024','-D','warnings','-C','opt-level=2']
run('runtime-build',rust+['--crate-name','mech_gpu','--crate-type=rlib',str(out/'runtime.rs'),'-o',str(out/'libmech_gpu.rlib')])
cmd=rust+['-L','dependency='+str(targets['cranelift_codegen'].parent)]
for name in required:cmd+=['--extern',name+'='+str(targets[name])]
run('generator-build',cmd+[str(out/'generator.rs'),'-o',str(out/'generator')])
run('generator',[str(out/'generator'),str(out/'abi.o')])
run('oracle-build',rust+['--extern','mech_gpu='+str(out/'libmech_gpu.rlib'),'-C','link-arg='+str(out/'abi.o'),str(out/'oracle.rs'),'-o',str(out/'oracle')])
run('oracle',[str(out/'oracle')])
result={'status':'passed','production_source_sha256':sha256(source.read_bytes()).hexdigest(),'runtime_source_sha256':sha256(runtime.read_bytes()).hexdigest(),'generator':(out/'generator.log').read_text().strip(),'oracle':(out/'oracle.log').read_text().strip(),'compiler_dependencies':{n:{'path':str(targets[n]),'sha256':sha256(targets[n].read_bytes()).hexdigest()} for n in required}}
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(result['generator']);print(result['oracle'])
