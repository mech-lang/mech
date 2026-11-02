use super::*;
use crate::resident::general::native_collection_calls::prepare_native_program_library_call;
use crate::{CanonicalSourceFrontend, ProgramArtifact};
use mech_core::{
    ManagedMemoryBudget, ValueDataDraft as Data, ValueDraft,
    snapshot::{MapEntryDraft, NamedValueDraft, SnapshotValidationContext},
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn variants() -> [ProgramArtifact; 2] {
    let source = "<words> := <{index<{string:u64}> count<u64>}>\nwords := input<words>\nindex := words.index\n(index{key<string>}, key ∈ index)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x42524d50), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let artifact = CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
}
fn catalog() -> mech_core::FunctionCatalog {
    let mut b = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_runtime(&mut b).unwrap();
    crate::install_intrinsic_compiler_runtime(&mut b).unwrap();
    crate::install_intrinsic_resident(&mut b).unwrap();
    crate::install_intrinsic_source(&mut b).unwrap();
    b.build().unwrap()
}
fn calls(artifact: &ProgramArtifact) -> Vec<NativePreparedLibraryCall> {
    let catalog = catalog();
    let plan = crate::native_artifact::lower_native_program_artifact(artifact, &catalog).unwrap();
    plan.blocks()
        .iter()
        .filter_map(|block| {
            let crate::native_artifact::NativeIterationAction::LibraryCall { template, .. } =
                block.action()
            else {
                return None;
            };
            Some(prepare_native_program_library_call(artifact, template, &catalog).unwrap())
        })
        .collect()
}
fn ordinal(call: &NativePreparedLibraryCall) -> usize {
    let SchemaBody::Record(fields) = call.template.inputs()[0].body() else {
        panic!("record")
    };
    fields.iter().position(|f| f.name == "index").unwrap()
}
fn limits() -> MemoryBudgetLimits {
    let mut l = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    l.max_compute_work = Some(u64::MAX);
    l.max_comparison_work = Some(u64::MAX);
    l.max_output_bytes = Some(u64::MAX);
    l.max_output_elements = Some(u64::MAX);
    l.max_temporary_bytes = Some(u64::MAX);
    l.max_cloned_bytes = Some(u64::MAX);
    l.max_retained_nodes = Some(u64::MAX);
    l
}
fn value(
    artifact: &ProgramArtifact,
    schema: SchemaId,
    data: Data,
    memory: &ManagedMemoryBudget,
) -> Value {
    let v = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut capacity = memory
        .reserve_capacity(v.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    v.into_memory_budget(&mut capacity).unwrap()
}
fn values(
    artifact: &ProgramArtifact,
    calls: &[NativePreparedLibraryCall],
    memory: &ManagedMemoryBudget,
    number: u64,
    key: &str,
) -> (Value, Value, Value) {
    let project = calls
        .iter()
        .find(|c| c.supports_borrowed_record_map_projection())
        .unwrap();
    let read = calls
        .iter()
        .find(|c| c.template.primitive() == Primitive::MapKeyAccess)
        .unwrap();
    let mech_core::ResolvedOperationContract::Declared(p) = project.template.contract() else {
        panic!("declared")
    };
    let mech_core::ResolvedOperationContract::Declared(r) = read.template.contract() else {
        panic!("declared")
    };
    let map = Data::Map(
        [("é", number), ("a\0b", 0), ("🧑‍🚀", u64::MAX)]
            .map(|(key, value)| MapEntryDraft {
                items: [Data::String(key.into()), Data::U64(value)].into(),
            })
            .into(),
    );
    let root = value(
        artifact,
        p.inputs[0].schema,
        Data::Record(
            [
                NamedValueDraft {
                    name: "index".into(),
                    value: map,
                },
                NamedValueDraft {
                    name: "count".into(),
                    value: Data::U64(number + 1),
                },
            ]
            .into(),
        ),
        memory,
    );
    (
        root,
        value(
            artifact,
            p.inputs[1].schema,
            Data::Id(mech_core::hash_str("index")),
            memory,
        ),
        value(
            artifact,
            r.inputs[1].schema,
            Data::String(key.into()),
            memory,
        ),
    )
}
#[test]
fn borrowed_component_measurement_matches_canonical_child_exactly() {
    for artifact in variants() {
        let calls = calls(&artifact);
        let memory = ManagedMemoryBudget::new(64 << 20);
        let (root, _, _) = values(&artifact, &calls, &memory, 11, "é");
        let call = calls
            .iter()
            .find(|c| c.supports_borrowed_record_map_projection())
            .unwrap();
        let ValueData::Record(record) = root.data() else {
            panic!("record")
        };
        let body = call.template.output().body();
        let (_, data) =
            record_map(&root, &call.schemas, root.schema(), ordinal(call), body).unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = call.template.contract()
        else {
            panic!("declared")
        };
        let Data::Record(fields) = root.canonical_data_draft().unwrap() else {
            panic!("record")
        };
        let canonical = value(
            &artifact,
            contract.outputs[0].schema,
            fields
                .iter()
                .find(|f| f.name == "index")
                .unwrap()
                .value
                .clone(),
            &memory,
        );
        let mut ordinary = ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
        let fp = crate::resident::budget::measure_borrowed_canonical_value_footprint(
            &mut ordinary,
            &canonical,
            artifact.schemas(),
        )
        .unwrap();
        let mut borrowed = ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
        let got =
            measure_borrowed_canonical_component_footprint(&mut borrowed, body, data).unwrap();
        assert_eq!(got, fp);
        assert_eq!(
            ordinary.estimate().compute_work(),
            borrowed.estimate().compute_work()
        );
        assert_eq!(
            ordinary.estimate().retained_nodes(),
            borrowed.estimate().retained_nodes()
        );
        assert_eq!(record.fields().len(), 2);
    }
}
#[test]
fn borrowed_record_map_reads_current_canonical_root_and_rejects_foreign_or_wrong_shape() {
    for artifact in variants() {
        let calls = calls(&artifact);
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let project = calls
            .iter()
            .find(|c| c.supports_borrowed_record_map_projection())
            .unwrap();
        let access = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapKeyAccess)
            .unwrap();
        let contains = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapContainsKey)
            .unwrap();
        assert!(
            access.supports_borrowed_record_map_read()
                && contains.supports_borrowed_record_map_read()
        );
        let ordinal = ordinal(project);
        for number in [7, 123, 0] {
            for key in ["é", "a\0b", "🧑‍🚀", "absent"] {
                let (root, selector, key_value) = values(&artifact, &calls, &memory, number, key);
                let before = root.canonical_data_draft().unwrap();
                let held = (memory.used_bytes(), memory.retained_nodes());
                project
                    .admit_borrowed_record_map(
                        &root,
                        &selector,
                        ordinal,
                        limits(),
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap();
                let found = contains
                    .read_borrowed_record_map(
                        &root,
                        root.schema(),
                        ordinal,
                        &key_value,
                        limits(),
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap();
                assert_eq!(found, u64::from(key != "absent"));
                let result = access.read_borrowed_record_map(
                    &root,
                    root.schema(),
                    ordinal,
                    &key_value,
                    limits(),
                    &work,
                    ResourceDemand::default(),
                );
                if key == "absent" {
                    assert_eq!(
                        result,
                        Err(InvokeError::Kernel(ResidentKernelError::InvalidInput))
                    );
                } else {
                    assert_eq!(
                        result,
                        Ok(match key {
                            "é" => number,
                            "a\0b" => 0,
                            _ => u64::MAX,
                        })
                    );
                }
                assert_eq!(root.canonical_data_draft().unwrap(), before);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), held);
                assert!(
                    access
                        .read_borrowed_record_map(
                            &root,
                            root.schema(),
                            1 - ordinal,
                            &key_value,
                            limits(),
                            &work,
                            ResourceDemand::default()
                        )
                        .is_err()
                );
                let foreign = ManagedMemoryBudget::new(64 << 20);
                let other = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &foreign).unwrap();
                assert_eq!(
                    access.read_borrowed_record_map(
                        &root,
                        root.schema(),
                        ordinal,
                        &key_value,
                        limits(),
                        &other,
                        ResourceDemand::default()
                    ),
                    Err(InvokeError::UnadmittedValue)
                );
            }
        }
    }
}

