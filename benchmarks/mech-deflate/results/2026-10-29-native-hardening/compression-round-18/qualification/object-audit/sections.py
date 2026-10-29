from pathlib import Path
import hashlib,json,struct
root=Path(__file__).resolve().parent
before=Path('/private/tmp/mech-binding-descriptors-round17-20261029/object-audit/complete-object.o')
after=root/'complete-object.o'
def sections(path):
 data=path.read_bytes();assert struct.unpack_from('<I',data)[0]==0xfeedfacf
 ncmds=struct.unpack_from('<I',data,16)[0];at=32;out={}
 for _ in range(ncmds):
  cmd,size=struct.unpack_from('<II',data,at);assert size>=8 and at+size<=len(data)
  if cmd==0x19:
   count=struct.unpack_from('<I',data,at+64)[0];assert 72+80*count<=size
   for i in range(count):
    entry=struct.unpack_from('<16s16sQQIIIIIIII',data,at+72+80*i)
    section,segment=entry[0].rstrip(b'\0').decode(),entry[1].rstrip(b'\0').decode();length,offset,flags=entry[3],entry[4],entry[8]
    if flags&255 in [1,12,18]:continue
    assert offset+length<=len(data)
    payload=data[offset:offset+length];out[segment+','+section]={'bytes':length,'sha256':hashlib.sha256(payload).hexdigest(),'offset':offset}
  at+=size
 return out
b=sections(before);a=sections(after);assert b.keys()==a.keys()
result={'before':str(before),'after':str(after),'sections':{name:{'before':b[name],'after':a[name],'identical':b[name]['sha256']==a[name]['sha256']} for name in a}}
assert '__TEXT,__text' in result['sections'];(root/'sections.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result['sections']['__TEXT,__text']))
