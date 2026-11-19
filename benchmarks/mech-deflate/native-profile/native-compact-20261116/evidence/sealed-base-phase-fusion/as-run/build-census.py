from pathlib import Path
from hashlib import sha256
import json,os,re,shutil,subprocess,sys
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');L=Path(__file__).resolve().parent;Q=L.parent.parent
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
base=read(Q/'current-roadmap/sealed-literal-batch/longer-statements/candidate/build.json');source=W/'src/syntax/src';diag=L/'diagnostic-source';shutil.copytree(source,diag)
cont=diag/'document/parser/canonical/base/continuation.rs';rec=diag/'document/parser/canonical/base/recognition.rs';text=cont.read_text();enum=text.split('enum Frame {',1)[1].split('\n}',1)[0];variants=[line.strip().split('(')[0].rstrip(',') for line in enum.splitlines() if line.strip()];assert len(variants)>20
module=r'''//! Diagnostic-only census. Never use this library for latency or deployment.
extern crate std;
use std::{cell::RefCell,time::Instant};
use crate::document::RuleId;
pub(crate) type Clock=(Instant,u64);
#[derive(Clone,Copy)]
struct Row{started:u64,completed:u64,success:u64,fresh:u64,eligible:u64,hits:u64,bytes:u64,work:u64,charges:u64,hit_work:u64,timed:u64,suspended:u64,inclusive_ns:u64,eligible_ns:u64}
impl Row{const ZERO:Self=Self{started:0,completed:0,success:0,fresh:0,eligible:0,hits:0,bytes:0,work:0,charges:0,hit_work:0,timed:0,suspended:0,inclusive_ns:0,eligible_ns:0};}
struct State{rows:[Row;super::super::NON_FIXED_RULES.len()],generation:u64,frames:[u64;FRAME_NAMES.len()]}
const FRAME_NAMES:&[&str]=__FRAME_NAMES__;
impl State{const fn new()->Self{Self{rows:[Row::ZERO;super::super::NON_FIXED_RULES.len()],generation:0,frames:[0;FRAME_NAMES.len()]}}}
impl Drop for State{fn drop(&mut self){
 for (i,r) in self.rows.iter().enumerate(){if r.started+r.hits==0{continue}std::eprintln!("sealed-base-census {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",super::super::NON_FIXED_RULES[i].0,r.started,r.completed,r.success,r.fresh,r.eligible,r.hits,r.bytes,r.work,r.charges,r.hit_work,r.timed,r.suspended,r.inclusive_ns,r.eligible_ns);}
 for (i,n) in self.frames.iter().enumerate(){std::eprintln!("sealed-base-frame {} {}",FRAME_NAMES[i],n);}
}}
std::thread_local!{static COUNTERS:RefCell<State>=const{RefCell::new(State::new())};}
pub(crate) fn advance(){COUNTERS.with(|c|c.borrow_mut().generation+=1);}
pub(crate) fn frame(kind:usize){COUNTERS.with(|c|c.borrow_mut().frames[kind]+=1);}
pub(crate) fn begin(rule:RuleId)->Clock{let now=Instant::now();let epoch=COUNTERS.with(|c|{let mut c=c.borrow_mut();let i=super::super::non_fixed_index(rule).unwrap();c.rows[i].started+=1;c.generation});(now,epoch)}
pub(crate) fn finished(rule:RuleId,clock:Clock,success:bool,fresh:bool)->Option<u64>{let elapsed=clock.0.elapsed().as_nanos() as u64;COUNTERS.with(|c|{let mut c=c.borrow_mut();let same=c.generation==clock.1;let r=&mut c.rows[super::super::non_fixed_index(rule).unwrap()];r.completed+=1;r.success+=u64::from(success);r.fresh+=u64::from(success&&fresh);if same{r.timed+=1;r.inclusive_ns+=elapsed;Some(elapsed)}else{r.suspended+=1;None}})}
pub(crate) fn eligible(rule:RuleId,bytes:u64,work:u64,charges:u64,elapsed:Option<u64>){COUNTERS.with(|c|{let mut c=c.borrow_mut();let r=&mut c.rows[super::super::non_fixed_index(rule).unwrap()];r.eligible+=1;r.bytes+=bytes;r.work+=work;r.charges+=charges;r.eligible_ns+=elapsed.unwrap_or(0);});}
pub(crate) fn replay(rule:RuleId,work:u64){COUNTERS.with(|c|{let mut c=c.borrow_mut();let r=&mut c.rows[super::super::non_fixed_index(rule).unwrap()];r.hits+=1;r.hit_work+=work;});}
'''.replace('__FRAME_NAMES__','&['+','.join(json.dumps(v) for v in variants)+']')
folder=rec.parent/'recognition';folder.mkdir(exist_ok=True);(folder/'census.rs').write_text(module)
s=rec.read_text();s=s.replace('const CAPACITY: usize = 1024;','pub(super) mod census;\n\nconst CAPACITY: usize = 1024;',1);s=s.replace('pub(super) struct Capture {','pub(super) struct Capture {\n    probe_clock:census::Clock,',1)
needle='''        Self {
            key: Key::new(p, rule),''';assert s.count(needle)==1;s=s.replace(needle,'''        Self {
            probe_clock:census::begin(rule),
            key: Key::new(p, rule),''')