fn cap(l: &mut MemoryBudgetLimits, dimension: &str, n: u64) {
    match dimension {
        "compute" => l.max_compute_work = Some(n),
        "comparison" => l.max_comparison_work = Some(n),
        "output-bytes" => l.max_output_bytes = Some(n),
        "output-elements" => l.max_output_elements = Some(n),
        "cloned-bytes" => l.max_cloned_bytes = Some(n),
        "temporary" => l.max_temporary_bytes = Some(n),
        "nodes" => l.max_retained_nodes = Some(n),
        _ => unreachable!(),
    }
}
#[test]
fn borrowed_record_map_exact_one_short_caps_keep_root_and_recover() {
    for artifact in variants() {
        let calls = calls(&artifact);
        let memory = ManagedMemoryBudget::new(64 << 20);
        let project = calls
            .iter()
            .find(|c| c.supports_borrowed_record_map_projection())
            .unwrap();
        let access = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapKeyAccess)
            .unwrap();
        let contains = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapContainsKey)
            .unwrap();
        let ordinal = ordinal(project);
        let (root, selector, key) = values(&artifact, &calls, &memory, 19, "é");
        let before = root.canonical_data_draft().unwrap();
        let held = (memory.used_bytes(), memory.retained_nodes());
        for phase in ["project", "access", "contains"] {
            for dimension in [
                "compute",
                "comparison",
                "output-bytes",
                "output-elements",
                "cloned-bytes",
                "temporary",
                "nodes",
            ] {
                let trial = |n| {
                    let mut policy = limits();
                    cap(&mut policy, dimension, n);
                    let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
                    let result = match phase {
                        "project" => project
                            .admit_borrowed_record_map(
                                &root,
                                &selector,
                                ordinal,
                                policy,
                                &work,
                                ResourceDemand::default(),
                            )
                            .map(|_| 0),
                        "access" => access.read_borrowed_record_map(
                            &root,
                            root.schema(),
                            ordinal,
                            &key,
                            policy,
                            &work,
                            ResourceDemand::default(),
                        ),
                        _ => contains.read_borrowed_record_map(
                            &root,
                            root.schema(),
                            ordinal,
                            &key,
                            policy,
                            &work,
                            ResourceDemand::default(),
                        ),
                    };
                    let pass = result.is_ok();
                    drop(work);
                    assert_eq!(root.canonical_data_draft().unwrap(), before);
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), held);
                    pass
                };
                let (mut low, mut high) = (0, 1_000_000_000_000u64);
                assert!(trial(high), "{phase} {dimension}");
                while low < high {
                    let m = low + (high - low) / 2;
                    if trial(m) { high = m } else { low = m + 1 }
                }
                assert!(trial(low), "{phase} {dimension} exact={low}");
                if low > 0 {
                    assert!(!trial(low - 1), "{phase} {dimension} one-short={low}");
                }
                assert!(trial(1_000_000_000_000));
                if phase != "project" && dimension == "cloned-bytes" {
                    assert_eq!(low, 0, "borrowed scalar read has no canonical clone");
                }
            }
        }
    }
}

