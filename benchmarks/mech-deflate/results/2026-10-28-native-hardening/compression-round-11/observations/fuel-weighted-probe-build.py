from pathlib import Path
import hashlib,json,subprocess,time
root=Path(__file__).resolve().parent
w=Path('/private/tmp/mech-deflate-linear-stack')
s=(w/'hosts/gpu/tests/canonical_artifact_program.rs').read_text()
inputs=s[s.index('fn inputs('):s.index('fn reference(')]
limits=s[s.index('fn limits('):s.index('fn inputs(')]
r=(w/'hosts/gpu/tests/support/canonical_program_numeric_tail_resources.rs').read_text()
import re
weighted=json.loads(re.search(r'pub\(super\) const WEIGHTED: &str = (".*");', (w/'hosts/gpu/tests/support/canonical_program_numeric_tail.rs').read_text()).group(1))
weighted=weighted.replace('total<u64>) => (out<u64>)', 'total<u64>, expected<u64>) => (out<bool>)').replace('end, next) | false => total', 'end, next, expected) | false => total == expected').replace('seed<u64>)', 'seed<u64>, expected<u64>)')
source='    let source = '+json.dumps(weighted)+';' 
code='''use mech_core::{ManagedMemoryBudget,Value,ValueDataDraft,snapshot::{ValueDraft,SnapshotValidationContext}};
use mech_engine::{ProgramArtifact,CanonicalSourceFrontend};
use mech_gpu::native_artifact::{JitProgramArtifact,NativeExecutionLimits,NativeProgramExecutionLimits,emit_program_object};
use mech_syntax::document::{AstNode,DocumentId,Revision,TextSnapshot,ParseConfig,DocumentSyntax,parse_canonical_document};
'''+inputs+limits+'''fn main(){
'''+source+'''
let parsed=parse_canonical_document(TextSnapshot::new(DocumentId(42),Revision(1),source).unwrap(),ParseConfig::default());
let artifact=CanonicalSourceFrontend.compile_document_with_catalog(&DocumentSyntax::cast(parsed.syntax()).unwrap(),mech_stdlib::source_catalog()).unwrap().compile_artifact().unwrap();
let catalog=mech_stdlib::source_catalog();let jit=JitProgramArtifact::compile(&artifact,&catalog).unwrap();
let metadata=emit_program_object(&artifact,&catalog).unwrap().metadata();
let cost=metadata["readonly_numeric_regions"][0]["compute_per_traversal"].as_u64().unwrap();
let memory=ManagedMemoryBudget::new(256*1024*1024);let mut output=vec![None];
let mut policy=limits();policy.execution.max_work=1_000_000_000_000;policy.library.max_compute_work=Some(1_000_000_000_000);
for steps in [0,1,8,64] {
 let args=inputs(&artifact,&[("data",ValueDataDraft::Matrix(vec![ValueDataDraft::U32(3);256].into())),("coefficients",ValueDataDraft::Matrix(vec![ValueDataDraft::U8(2);256].into())),("seed",ValueDataDraft::U64(0)),("steps",ValueDataDraft::U64(steps)),("expected",ValueDataDraft::U64(steps*6))],&memory);
 jit.evaluate_into(&args,&mut output,policy,&memory).unwrap();
 let full=jit.evaluate_into(&args,&mut output,policy,&memory).unwrap();
 println!("{}",serde_json::json!({"steps":steps,"full_work":full.work,"full_visits":full.visits,"body_cost":cost}));
 for quarter in -4i64..=8 {
  let limit=(full.work as i64+quarter*cost as i64/4) as u64; let mut bounded=policy; bounded.execution.max_work=limit;
  let prior=output.clone();let before=(memory.used_bytes(),memory.retained_snapshot_nodes());
  let run=jit.evaluate_into(&args,&mut output,bounded,&memory);
  if run.is_err(){assert_eq!(before,(memory.used_bytes(),memory.retained_snapshot_nodes()));assert!(output[0].as_ref().unwrap().shares_frozen_storage(prior[0].as_ref().unwrap()));}
  else {assert_eq!(output[0].as_ref().unwrap().canonical_data_draft().unwrap(),ValueDataDraft::Bool(true));}
  println!("{}",serde_json::json!({"steps":steps,"quarter_delta":quarter,"limit":limit,"result":format!("{run:?}")}));
 }
}
drop(output);assert_eq!(memory.used_bytes(),0);assert_eq!(memory.retained_snapshot_nodes(),0);
}
'''
p=root/'fuel-weighted-probe.rs';p.write_text(code)
b=json.loads((root/'qualified-supervision/current-build.json').read_text());cmd=b['command'][:-3]
i=cmd.index('-C',cmd.index('-C')+2);del cmd[i:i+2]
cmd += [str(p),'-o',str(root/'fuel-weighted-probe')]
a=time.monotonic();res=subprocess.run(cmd,capture_output=True,text=True)
(root/'fuel-weighted-probe-build.log').write_text(res.stdout+res.stderr)
(root/'fuel-weighted-probe-build.json').write_text(json.dumps({'command':cmd,'exit':res.returncode,'seconds':time.monotonic()-a,'source_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'libraries':b['libraries']},indent=2)+'\n');res.check_returncode()
res=subprocess.run([str(root/'fuel-weighted-probe')],capture_output=True,text=True)
(root/'fuel-weighted-probe.stdout').write_text(res.stdout);(root/'fuel-weighted-probe.stderr').write_text(res.stderr);print(res.stderr);res.check_returncode()
