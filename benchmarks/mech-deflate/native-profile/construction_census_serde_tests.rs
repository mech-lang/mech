//! Deserializing an IR graph must participate in the same arena lifecycle.
use cranelift_codegen::{
    construction_census as census,
    ir::{DataFlowGraph, types},
};
#[test]
fn deserialized_graph_accounts_for_its_maps_without_changing_the_wire_shape() {
    census::reset();
    let mut original = DataFlowGraph::new();
    let block = original.make_block();
    original.append_block_param(block, types::I64);
    let before = census::snapshot();
    let encoded = serde_json::to_string(&original).unwrap();
    let restored: DataFlowGraph = serde_json::from_str(&encoded).unwrap();
    assert!(restored == original);
    assert_eq!(serde_json::to_string(&restored).unwrap(), encoded);
    let after = census::snapshot();
    assert_eq!(after.values.created, 2 * before.values.created);
    assert_eq!(after.blocks.created, 2 * before.blocks.created);
    assert_eq!(after.values.resident, 2 * before.values.resident);
    drop(restored);
    assert_eq!(census::snapshot().values.resident, before.values.resident);
    drop(original);
    assert_eq!(census::snapshot().values.resident, 0);
    assert_eq!(census::snapshot().blocks.resident, 0);
}
