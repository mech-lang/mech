use mech_core::{ValueData as V,ValueDataDraft as D,snapshot::{SnapshotValidationContext,ValueDraft}};
use mech_engine::CanonicalSourceFrontend;
use mech_gpu::native_artifact::{JitProgramArtifact,NativeProgramLimitMode};
use mech_syntax::document::{AstNode,DocumentId,DocumentSyntax,ParseConfig,Revision,TextSnapshot,parse_canonical_document};
const SOURCE:&str="+> sequence\npack(nodes<[u64]>, costs<[u32]>) => (out<{nodes<[u64]> costs<{literal<[u32]>}>}>) :=\n  out := {nodes: nodes, costs: {literal: costs}}.\nreturned := pack(sequence/with-capacity(0u64, capacity<u64>), [7u32])\nsequence/append(returned.nodes, 17u64)\n";
fn main(){
 let catalog=mech_stdlib::source_catalog();let mode=NativeProgramLimitMode::NoLimits;
 for (width,real)in[(64,305257u64),(8,6633832)]{
  let source=if width==64{SOURCE.to_owned()}else{SOURCE.replace("[u64]","[u8]").replace("0u64, capacity","0u8, capacity").replace("17u64","17u8")};
  let parsed=parse_canonical_document(TextSnapshot::new(DocumentId(18),Revision(1),source.as_str()).unwrap(),ParseConfig::default());assert!(parsed.is_strictly_clean(),"{:?}",parsed.diagnostics);
  let original=CanonicalSourceFrontend.compile_document_with_catalog(&DocumentSyntax::cast(parsed.syntax()).unwrap(),catalog.clone()).unwrap().compile_artifact().unwrap();let decoded=mech_engine::decode_program_artifact_bytecode_v1(&mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
  for(form,artifact)in[("original",original),("decoded",decoded)]{
   let jit=JitProgramArtifact::compile_with_limits(&artifact,&catalog,mode.compilation()).unwrap();
   for capacity in[16,real]{
    let memory=mode.memory_budget();let inputs=artifact.inputs().iter().map(|input|{let value=ValueDraft{schema:input.schema,shape_values:Box::new([]),data:D::U64(capacity)}.finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();let mut reservation=memory.reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap()).unwrap();value.into_memory_budget(&mut reservation).unwrap()}).collect::<Vec<_>>();let mut out=vec![None;artifact.outputs().len()];let stats=jit.evaluate_into(&inputs,&mut out,mode.execution(),&memory).unwrap();let value=out.last().unwrap().as_ref().unwrap();let V::Matrix(matrix)=value.data()else{panic!()};assert_eq!(value.canonical_data_draft().unwrap(),D::Matrix(vec![if width==64{D::U64(17)}else{D::U8(17)}].into()));
    println!("{}",serde_json::json!({"witness":"nested-record-reservation","form":form,"width":width,"requested_capacity":capacity,"actual_capacity":matrix.unsigned_buffer_capacity(),"private_frame_fields":jit.descriptor().plan().facts().buffer_frames().elements(),"work":stats.work,"visits":stats.visits}));drop((out,inputs));assert_eq!((memory.used_bytes(),memory.retained_snapshot_nodes()),(0,0));
   }
  }
 }
}
