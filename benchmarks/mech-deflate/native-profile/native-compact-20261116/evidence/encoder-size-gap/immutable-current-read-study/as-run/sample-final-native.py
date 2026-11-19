"""Sample the unchanged current compiler after encoding; retain native/exit boundary honestly."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess
ROOT=Path(__file__).resolve().parent;L=ROOT/'placement';read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();O=L/'native-sample';O.mkdir();r=read(L/'immutable-profiled/receipt.json');front=O/'frontend';command=r['compile'].copy();command[2]=str(front);env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|r['flags']
with (O/'compile.log').open('w') as log:
 child=subprocess.Popen(command,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  while True:
   line=child.stderr.readline();assert line,('compiler exited before encode boundary',child.poll());log.write(line);log.flush()
   try:phase=json.loads(line).get('phase')
   except (ValueError,AttributeError):phase=None
   if phase=='encode':break
  sample=['/usr/bin/sample',str(child.pid),'3','1','-file',str(O/'sample.txt')]
  with (O/'sample.log').open('w') as stream:observed=subprocess.run(sample,stdout=stream,stderr=stream)
  stdout,stderr=child.communicate(timeout=30);log.write(stderr);assert child.returncode==0;assert (O/'sample.txt').exists()
 finally:
  if child.poll() is None:child.terminate();child.wait(timeout=10)
actual={n:sha(front/n) for n in r['artifacts']};assert actual==r['artifacts'];record={'command':command,'compiler_sha256':sha(command[0]),'flags':r['flags'],'artifacts':actual,'sample_command':sample,'sample_exit_code':observed.returncode,'sample_sha256':sha(O/'sample.txt'),'scope':'Untimed sampling begins after the encode-phase record and continues through native compilation and process exit. Native work, output serialization and destruction can appear; exclusive native elapsed-time shares are not inferred from this single capture. Original emitted artifacts reproduce exactly.'};(O/'result.json').write_text(json.dumps(record,indent=2)+'\n');print('Captured current native compiler with identical object, metadata and bytecode.',flush=True)
