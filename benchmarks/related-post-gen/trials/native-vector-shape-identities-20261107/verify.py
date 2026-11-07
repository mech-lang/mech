from pathlib import Path
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
