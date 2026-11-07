from pathlib import Path
import json,hashlib,subprocess,time,shutil,difflib
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');U=Path('/private/tmp/mech-related-invariant-key-source-runners-20261107-v1');U.mkdir(exist_ok=True)
source=Path('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1/reuse-4/build/source.mec').read_text()
def replace_fn(text,name,body):
 start=text.index(name+'(');end=text.index('.\n',start)+2
 assert text[start:end].startswith(name+'(')
 return text[:start]+body.strip()+'\n'+text[end:]
# Every read is inside the productive child-call arm. Empty inner loops never
# gain a speculative source read; source-level changes keep ordinary checked ops.
read_hoist={
'subset-third':'''subset-third(ids<[u64]>, a<u64>, b<u64>, third<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := third < end
  next := (more? | true => sequence/append(output, subset-key(a, b, ids[third + 1u64])) | false => output)
  out := (more? | true => subset-third(ids, a, b, third + 1u64, end, next) | false => output).''',
'subset-second':'''subset-second(ids<[u64]>, a<u64>, second<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := second + 1u64 < end
  next := (more? | true => subset-third(ids, a, ids[second + 1u64], second + 1u64, end, output) | false => output)
  out := (more? | true => subset-second(ids, a, second + 1u64, end, next) | false => output).''',
'subset-first':'''subset-first(ids<[u64]>, first<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := first + 2u64 < end
  next := (more? | true => subset-second(ids, ids[first + 1u64], first + 1u64, end, output) | false => output)
  out := (more? | true => subset-first(ids, first + 1u64, end, next) | false => output).''',
'quartet-fourth':'''quartet-fourth(ids<[u64]>, a<u64>, b<u64>, c<u64>, fourth<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := fourth < end
  next := (more? | true => sequence/append(output, quartet-key(a, b, c, ids[fourth + 1u64])) | false => output)
  out := (more? | true => quartet-fourth(ids, a, b, c, fourth + 1u64, end, next) | false => output).''',
'quartet-third':'''quartet-third(ids<[u64]>, a<u64>, b<u64>, third<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := third + 1u64 < end
  next := (more? | true => quartet-fourth(ids, a, b, ids[third + 1u64], third + 1u64, end, output) | false => output)
  out := (more? | true => quartet-third(ids, a, b, third + 1u64, end, next) | false => output).''',
'quartet-second':'''quartet-second(ids<[u64]>, a<u64>, second<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := second + 2u64 < end
  next := (more? | true => quartet-third(ids, a, ids[second + 1u64], second + 1u64, end, output) | false => output)
  out := (more? | true => quartet-second(ids, a, second + 1u64, end, next) | false => output).''',
'quartet-first':'''quartet-first(ids<[u64]>, first<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := first + 3u64 < end
  next := (more? | true => quartet-second(ids, ids[first + 1u64], first + 1u64, end, output) | false => output)
  out := (more? | true => quartet-first(ids, first + 1u64, end, next) | false => output).'''}
rank_hoist={
'subset-key':'''subset-key(a<u64>, b<u64>, c<u64>) => (out<u64>) :=
  out := a + b * (b - 1u64) / 2u64 + c * (c - 1u64) * (c - 2u64) / 6u64.
subset-second-base(a<u64>, b<u64>) => (out<u64>) :=
  out := a + b * (b - 1u64) / 2u64.
subset-third-key(base<u64>, c<u64>) => (out<u64>) :=
  out := base + c * (c - 1u64) * (c - 2u64) / 6u64.''',
'subset-third':'''subset-third(ids<[u64]>, base<u64>, third<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := third < end
  next := (more? | true => sequence/append(output, subset-third-key(base, ids[third + 1u64])) | false => output)
  out := (more? | true => subset-third(ids, base, third + 1u64, end, next) | false => output).''',
'subset-second':'''subset-second(ids<[u64]>, a<u64>, second<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := second + 1u64 < end
  next := (more? | true => subset-third(ids, subset-second-base(a, ids[second + 1u64]), second + 1u64, end, output) | false => output)
  out := (more? | true => subset-second(ids, a, second + 1u64, end, next) | false => output).''',
'quartet-fourth':'''quartet-fourth(ids<[u64]>, base<u64>, fourth<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := fourth < end
  next := (more? | true => sequence/append(output, quartet-fourth-key(base, ids[fourth + 1u64])) | false => output)
  out := (more? | true => quartet-fourth(ids, base, fourth + 1u64, end, next) | false => output).''',
'quartet-key':'''quartet-key(a<u64>, b<u64>, c<u64>, d<u64>) => (out<u64>) :=
  out := a + b * (b - 1u64) / 2u64 + c * (c - 1u64) * (c - 2u64) / 6u64 + d * (d - 1u64) * (d - 2u64) * (d - 3u64) / 24u64.
quartet-fourth-key(base<u64>, d<u64>) => (out<u64>) :=
  out := base + d * (d - 1u64) * (d - 2u64) * (d - 3u64) / 24u64.''',
'quartet-third':'''quartet-third(ids<[u64]>, base<u64>, third<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := third + 1u64 < end
  next := (more? | true => quartet-fourth(ids, subset-third-key(base, ids[third + 1u64]), third + 1u64, end, output) | false => output)
  out := (more? | true => quartet-third(ids, base, third + 1u64, end, next) | false => output).''',
'quartet-second':'''quartet-second(ids<[u64]>, a<u64>, second<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := second + 2u64 < end
  next := (more? | true => quartet-third(ids, subset-second-base(a, ids[second + 1u64]), second + 1u64, end, output) | false => output)
  out := (more? | true => quartet-second(ids, a, second + 1u64, end, next) | false => output).'''}
# Replace definitions only, never a call occurrence. Definitions are at column0.
def apply(text,replacements):
 for name,body in replacements.items():
  token='\n'+name+'(';start=text.index(token)+1;end=text.index('.\n',start)+2
  text=text[:start]+body.strip()+'\n'+text[end:]
 return text
texts={'reads-4':apply(source,read_hoist)};texts['partial-4']=apply(texts['reads-4'],rank_hoist)
for name,text in texts.items():
 p=U/(name+'.mec');p.write_text(text)
 (U/(name+'.patch')).write_text(''.join(difflib.unified_diff(source.splitlines(True),text.splitlines(True),fromfile='selected-mask-reuse.mec',tofile=p.name)))
assert all(t.count('subset-third(')==3 for t in texts.values())
versions={242:Path('/private/tmp/mech-related-conjoined-window-integration-opt-20261107-v1'),247:Path('/private/tmp/mech-related-initialized-homes-integration-opt-20261107-v1')}
def run(c,p):
 c=list(map(str,c));start=time.monotonic();x=subprocess.run(c,capture_output=True,text=True);p.write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-start,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');assert x.returncode==0,x.stdout+x.stderr
for ir,O in versions.items():
 for name in texts:
  p=U/(str(ir)+'-'+name);p.mkdir(exist_ok=True)
  run([O/'compiler',U/(name+'.mec'),p/'build'],p/'compile-run.json')
  shutil.copyfile('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1/reuse-4/runner.rs',p/'runner.rs')
  run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
  run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
  run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',p/'edges'],p/'edge-run.json')
  m=json.loads((p/'build/metadata.json').read_text());print(ir,name,'31edges exact, code',m['code_bytes'],'stack',m['stack_bytes'],flush=True)
