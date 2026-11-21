"""Compare original and early-unit native status, source origin and outputs."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess,sys
ROOT=Path(__file__).resolve().parent
compiler=Path(sys.argv[1]).resolve()
root=ROOT/'pure-smoke';root.mkdir(exist_ok=True)
source=root/'source.mec'
source.write_text('x := x<u64>\ny := y<u64>\nquotient := x / y\nanswer := quotient + 7<u64>\nanswer\n')
clean={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
base=root/'original'
with (root/'original.log').open('w') as log:
 subprocess.run([str(compiler),str(source),str(base),'compact'],env=clean|{'MECH_COMPACT_ROUTE_DUMP':str(root/'source-cfg')},stdout=log,stderr=log,check=True)
cfg=json.loads((root/'source-cfg/source-cfg.json').read_text())
candidates=[s for s in cfg['sites'] if 'Div' in s['action']]
assert len(candidates)==1,candidates
site=candidates[0]['site']
assert cfg['sites'][site+1]['site']==site+1
window=f'{site}:{site+1}'
early=root/'early'
with (root/'early.log').open('w') as log:
 subprocess.run([str(compiler),str(source),str(early),'compact'],env=clean|{'MECH_COMPACT_EARLY_SOURCE_REGION':window},stdout=log,stderr=log,check=True)
for folder in [base,early]:
 meta=json.loads((folder/'metadata.json').read_text())
 assert meta['input_words']==2 and meta['output_words']==1,meta
harness=root/'worker.c'
harness.write_text("""#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <inttypes.h>
typedef struct { uint64_t status, origin; } Status;
extern Status mech_compact_closed_v1(const uint64_t*, uint64_t*);
int main(int argc, char** argv) {
 if (argc != 3) return 2;
 uint64_t inputs[2] = {strtoull(argv[1],0,0),strtoull(argv[2],0,0)};
 uint64_t out = UINT64_C(0xa5a5a5a5a5a5a5a5);
 Status s = mech_compact_closed_v1(inputs,&out);
 printf("%" PRIu64 " %" PRIu64 " %" PRIu64 "\\n",s.status,s.origin,out);
 return 0;
}
""")
for folder in [base,early]:
 subprocess.run(['clang','-O2',str(harness),str(folder/'program.o'),'-o',str(folder/'worker')],check=True)
rows=[]
for args in [(0,1),(1,1),(100,3),(99,0),(0,0),(2**64-1,1),(2**64-1,2),(2**63,3),(2**64-1,2**64-1)]:
 observed={label:subprocess.check_output([str(folder/'worker'),*map(str,args)],text=True).strip() for label,folder in [('original',base),('early',early)]}
 assert observed['original']==observed['early'],(args,observed)
 rows.append({'inputs':args,**observed})
report={'compiler_sha256':sha256(compiler.read_bytes()).hexdigest(),'source_window':window,'cases':rows,'early_unit':json.loads((early/'metadata.json').read_text())['early_source_region']}
(root/'result.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'source_window':window,'matched_cases':len(rows),'observations':rows},indent=2))
