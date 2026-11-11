include!("build/linked.rs");
include!("build/catalog.rs");

fn main() {
    use mech_gpu::native_artifact::LinkedProgramArtifact;
    let catalog = linked_catalog().unwrap();
    let original = include_bytes!("build/descriptor.json");
    // These constructors validate descriptors without invoking native code.
    unsafe {
        LinkedProgramArtifact::from_build_owned_image(original, &catalog, &LINKED_IDENTITY, linked_entry).unwrap();
        LinkedProgramArtifact::from_object_bound_image(original, &catalog, &LINKED_IDENTITY, &LINKED_IMAGE_BINDING, linked_entry).unwrap();
        LinkedProgramArtifact::from_compiled_module(&LINKED_MODULE, &catalog).unwrap();
    }
    let image: serde_json::Value = serde_json::from_slice(original).unwrap();
    let region = image["descriptor"]["closed_tails"]["regions"].as_array().unwrap().iter()
        .position(|r| r["tail"]["header"].as_u64() == Some(1471)).unwrap();
    let rows = image["descriptor"]["closed_tails"]["regions"][region]["numeric"]["append_limits"].as_array().unwrap();
    let positive = rows.iter().position(|r| r[1][0][1].as_u64().unwrap() > 0).unwrap();
    let zero = rows.iter().position(|r| r[1][0][1].as_u64() == Some(0)).unwrap();
    let mut cases = Vec::new();
    for kind in ["positive_to_zero", "zero_to_positive", "missing", "empty", "duplicate", "remove_zero", "wrong_header", "wrong_array"] {
        let mut changed = image.clone();
        let proof = &mut changed["descriptor"]["closed_tails"]["regions"][region]["numeric"];
        match kind {
            "positive_to_zero" => proof["append_limits"][positive][1][0][1] = 0.into(),
            "zero_to_positive" => proof["append_limits"][zero][1][0][1] = 1.into(),
            "missing" => { proof.as_object_mut().unwrap().remove("append_limits"); },
            "empty" => proof["append_limits"] = serde_json::json!([]),
            "duplicate" => { let row = proof["append_limits"][zero].clone(); proof["append_limits"].as_array_mut().unwrap().push(row); },
            "remove_zero" => { proof["append_limits"].as_array_mut().unwrap().remove(zero); },
            "wrong_header" => { let old = proof["append_limits"][zero][0].as_u64().unwrap(); proof["append_limits"][zero][0] = (old + 1).into(); },
            "wrong_array" => proof["append_limits"][zero][1][0][0] = 999.into(),
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&changed).unwrap();
        let independent = unsafe { LinkedProgramArtifact::from_build_owned_image(&bytes, &catalog, &LINKED_IDENTITY, linked_entry) };
        let bound = unsafe { LinkedProgramArtifact::from_object_bound_image(&bytes, &catalog, &LINKED_IDENTITY, &LINKED_IMAGE_BINDING, linked_entry) };
        assert!(independent.is_err(), "independent loader accepted {kind}");
        assert!(bound.is_err(), "Object-bound loader accepted {kind}");
        cases.push(serde_json::json!({"case":kind,"independent_rejected":true,"object_bound_rejected":true}));
    }
    println!("{}", serde_json::json!({"genuine_legacy_independent_bound_and_compiled_module_accepted":true,"forged_cases":cases,"native_entry_never_invoked":true}));
}
