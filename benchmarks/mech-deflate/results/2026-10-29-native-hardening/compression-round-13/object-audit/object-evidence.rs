use mech_gpu::native_artifact::{emit_program_object_with_limits, NativeProgramLimitMode};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2);
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(&std::fs::read(&args[0]).unwrap()).unwrap();
    let object = emit_program_object_with_limits(&artifact, &mech_stdlib::source_catalog(), NativeProgramLimitMode::NoLimits.compilation()).unwrap();
    std::fs::write(format!("{}.o", args[1]), object.bytes()).unwrap();
    let metadata = object.metadata();
    std::fs::write(format!("{}.json", args[1]), serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
    let mut receipt = serde_json::Map::new();
    for (name, key) in [("read_only", "compiled_readonly_numeric_loops"), ("writes", "compiled_numeric_write_loops")] {
        let regions = metadata[key].as_array().unwrap();
        assert!(!regions.is_empty());
        assert!(regions.iter().all(|r| r["successful_backedge_helpers"] == 0));
        receipt.insert(name.into(), serde_json::json!({"regions":regions.len(),"parents":regions.iter().filter(|r|r["parent"].is_number()).count(),"typed_load_sites":regions.iter().map(|r|r["typed_loads"].as_array().unwrap().len()).sum::<usize>(),"typed_store_sites":regions.iter().map(|r|r["typed_stores"].as_array().unwrap().len()).sum::<usize>()}));
    }
    let mut dispositions = std::collections::BTreeMap::<String,usize>::new();
    for candidate in metadata["readonly_numeric_candidates"].as_array().unwrap() {
        let reason = if candidate["selected"] == true {"selected"} else {candidate["rejection"].as_str().unwrap_or("unspecified")};
        *dispositions.entry(reason.into()).or_default() += 1;
    }
    let mixed = ["numeric_write_regions","readonly_numeric_regions"].into_iter().flat_map(|key|metadata[key].as_array().unwrap()).filter(|r|r["proof"]["result"]["Frame"].is_object()).collect::<Vec<_>>();
    receipt.insert("private_mixed_result_regions".into(),serde_json::json!(mixed));
    receipt.insert("first_recorded_dispositions".into(),serde_json::json!(dispositions));
    receipt.insert("object_bytes".into(), object.bytes().len().into());
    println!("{}", serde_json::Value::Object(receipt));
}
