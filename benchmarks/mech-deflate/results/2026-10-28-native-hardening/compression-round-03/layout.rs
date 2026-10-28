use mech_core::{SchemaTable, SchemaTableBuilder, SchemaDraft, SchemaBody};
fn main() {
 let mut builder=SchemaTableBuilder::new();
 builder.insert(SchemaDraft{dimension_parameters:Box::new([]),body:SchemaBody::Bool}.finalize().unwrap()).unwrap();
 let table=builder.finish().unwrap().table;
 let mut contributions=Vec::new();
 let walked=table.clone_allocation_bound_bytes_with_precharge(|bytes|{contributions.push(bytes);Ok::<_,()>(())}).unwrap();
 assert_eq!(table.clone_allocation_bound_bytes(),Some(walked));
 println!("schema_table_bytes={} schema_table_align={} entries={} bound={} contributions={:?}",std::mem::size_of::<SchemaTable>(),std::mem::align_of::<SchemaTable>(),table.len(),walked,contributions);
}
