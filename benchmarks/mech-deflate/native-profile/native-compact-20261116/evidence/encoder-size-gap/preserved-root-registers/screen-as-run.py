"""Compare the optional allocation preference with the exact qualified root."""
from pathlib import Path
from hashlib import sha256
from shutil import copy2
import json,os,struct,subprocess
lab=Path(__file__).resolve().parent
root=lab/'preserved-registers'
read=lambda p:json.loads(p.read_text())
save=lambda p,v:p.write_text(json.dumps(v,indent=2)+'\n')
sha=lambda p:sha256(p.read_bytes()).hexdigest()
base=read(root/'control-source/rebuild.json')
assert base['full_integrated_source_identity'] and base['object_text_bytes']==64444
binary=Path(base['phases']['source_to_native']['command'][0])
assert sha(binary)==base['compiler_sha256']
snapshot=lab/'reconsidered-size-compiler'/sha(binary)[:16];snapshot.mkdir(exist_ok=True)
compiler=snapshot/'compile_profile'
if not compiler.exists():copy2(binary,compiler)
assert sha(compiler)==base['compiler_sha256']
work=root/'preserved';work.mkdir()
output=work/'frontend';flags=base['native_flags']|{'MECH_COMPACT_PREFER_PRESERVED_REGS':'1'}
command=[str(compiler),base['phases']['source_to_native']['command'][1],str(output),'compact']
with (work/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
data=(output/'program.o').read_bytes();offset=32;size=None
for _ in range(struct.unpack_from('<I',data,16)[0]):
 kind,length=struct.unpack_from('<II',data,offset)
 if kind==0x19:
  for section in range(struct.unpack_from('<I',data,offset+64)[0]):
   at=offset+72+section*80
   if data[at:at+16].rstrip(b'\0')==b'__text':size=struct.unpack_from('<Q',data,at+40)[0]
 offset+=length
assert size is not None
metadata=read(output/'metadata.json')
record={'source_commit':base['source_commit'],'compiler_sha256':sha(compiler),'command':command,'flags':flags,'text_bytes':size,'root_bytes':metadata['code_bytes'],'root_stack_bytes':metadata['stack_bytes'],'object_sha256':sha(output/'program.o'),'metadata_sha256':sha(output/'metadata.json'),'bytecode_sha256':sha(output/'program.mecb'),'bytecode_unchanged':sha(output/'program.mecb')==base['bytecode_sha256'],'bytes_vs_qualified':size-64444,'runtime':'unqualified; requires complete checks and both-order runtime qualification'}
assert record['bytecode_unchanged']
save(work/'receipt.json',record)
print(json.dumps({k:v for k,v in record.items() if k not in ('flags','command')},indent=2),flush=True)
