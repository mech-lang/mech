from pathlib import Path
import os,json,gzip,hashlib,subprocess,shutil,importlib.util
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-native-vector-shape-identities-compiler-20261107');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');L=S/'benchmarks/related-post-gen/trials/native-vector-shape-identities-20261107';O=Path('/private/tmp/mech-related-native-vector-shape-identities-opt-20261107-v3');N=Path('/private/tmp/mech-related-native-vector-shape-identities-runners-20261107-v3')
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(Path(p).read_text())
def write(p,j):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(j,indent=2)+'\n')
r=read(P/'decision.json');assert r['source_commit']==subprocess.check_output(['git','-c','gc.auto=0','rev-parse','HEAD'],cwd=S,text=True).strip()
spec=importlib.util.spec_from_file_location('helper',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);assert h.source_inventory(S)==read(O/'source-inventory.json')['files']
L.mkdir(parents=True,exist_ok=True)
def copy(f,rel):
 f=Path(f);b=f.read_bytes()
 if f.suffix in ['.rs','.o','.bin','.mecb','.toml']or len(b)>16000:rel+='.gz';b=gzip.compress(b,mtime=0)
 p=L/rel;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
def tree(path,prefix):
 path=Path(path)
 for parent,dirs,files in os.walk(path):
  dirs[:]=sorted(n for n in dirs if n not in ['target','.git','__pycache__','edges','build','runtime'])
  for n in sorted(files):
   f=Path(parent)/n;rel=f.relative_to(path)
   if len(rel.parts)>3:continue
   if f.suffix in ['.py','.rs','.mec']or(f.suffix=='.json'and'output'not in n and not n.startswith('full5000.json')):copy(f,prefix+'/'+str(rel))
copy(P/'decision.json','report.json')
for v in [1,2,3]:
 tree(f'/private/tmp/mech-related-native-vector-shape-identities-opt-20261107-v{v}',f'build-v{v}')
 tree(f'/private/tmp/mech-related-native-vector-shape-identities-tests-20261107-v{v}',f'tests-v{v}')
for label,path in [('runners',N),('regressions','/private/tmp/mech-related-native-vector-shape-identities-regressions-20261107-v3'),('protocol',P)]:tree(path,label)
for f in (N/'build').iterdir():
 if f.is_file():copy(f,'Object/'+f.name)
for f in (N/'runtime').glob('*.json'):copy(f,'runtime/'+f.name)
copy(P/'first-incorrect-vector-eligibility-test.rs.gz','first-incorrect-vector-eligibility-test.rs.gz')
copy(P/'fixture-Object/metadata.json','generic-matrix-witness-metadata.json')
for v in [1,2,3]:
 p=Path(f'/private/tmp/mech-related-unused-source-functions-20261107-v{v}')
 tree(p,f'source-pruning/v{v}')
 for n in ['metadata.json','descriptor.json']:copy(p/'build'/n,f'source-pruning/v{v}/Object/'+n)
for n in ['check-unused-source-functions.py','check-unused-source-functions-v2.py','check-unused-source-functions-v3.py']:
 copy(Path('/private/tmp/mech-related-native-check-census-20261107-v1')/n,'source-pruning/recipes/'+n)
