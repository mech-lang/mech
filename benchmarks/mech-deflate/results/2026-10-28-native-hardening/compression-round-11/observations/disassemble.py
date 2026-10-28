from pathlib import Path
import gzip,json,hashlib,subprocess,time,collections,re
r=Path(__file__).resolve().parent
p=r/'complete-object.o';command=['otool','-tvV',str(p)]
start=time.monotonic();out=subprocess.run(command,capture_output=True,text=True);out.check_returncode()
(r/'complete-disassembly.txt.gz').write_bytes(gzip.compress(out.stdout.encode(),mtime=0))
lines=out.stdout.splitlines();chosen=[];last=-1000
for i,line in enumerate(lines):
 if '\tudiv\t' in line and i-last>60:
  nearby=lines[max(0,i-15):i+45]
  if any(re.search(r'\t(ldrb|ldrh)\t|\tldr\tw', l) for l in nearby):
   chosen += [f'// full disassembly lines {max(0,i-15)+1}..{i+45}']+nearby
   last=i
   if len(chosen)>300:break
assert chosen
(r/'typed-load-backedge-excerpts.txt').write_text('\n'.join(chosen)+'\n')
(r/'disassembly.json').write_text(json.dumps({'command':command,'exit':out.returncode,'seconds':time.monotonic()-start,'object_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'lines':len(lines),'full_text_sha256':hashlib.sha256(out.stdout.encode()).hexdigest(),'excerpts':'contexts around unsigned quotient geometry and U8/U16/U32 loads; no source-name selection'},indent=2)+'\n')
print(len(lines),len(chosen))
