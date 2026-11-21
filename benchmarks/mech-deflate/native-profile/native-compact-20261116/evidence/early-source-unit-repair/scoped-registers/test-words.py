from pathlib import Path
import hashlib,json,subprocess,time
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');Q=Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/consolidation-995');D=Q/'controller-source-boundaries/scoped-registers';T=D/'word-tests';T.mkdir(exist_ok=False)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
common=W/'hosts/gpu/src/native_artifact/codegen/common';compiler=W/'hosts/gpu/src/native_artifact/compiler/mod.rs';s=compiler.read_text();a=s.index('pub(in crate::native_artifact) fn offset(');end=s.index('pub(in crate::native_artifact) fn guard(',a);offset=s[a:end]
adapter=r"""#![allow(dead_code)]
mod native_artifact {
 #[derive(Debug)] pub enum NativeCompileError { InvalidPlan { node:Option<usize>, reason:&'static str }, Backend(String) }
 pub mod compiler { use super::NativeCompileError;
  pub fn backend(error:impl std::fmt::Display)->NativeCompileError {NativeCompileError::Backend(error.to_string())}
"""+offset+r""" }
 pub mod codegen {
  pub mod control { pub mod plan {
   #[derive(Clone,Copy)] pub struct CodePlan<'a>(std::marker::PhantomData<&'a ()>);
   pub struct Slot; impl Slot {pub fn get(self)->u32 {panic!("constructor plan is outside test scope")}}
   impl CodePlan<'_> {
    pub fn constant_count(self)->usize {panic!("constructor plan is outside test scope")}
    pub fn constant(self,_:usize)->Result<(Slot,Option<u64>),crate::native_artifact::NativeCompileError> {panic!("constructor plan is outside test scope")}
   }
  }}
  pub mod common {
"""+'#[path='+json.dumps(str(common/'ssa_words.rs'))+'] mod ssa_words;\n'+'}}}\n'
(T/'tests.rs').write_text(adapter)
libs=json.loads((Q/'collector-read-lifetimes/focused-tests/build.json').read_text())['dependencies']
for line in (D.parent/'build.jsonl').read_text().splitlines():
 try:j=json.loads(line)
 except ValueError:continue
 if j.get('reason')=='compiler-artifact' and j['target']['name']=='serde':
  path=next(Path(p) for p in j['filenames'] if p.endswith('.rlib'));libs['serde']={'path':str(path),'sha256':sha(path)}
command=['rustc','+nightly-2026-03-03','--edition=2024','--test','-C','opt-level=3','-C','codegen-units=16','--cfg','feature="jit"','--cfg','feature="early-region-emission"','--cfg','feature="canonical-native-program"','-L','dependency='+str(Q/'target/release/deps'),str(T/'tests.rs'),'-o',str(T/'tests')]
for name,info in libs.items():
 assert sha(Path(info['path']))==info['sha256'];command+=['--extern',name+'='+info['path']]
inputs={str(p.relative_to(W)):sha(p) for p in [common/'ssa_words.rs',common/'ssa_words_tests.rs',common/'ssa_direct_homes.rs',common/'ssa_packet_homes_tests.rs',compiler]}
t=time.perf_counter()
with (T/'build.log').open('w') as log:r=subprocess.run(command,cwd=W,stdout=log,stderr=log)
record={'command':command,'source_inputs':inputs,'offset_span_sha256':hashlib.sha256(offset.encode()).hexdigest(),'dependencies':libs,'exit_code':r.returncode,'diagnostic_seconds':time.perf_counter()-t,'scope':'Compiles the complete production Words module, direct-home implementation and their original tests, plus production packet JIT tests. The error adapter preserves the offset rejection. Constructor-only CodePlan is an unexecuted panic adapter; no constructor/plan proof is inferred. This does not replace actual original-codec output/error/recovery/stack checks.'};(T/'build.json').write_text(json.dumps(record,indent=2)+'\n')
assert r.returncode==0,(T/'build.log').read_text()
with (T/'tests.log').open('w') as log:r=subprocess.run([str(T/'tests'),'--test-threads=1'],stdout=log,stderr=log)
(T/'result.json').write_text(json.dumps({'exit_code':r.returncode,'test_binary_sha256':sha(T/'tests'),'output':(T/'tests.log').read_text()},indent=2)+'\n');print((T/'tests.log').read_text(),flush=True);assert r.returncode==0
