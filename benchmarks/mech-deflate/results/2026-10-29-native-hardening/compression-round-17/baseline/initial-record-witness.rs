use mech_core::{SchemaBody as B, Value, ValueDataDraft as D, snapshot::{NamedValueDraft, SnapshotValidationContext, ValueDraft}};
use mech_engine::{CanonicalSourceFrontend, native_artifact::{NativeCollectionPrimitive as P, NativeIterationAction as A, NativeLibraryWorkBudget, NativePreparedLibraryCallCache, lower_native_program_artifact, prepare_native_program_library_call}};
use mech_gpu::native_artifact::NativeProgramLimitMode;
use mech_syntax::document::{AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot, parse_canonical_document};
#[path = "/private/tmp/mech-deflate-linear-stack/src/core/tests/support/allocation_probe.rs"]
mod allocation_probe;
#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;
fn words(width: u8, n: usize, value: u64) -> D {
 D::Matrix((0..n).map(|_| match width {8=>D::U8(value as u8),16=>D::U16(value as u16),32=>D::U32(value as u32),64=>D::U64(value),_=>unreachable!()}).collect())
}
fn record(values: Vec<(&str,D)>) -> D {
 D::Record(values.into_iter().map(|(name,value)|NamedValueDraft{name:name.into(),value}).collect())
}
fn admit(value:Value,memory:&mech_core::ManagedMemoryBudget)->Value{
 let mut claim=memory.reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap()).unwrap();
 value.into_memory_budget(&mut claim).unwrap()
}
fn main(){
 // Keep the shared allocation-probe API exercised, without a nested measurement.
 assert_eq!(allocation_probe::measured(|| 7).0,7);
 let catalog=mech_stdlib::source_catalog();
 for width in [8u8,16,32,64]{
  let source=format!("values := {{left: a<[u{width}]>, right: b<[u{width}]>, costs: {{literal: 1u64, length: 2u64, offset: 3u64}}, count: n<u64>}}\nvalues.left\n");
  let parsed=parse_canonical_document(TextSnapshot::new(DocumentId(17),Revision(1),&source).unwrap(),ParseConfig::default());assert!(parsed.is_strictly_clean());
  let original=CanonicalSourceFrontend.compile_document_with_catalog(&DocumentSyntax::cast(parsed.syntax()).unwrap(),catalog.clone()).unwrap().compile_artifact().unwrap();
  let bytes=mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
  let decoded=mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
  for (form,artifact) in [("original",original),("decoded",decoded)]{
   let plan=lower_native_program_artifact(&artifact,&catalog).unwrap();
   let template=plan.blocks().iter().find_map(|b|match b.action(){A::LibraryCall{template,..}if template.primitive()==P::RecordFieldAccess=>Some(template),_=>None}).unwrap();
   let call=prepare_native_program_library_call(&artifact,template,&catalog).unwrap();
   let mech_core::ResolvedOperationContract::Declared(contract)=template.contract() else{panic!()};
   let schema=artifact.schemas().get(contract.inputs[0].schema).unwrap();
   let mode=NativeProgramLimitMode::NoLimits;let memory=mode.memory_budget();
   let work=NativeLibraryWorkBudget::new(u64::MAX,u64::MAX,&memory).unwrap();let mut cache=NativePreparedLibraryCallCache::default();let mut output=None;
   for (ar,ac,br,bc) in [(1,3,2,2),(2,3,1,0),(0,7,3,2),(3,0,1,1)]{
    let mut body=schema.body().clone();let B::Record(fields)=&mut body else{panic!()};
    for field in fields{let dims=match field.name.as_ref(){"left"=>Some((ar,ac)),"right"=>Some((br,bc)),_=>None};if let Some((r,c))=dims{let B::Matrix{dimensions,..}=&mut field.schema else{panic!()};*dimensions=vec![mech_core::DimensionExpr::Constant(r),mech_core::DimensionExpr::Constant(c)].into();}}
    let shape=mech_core::shape_for_schema_components(schema,&[(schema.body(),body)],None).unwrap();
    let left=words(width,(ar*ac)as usize,7);let right=words(width,(br*bc)as usize,19);
    let data=record(vec![("left",left.clone()),("right",right.clone()),("costs",record(vec![("literal",D::U64(1)),("length",D::U64(2)),("offset",D::U64(3))])),("count",D::U64(23))]);
    let source=admit(ValueDraft{schema:contract.inputs[0].schema,shape_values:shape.parameter_values().into(),data}.finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap(),&memory);
    let alias=source.clone();
    for field in ["left","right","missing","left"]{
     let selector=admit(ValueDraft{schema:contract.inputs[1].schema,shape_values:Box::new([]),data:D::Id(mech_core::hash_str(field))}.finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap(),&memory);
     let args=[Some(source.clone()),Some(selector)];let prior=output.clone();let before_memory=memory.used_bytes();let before_work=work.used();let start=std::time::Instant::now();
     let (result,requests,requested_bytes)=allocation_probe::measured_with_bytes(||call.invoke_current_cached(&mut cache,&args,&mut output,mode.execution().library,&work));let elapsed=start.elapsed().as_secs_f64();
     let rejection=field=="missing";
     if rejection{assert!(result.is_err());assert!(output.as_ref().unwrap().shares_frozen_storage(prior.as_ref().unwrap()));assert_eq!(memory.used_bytes(),before_memory);}else{result.as_ref().unwrap();assert_eq!(output.as_ref().unwrap().canonical_data_draft().unwrap(),if field=="left"{left.clone()}else{right.clone()});}
     assert!(source.shares_frozen_storage(&alias));
     let demand=cache.retained_metadata_demand(&work).unwrap();assert_eq!(demand.turn_peak_bytes,0);
     println!("{}",serde_json::json!({"kind":"source-derived-record-binding","form":form,"width":width,"left_shape":[ar,ac],"right_shape":[br,bc],"selector":field,"rejected":rejection,"error":result.err().map(|e|format!("{e:?}")),"allocation_requests":requests,"requested_bytes":requested_bytes,"seconds":elapsed,"work":work.used().0-before_work.0,"metadata_bytes":demand.turn_peak_bytes}));
    }
   }
   drop((output,cache,work));assert_eq!((memory.used_bytes(),memory.retained_snapshot_nodes()),(0,0));
  }
 }
}