needle='''        let start = self.checkpoint;
        let end = p.offset();''';assert s.count(needle)==1;s=s.replace(needle,needle+'''\n        let probe_elapsed=census::finished(self.key.rule,self.probe_clock,success,self.first_raw_fresh==Some(true));''')
needle='''        cache.slots[self.key.slot()] = Some(entry);''';assert s.count(needle)==1;s=s.replace(needle,'''        census::eligible(self.key.rule,count as u64,entry.work,entry.charges,probe_elapsed);
'''+needle)
needle='''        Some((entry.work, entry.raw))''';assert s.count(needle)==1;s=s.replace(needle,'''        census::replay(rule,entry.work);
'''+needle);rec.write_text(s)
needle='''    ) -> Progress {
        if let Some(rule) = self.initial.take() {''';assert text.count(needle)==1;text=text.replace(needle,'''    ) -> Progress {
        super::recognition::census::advance();
        if let Some(rule) = self.initial.take() {''')
needle='''            let frame = self.storage.frames.pop().expect("pending base phase");''';assert text.count(needle)==1;arms=[]
for i,line in enumerate(enum.splitlines()):
 pass
for i,name in enumerate(variants):
 original=next(line for line in enum.splitlines() if line.strip().startswith(name+ '(') or line.strip()==name+',');pattern='Frame::'+name+('(..)' if '(' in original else '');arms.append(pattern+'=>'+str(i))
text=text.replace(needle,needle+'\n            super::recognition::census::frame(match &frame{'+','.join(arms)+'});');cont.write_text(text)
features=base['compile'];env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|{'CARGO_MANIFEST_DIR':str(W/'src/syntax'),'CARGO_PKG_VERSION':'0.3.5'}
for role,path in [('control',source),('census',diag)]:
 out=L/role;out.mkdir();library=out/'libmech_syntax.rlib';cmd=features.copy();oldsrc=next(x for x in cmd if x.endswith('/src/syntax/src/lib.rs'));cmd[cmd.index(oldsrc)]=str(path/'lib.rs');cmd[-1]=str(library);cmd=[('metadata=sealed-base-'+role if x.startswith('metadata=') else x) for x in cmd]
 with (out/'build.log').open('w') as log:subprocess.run(cmd,env=env,cwd=W,stdout=log,stderr=log,check=True)
 link=read(L/'probe-build.json')['command'].copy();link=[('mech_syntax='+str(library) if x.startswith('mech_syntax=') else str(out/'parser') if x==str(L/'current-parser') else x) for x in link]
 with (out/'link.log').open('w') as log:subprocess.run(link,env=env,cwd=W,stdout=log,stderr=log,check=True)
 record={'source_commit':read(L/'plan.json')['source_commit'],'source_files':{str(p.relative_to(path)):sha(p) for p in path.rglob('*.rs')},'command':cmd,'link':link,'library_sha256':sha(library),'binary_sha256':sha(out/'parser'),'scope':'Standalone optimized parser. Census is diagnostic only; control is rebuilt from current source with the same dependencies.'};(out/'build.json').write_text(json.dumps(record,indent=2)+'\n');print('Built',role,flush=True)
 for n in range(2):
  with (out/f'run-{n}.log').open('w') as log:subprocess.run([str(out/'parser'),str(W/'benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec'),str(out/f'run-{n}.json')],env=env,stdout=log,stderr=log,check=True)
 print('Ran',role,flush=True)