copy('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','inputs/posts.json')
copy('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json','outputs/Rust.json')
patch=subprocess.check_output(['git','-c','gc.auto=0','diff','eb5556fb58f760c347dc8465444d0e3fd8f3fc47',r['source_commit']],cwd=S);(L/'source.patch.gz').write_bytes(gzip.compress(patch,mtime=0))
(L/'README.md').write_text('# Rejected native vector shape identities\n\nIR267 is unselected. All benchmark native text is byte-exact to matched IR264, so no latency cohort is run and no speedup is claimed. Final qualification:400 actual native CI names, two native geometry tests, one general-schema COW/error/finite/retry test,32 benchmark outputs and133 exact scalar/DEFLATE non-timing/allocation rows. The first test incorrectly expected specialization for a general matrix schema; source metadata proves that program already lowers. A later broad run found12 panics because synthetic capture views have no selected-call witness. The final helper correctly declines the optional shape fact for those views; all400 final tests pass. All first failures are retained. Separate unused-source probes produce no native code change; one unused nominal declaration saves only about1KB artifact/3KB metadata. Those source variants are not output-qualified or adopted. LLVM remains optional; selected IR242 unchanged. Run python3 verify.py for packet integrity and qualification checks.\n')
verify='''from pathlib import Path
import gzip,hashlib,json,struct
P=Path(__file__).parent
def blob(name):
 p=P/name
 if not p.exists():p=Path(str(p)+'.gz')
 b=p.read_bytes();return gzip.decompress(b)if p.suffix=='.gz'else b
def read(name):return json.loads(blob(name))
m=read('manifest.json')
for n,i in m['files'].items():
 b=(P/n).read_bytes();assert len(b)==i['bytes']and hashlib.sha256(b).hexdigest()==i['sha256'],n
 b=gzip.decompress(b)if i['gzip']else b;assert len(b)==i['decoded_bytes']and hashlib.sha256(b).hexdigest()==i['decoded_sha256'],n
r=read('report.json');assert not r['selected']and not r['goal_complete']and not r['performance_measured']and r['production_runners_v3_same_native_text']
assert r['actual_CI']['actual_CI_passed']==400 and r['actual_CI']['actual_CI_names_exact']
ci=read('tests-v3/ci-execute.json');assert ci['exit']==0 and '400 passed; 0 failed'in ci['stdout']
for k in ['new-native-geometry-tests','new-native-composition-test','address-tests']:assert read('tests-v3/'+k+'.json')['exit']==0
for k in ['scalar','one-kib']:assert read('regressions/'+k+'-comparison.json')['all_nontiming_fields_equal_IR240']
assert read('runners/summary.json')['exact_outputs']==32 and read('runners/summary.json')['linked_audit_passed']
assert read('tests-v1/new-native-composition-test.json')['exit']!=0 and read('tests-v2/ci-execute.json')['exit']!=0
# Independently locate the actual Mach-O text, rather than trusting a report hash.
b=blob('Object/object.o');magic,cpu,subtype,ft,ncmd,sizecmd,flags,reserved=struct.unpack_from('<8I',b);assert magic==0xfeedfacf and cpu==0x100000c
at=32;found=[]
for _ in range(ncmd):
 cmd,size=struct.unpack_from('<II',b,at)
 if cmd==0x19:
  nsect=struct.unpack_from('<I',b,at+64)[0]
  for i in range(nsect):
   q=at+72+i*80;name=b[q:q+16].split(bytes([0]))[0]
   if name==b'__text':
    length=struct.unpack_from('<Q',b,q+40)[0];offset=struct.unpack_from('<I',b,q+48)[0];found.append(b[offset:offset+length])
 at+=size
assert len(found)==1 and hashlib.sha256(found[0]).hexdigest()==r['native_text_sha256']
print(json.dumps(dict(files=len(m['files']),native_CI=400,new_tests=3,benchmark_outputs=32,scalar_DEFLATE_rows=133,native_text_exact_parent=True,selected=False,timing_claim=False)))
'''
(L/'verify.py').write_text(verify)
files={}
for f in sorted(L.rglob('*')):
 if not f.is_file()or f.name=='manifest.json':continue
 b=f.read_bytes();d=gzip.decompress(b)if f.suffix=='.gz'else b;files[str(f.relative_to(L))]=dict(bytes=len(b),sha256=sha(b),gzip=f.suffix=='.gz',decoded_bytes=len(d),decoded_sha256=sha(d))
write(L/'manifest.json',dict(schema='mech.related-post-gen.evidence-packet.v1',source_commit=r['source_commit'],files=files));print('Packet',len(files),'records',sum(x['bytes']for x in files.values()),'bytes',flush=True)