#[test]
fn borrowed_projection_matches_canonical_output_and_clone_limits() {
    for artifact in variants() {
        let calls = calls(&artifact);
        let memory = ManagedMemoryBudget::new(128 << 20);
        let project = calls
            .iter()
            .find(|c| c.supports_borrowed_record_map_projection())
            .unwrap();
        let ordinal = ordinal(project);
        let (root, selector, _) = values(&artifact, &calls, &memory, 31, "é");
        for dimension in ["output-bytes", "output-elements", "cloned-bytes"] {
            let minimum = |borrowed| {
                let trial = |n| {
                    let mut policy = limits();
                    cap(&mut policy, dimension, n);
                    let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
                    let mut output = None;
                    let pass = if borrowed {
                        project
                            .admit_borrowed_record_map(
                                &root,
                                &selector,
                                ordinal,
                                policy,
                                &work,
                                ResourceDemand::default(),
                            )
                            .is_ok()
                    } else {
                        project
                            .invoke_current(
                                &[Some(root.clone()), Some(selector.clone())],
                                &mut output,
                                policy,
                                &work,
                            )
                            .is_ok()
                    };
                    drop((output, work));
                    pass
                };
                let (mut lo, mut hi) = (0, 1_000_000_000_000u64);
                assert!(trial(hi));
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    if trial(mid) {
                        hi = mid
                    } else {
                        lo = mid + 1;
                    }
                }
                assert!(trial(lo));
                if lo > 0 {
                    assert!(!trial(lo - 1));
                }
                lo
            };
            assert_eq!(
                minimum(true),
                minimum(false),
                "{dimension}: opaque view must retain maintained canonical logical caps"
            );
        }
    }
}

#[test]
fn borrowed_contains_uses_one_bool_byte_at_exact_temporary_boundary() {
    for artifact in variants() {
        let calls = calls(&artifact);
        let memory = ManagedMemoryBudget::new(64 << 20);
        let project = calls
            .iter()
            .find(|c| c.supports_borrowed_record_map_projection())
            .unwrap();
        let contains = calls
            .iter()
            .find(|c| c.template.primitive() == Primitive::MapContainsKey)
            .unwrap();
        let (root, _, key) = values(&artifact, &calls, &memory, 7, "é");
        let (map, _) = record_map(
            &root,
            &contains.schemas,
            root.schema(),
            ordinal(project),
            contains.template.inputs()[0].body(),
        )
        .unwrap();
        let keys = map
            .entries()
            .iter()
            .map(|e| match e.key().data() {
                ValueData::String(s) => s.len() as u64,
                _ => panic!("string key"),
            })
            .sum::<u64>();
        let exact = root.memory_budget_retained_bytes(&memory).unwrap()
            + key.memory_budget_retained_bytes(&memory).unwrap()
            + 1
            + keys;
        let held = (memory.used_bytes(), memory.retained_nodes());
        for allowance in [exact, exact - 1, exact] {
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let mut policy = limits();
            policy.max_temporary_bytes = Some(allowance);
            policy.max_output_bytes = Some(1);
            policy.max_output_elements = Some(1);
            policy.max_cloned_bytes = Some(0);
            let result = contains.read_borrowed_record_map(
                &root,
                root.schema(),
                ordinal(project),
                &key,
                policy,
                &work,
                ResourceDemand::default(),
            );
            if allowance == exact {
                assert_eq!(result, Ok(1));
            } else {
                assert!(result.is_err());
            }
            drop(work);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), held);
        }
    }
}
