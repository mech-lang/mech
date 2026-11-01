"""Verify actual code bytes/stack sizes for the precise failure-return experiment."""
from pathlib import Path
import hashlib,json,struct,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'variants':{}}
def sections(blob):
 header=struct.unpack_from('<IiiIIIII',blob);assert header[0]==0xfeedfacf
 at=32;found={}
 for _ in range(header[4]):
  command,size=struct.unpack_from('<II',blob,at);assert size>=8
  if command==0x19:
   count=struct.unpack_from('<I',blob,at+64)[0]
   for n in range(count):
    x=struct.unpack_from('<16s16sQQIIIIIIII',blob,at+72+n*80);name=x[0].rstrip(b'\0').decode();segment=x[1].rstrip(b'\0').decode()
    if name=='__text' and segment=='__TEXT':found[name]={'address':x[2],'size':x[3],'offset':x[4]}
  at+=size
 assert '__text' in found;return found
for label in ['before','after']:
 audit=json.loads((r/'host-assembly'/label/'audit.json').read_text());binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());assert audit['worker_sha256']==binding['worker_sha256']
 # The byte slices are captured separately during live audit. Replay uses the
 # archived hashes/assembly and never needs the private executable.
 captured=json.loads((r/'machine-bytes'/(label+'.json')).read_text());assert captured['worker_sha256']==binding['worker_sha256']
 rows={}
 for needle in ['ordinary-budget','cold-debit-error']:
  raw=bytes.fromhex(captured['regions'][needle]['hex']);assert hashlib.sha256(raw).hexdigest()==captured['regions'][needle]['sha256'] and len(raw)==captured['regions'][needle]['bytes']
  region=next(x for x in audit['regions'] if x['needle']==needle);first=region['instructions'][0];frame=int(first['operands'].split('#')[1],16) if first['opcode']=='sub' and first['operands'].startswith('sp, sp, #0x') else 0
  tail=[x for x in region['instructions'] if 0x1b0<=x['address']-region['start']<=0x1d4] if needle=='ordinary-budget' else []
  if needle=='ordinary-budget':assert len(tail)==10 and frame==176
  rows[needle]={'symbol':region['symbol'],'own_static_instructions':region['static_instruction_count'],'own_stack_frame_bytes':frame,'opcode_operands':[(x['opcode'],x['operands']) for x in region['instructions']],'selected_success_tail_instructions':tail,'machine_bytes':captured['regions'][needle]}
 result['variants'][label]={'worker_sha256':binding['worker_sha256'],'compiler_source':binding['compiler_source'],'text':captured['text'],'regions':rows}
b=result['variants']['before'];a=result['variants']['after'];same={key:b['regions'][key]['opcode_operands']==a['regions'][key]['opcode_operands'] and b['regions'][key]['machine_bytes']==a['regions'][key]['machine_bytes'] for key in b['regions']};assert all(same.values())
result.update(selected_routines_machine_bytes_identical=same,whole_text_sha256_identical=b['text']['sha256']==a['text']['sha256'],success_tail_static_instructions={'before':10,'after':10},own_compute_frame_bytes={'before':176,'after':176},public_result_bytes=40,full_inactive_result_payload_still_transferred_on_success=True,scope='Actual selected linked routines have identical machine bytes, opcodes/operands, static instruction counts and frames. Raw section digests are captured from the private Mach-O files; replay verifies the archived extraction receipt and assembly without binaries. No further hot-path, code-size or stack improvement is established. Whole-call timing/counters remain separate observations, including contrary cycle/elapsed directions.')
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='variants'},indent=2))
