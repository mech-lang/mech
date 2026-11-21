from pathlib import Path
import json,hashlib,subprocess
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');Q=Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/consolidation-995');D=Q/'controller-source-boundaries/scoped-registers';T=D/'phase-tests';T.mkdir(exist_ok=False);sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
compiler=W/'hosts/gpu/src/native_artifact/compiler/mod.rs';s=compiler.read_text();a=s.index('pub(in crate::native_artifact) fn offset(');end=s.index('pub(in crate::native_artifact) fn guard(',a);offset=s[a:end]
source=r"""#![allow(dead_code)]
mod native_artifact {
 #[derive(Debug)] pub enum NativeCompileError {InvalidPlan{node:Option<usize>,reason:&'static str},Backend(String)}
 pub mod compiler {use super::NativeCompileError;pub fn backend(error:impl std::fmt::Display)->NativeCompileError {NativeCompileError::Backend(error.to_string())}
"""+offset+'}\n pub mod program {pub mod compact {\n'+'#[path='+json.dumps(str(W/'hosts/gpu/src/native_artifact/program/compact/phase_storage.rs'))+']mod phase_storage;\n}}}\n'
(T/'tests.rs').write_text(source)
libs=json.loads((D.parent/'owner-homes/word-tests/build.json').read_text())['dependencies']
for line in (D.parent/'build.jsonl').read_text().splitlines():
 try:j=json.loads(line)
 except ValueError:continue
 if j.get('reason')=='compiler-artifact' and j['target']['name']=='serde_json':
  path=next(Path(p) for p in j['filenames'] if p.endswith('.rlib'));libs['serde_json']={'path':str(path),'sha256':sha(path)}
command=['rustc','+nightly-2026-03-03','--edition=2024','--test','-C','opt-level=3','-C','codegen-units=16','--cfg','feature="early-region-emission"','-L','dependency='+str(Q/'target/release/deps'),str(T/'tests.rs'),'-o',str(T/'tests')]
for name,info in libs.items():
 assert sha(Path(info['path']))==info['sha256'];command+=['--extern',name+'='+info['path']]
with (T/'build.log').open('w') as log:r=subprocess.run(command,stdout=log,stderr=log)
record={'command':command,'dependencies':libs,'source_inputs':{p:sha(W/p) for p in ['hosts/gpu/src/native_artifact/program/compact/phase_storage.rs','hosts/gpu/src/native_artifact/program/compact/phase_storage_tests.rs']},'offset_span_sha256':hashlib.sha256(offset.encode()).hexdigest(),'exit_code':r.returncode,'scope':'Exact complete production phase_storage module and original/new JIT tests, with compiler error adapter and exact offset implementation. Does not replace coarse-unit boundary or full-worker qualification.'};(T/'build.json').write_text(json.dumps(record,indent=2)+'\n');assert r.returncode==0,(T/'build.log').read_text()
with (T/'tests.log').open('w') as log:r=subprocess.run([str(T/'tests'),'--test-threads=1'],stdout=log,stderr=log)
(T/'result.json').write_text(json.dumps({'exit_code':r.returncode,'binary_sha256':sha(T/'tests'),'output':(T/'tests.log').read_text()},indent=2)+'\n');print((T/'tests.log').read_text(),flush=True);assert r.returncode==0
