"""Compile the production defaulted-load module and its JIT tests with exact production recognizer spans."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess,time
L=Path(__file__).resolve().parent;W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');BASE=W/'hosts/gpu/src/native_artifact/program/codegen/cfg_cleanup';O=L/'placement-focused-tests';O.mkdir(exist_ok=True);D=L.parent/'target/release/deps';sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();spans=[]
def span(name,start,end):
 p=BASE/name;s=p.read_text();a=s.index(start);b=s.index(end,a);part=s[a:b];spans.append({'source':str(p),'source_sha256':sha(p),'start_marker':start,'end_marker':end,'span_sha256':sha256(part.encode()).hexdigest()});return part
word=span('word_ranges/mod.rs','fn word_mask','#[cfg(test)]')
masked=span('boolean_conditions/mod.rs','pub(in super::super) fn masked_integer_comparison','fn literal')
comparison=span('window_bounds/mod.rs','fn predicate_definition','fn length_test')
view=span('composed_reads.rs','#[derive(Clone)]\npub(in crate::native_artifact) struct Literal','pub(in crate::native_artifact) struct Capture')
incoming=span('local_phis/mod.rs','pub(in crate::native_artifact::program) fn incoming','fn graph')
read=span('composed_reads.rs','#[derive(Clone)]\npub(super) struct Read','pub(super) fn between')
common='use cranelift_codegen::ir::{Block,Function,Inst,InstructionData as Data,Opcode,Opcode as Op,MemFlags,Value,ValueDef,Endianness,types,condcodes::IntCC};use cranelift_codegen::flowgraph::ControlFlowGraph;use std::collections::{BTreeMap,BTreeSet};'
source='''#![allow(dead_code,unused_imports)]
mod native_artifact {
 pub type NativeCompileError=String;
 pub mod compiler {pub fn backend(error:impl std::fmt::Display)->String {error.to_string()}}
 pub mod program {pub mod codegen {pub mod cfg_cleanup {
'''
source+=common+incoming+'\nuse incoming as incoming_block_arguments;\n'
source+='mod word_ranges{'+common+word+'}\n'
source+='mod boolean_conditions{use cranelift_codegen::ir::{Function,InstructionData as D,Opcode as O,Value,ValueDef,types};'+masked+'}\nuse boolean_conditions::masked_integer_comparison;\n'
source+='mod window_bounds{'+common+'use super::{masked_integer_comparison,word_ranges::literal};'+comparison+'}\n'
source+='mod composed_reads{'+common+'use std::sync::Arc;use super::{window_bounds::comparison,word_ranges::{constant_offset,literal}};'+view+read+'}\n'
source+='#[path='+json.dumps(str(BASE/'defaulted_loads.rs'))+'] mod defaulted_loads;\n'
source+='}}}}\n';probe=O/'tests.rs';probe.write_text(source)
libs={'cranelift_codegen':'d7c7fff1066f618e','cranelift_jit':'c2b908fd462bd708','cranelift_module':'66db0da08c7f38f6','cranelift_frontend':'058074d3f4503b68'}
command=['rustc','+nightly-2026-03-03','--edition=2024','--test','-C','opt-level=3','-C','codegen-units=16','-C','embed-bitcode=no','--cfg','feature="jit"','-L','dependency='+str(D),str(probe),'-o',str(O/'tests')]
for name,h in libs.items():command+=['--extern',name+'='+str(D/f'lib{name}-{h}.rlib')]
start=time.perf_counter()
with (O/'build.log').open('w') as log:r=subprocess.run(command,cwd=W,stdout=log,stderr=log)
record={'scope':'Focused test harness. Exact production defaulted_loads.rs is compiled including its tests; the surrounding recognizer functions are exact SHA-bound source spans, with only the error adapter represented by String. This does not substitute for private-loan contract or full-worker qualification.','command':command,'source_spans':spans,'production_module_sha256':sha(BASE/'defaulted_loads.rs'),'production_helper_sha256':sha(BASE/'defaulted_loads/immutable.rs'),'dependencies':{name:{'path':str(D/f'lib{name}-{h}.rlib'),'sha256':sha(D/f'lib{name}-{h}.rlib')} for name,h in libs.items()},'exit_code':r.returncode,'seconds_diagnostic':time.perf_counter()-start};(O/'build.json').write_text(json.dumps(record,indent=2)+'\n');assert r.returncode==0
args=[str(O/'tests'),'--test-threads=1'];start=time.perf_counter()
with (O/'tests.log').open('w') as log:r=subprocess.run(args,cwd=W,stdout=log,stderr=log)
(O/'result.json').write_text(json.dumps({'command':args,'exit_code':r.returncode,'binary_sha256':sha(O/'tests'),'seconds_diagnostic':time.perf_counter()-start,'output':(O/'tests.log').read_text()},indent=2)+'\n');assert r.returncode==0;print((O/'tests.log').read_text(),flush=True)
