include!("build/linked.rs");
include!("build/catalog.rs");
fn main() {
 let catalog=linked_catalog().unwrap();
 let result=unsafe { mech_gpu::native_artifact::LinkedProgramArtifact::from_object_bound_image(&LINKED_NATIVE_METADATA,&catalog,&LINKED_IDENTITY,&LINKED_NATIVE_METADATA_BINDING,linked_entry) };
 let error=match result { Ok(_) => panic!("old Object must not enter"), Err(e)=>e.to_string() };
 assert!(!error.is_empty());
 let compiled=unsafe { mech_gpu::native_artifact::LinkedProgramArtifact::from_compiled_module(&LINKED_MODULE,&catalog) };
 let compiled_error=match compiled {Ok(_)=>panic!("old compiled module must not enter"),Err(e)=>e.to_string()};
 assert!(!compiled_error.is_empty());
 println!("{}",serde_json::json!({"old_typed_ir":213,"old_entry_never_invoked":true,"object_bound_rejected":error,"compiled_module_rejected":compiled_error}));
}
