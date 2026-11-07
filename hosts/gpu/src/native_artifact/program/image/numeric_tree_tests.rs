//! Forged ancestry, source edges, scalar homes and traversal prices cannot load.
use super::{NativeProgramDescriptor, identity};
use crate::native_artifact::{JitProgramArtifact, emit_program_object};
use mech_engine::ProgramArtifact;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts() -> [ProgramArtifact; 2] {
    artifacts_for(include_str!(
        "../../../../tests/fixtures/native/numeric-changing-capture-tree.mec"
    ))
}

fn artifacts_for(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x54524545494d47), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

#[test]
fn numeric_tree_images_rederive_ancestry_fuel_edges_and_cold_scalar_homes() {
    let catalog = mech_stdlib::source_catalog();
    let mut previous = None;
    for artifact in artifacts() {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let descriptor = jit.descriptor();
        let encoded = descriptor.encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        let tails = &image["descriptor"]["closed_tails"];
        let at = tails["regions"]
            .as_array()
            .unwrap()
            .iter()
            .position(|region| region["numeric_tree"].is_object())
            .expect("the captured parent/child fixture must contain a real tree");
        let original = tails["regions"][at]["numeric_tree"].clone();
        assert!(original["nodes"].as_array().unwrap().len() >= 2);
        if let Some(previous) = &previous {
            assert_eq!(previous, &original);
        }
        previous = Some(original.clone());
        // SAFETY: exact unchanged build-owned metadata; restored ownership is
        // confined to this JIT and never substitutes different generated code.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                .unwrap();
        assert_eq!(restored.typed_ir_sha256(), descriptor.typed_ir_sha256());
        let restored_image: serde_json::Value =
            serde_json::from_slice(&restored.encode_build_owned_image().unwrap()).unwrap();
        assert_eq!(restored_image["descriptor"]["closed_tails"], *tails);

        let mut changes = Vec::new();
        for field in ["iteration", "compute", "comparison"] {
            let mut changed = original.clone();
            let word = &mut changed["nodes"][1][field];
            *word = (word.as_u64().unwrap() + 1).into();
            changes.push(changed);
        }
        let mut cyclic = original.clone();
        cyclic["nodes"][1]["parent"] = 1.into();
        changes.push(cyclic);
        let mut detached = original.clone();
        detached["nodes"][1]["parent"] = serde_json::Value::Null;
        changes.push(detached);
        let mut counters = original.clone();
        counters["counters"] = (counters["counters"].as_u64().unwrap() + 1).into();
        changes.push(counters);
        for column in [0, 1] {
            let mut changed = original.clone();
            let word = &mut changed["sites"][0][column];
            *word = (word.as_u64().unwrap() + 1).into();
            changes.push(changed);
        }
        for field in ["nodes", "sites"] {
            let mut missing = original.clone();
            missing[field].as_array_mut().unwrap().pop();
            changes.push(missing);
            let mut duplicate = original.clone();
            duplicate[field]
                .as_array_mut()
                .unwrap()
                .push(original[field][0].clone());
            changes.push(duplicate);
        }
        let binding_node = original["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|node| !node["scalar_bindings"].as_array().unwrap().is_empty())
            .expect("exercise a real prepared cold scalar binding");
        for field in ["word", "physical", "steps"] {
            let mut changed = original.clone();
            let word = &mut changed["nodes"][binding_node]["scalar_bindings"][0][field];
            *word = (word.as_u64().unwrap() + 1).into();
            changes.push(changed);
        }
        let mut unknown = original.clone();
        unknown["native_permission"] = true.into();
        changes.push(unknown);
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove("counters");
        changes.push(missing);
        changes.push(serde_json::Value::Null);
        for changed in changes {
            let mut forged = image.clone();
            forged["descriptor"]["closed_tails"]["regions"][at]["numeric_tree"] = changed;
            let encoded = serde_json::to_vec(&forged).unwrap();
            // SAFETY: wire-only negative control; forged recipes are never
            // attached to an Object or invoked, even if the loader regresses.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                    .is_err()
            );
        }
    }
}

#[test]
fn numeric_tree_object_identity_rejects_ir152_and_work104_images() {
    let catalog = mech_stdlib::source_catalog();
    assert!(identity::NATIVE_PROGRAM_TYPED_IR_VERSION > 152);
    assert!(identity::NATIVE_PROGRAM_WORK_POLICY_VERSION > 104);
    for artifact in artifacts() {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let identity = object.identity();
        for (old_ir, old_work) in [(true, false), (false, true), (true, true)] {
            let mut old = *identity.bytes();
            if old_ir {
                old[16..24].copy_from_slice(&152u64.to_le_bytes());
            }
            if old_work {
                old[32..40].copy_from_slice(&104u64.to_le_bytes());
            }
            assert!(
                crate::native_artifact::program::linked::prepare_descriptor(
                    &artifact, &catalog, &old
                )
                .is_err()
            );
        }
    }
}

#[test]
fn numeric_induction_object_identity_rejects_ir153_images() {
    let catalog = mech_stdlib::source_catalog();
    assert!(identity::NATIVE_PROGRAM_TYPED_IR_VERSION > 153);
    for artifact in artifacts() {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let mut old = *object.identity().bytes();
        old[16..24].copy_from_slice(&153u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
    }
}

const APPEND_CAPTURE_TREE: &str =
    include_str!("../../../../tests/fixtures/native/numeric-changing-capture-tree.mec");

#[test]
fn numeric_tree_images_rederive_exact_cold_alias_roots_values_and_header_availability() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts_for(APPEND_CAPTURE_TREE) {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        let regions = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap();
        let (at, header) = regions
            .iter()
            .enumerate()
            .find_map(|(at, region)| {
                if !region["numeric_tree"].is_object() {
                    return None;
                }
                region["numeric"]["cold_aliases"]
                    .as_array()?
                    .iter()
                    .enumerate()
                    .find(|(_, row)| row[1].as_array().is_some_and(|aliases| !aliases.is_empty()))
                    .map(|(header, _)| (at, header))
            })
            .expect("the captured nested append fixture must prepare a real cold alias");
        let original = regions[at]["numeric"]["cold_aliases"].clone();
        // SAFETY: exact unchanged build-owned image, never foreign generated code.
        unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }.unwrap();
        let mut changes = Vec::new();
        let mut site = original.clone();
        site[header][0] = (site[header][0].as_u64().unwrap() + 1).into();
        changes.push(site);
        for field in ["value", "array"] {
            for replacement in [
                original[header][1][0][field].as_u64().unwrap() + 1,
                u64::MAX,
            ] {
                let mut changed = original.clone();
                changed[header][1][0][field] = replacement.into();
                changes.push(changed);
            }
        }
        let mut missing_alias = original.clone();
        missing_alias[header][1].as_array_mut().unwrap().remove(0);
        changes.push(missing_alias);
        let mut duplicate_alias = original.clone();
        duplicate_alias[header][1]
            .as_array_mut()
            .unwrap()
            .push(original[header][1][0].clone());
        changes.push(duplicate_alias);
        let mut missing_header = original.clone();
        missing_header.as_array_mut().unwrap().remove(header);
        changes.push(missing_header);
        let mut duplicate_header = original.clone();
        duplicate_header
            .as_array_mut()
            .unwrap()
            .push(original[header].clone());
        changes.push(duplicate_header);
        let mut unknown_permission = original.clone();
        unknown_permission[header][1][0]["refresh_any_version"] = true.into();
        changes.push(unknown_permission);
        changes.push(serde_json::Value::Null);
        for changed in changes {
            let mut forged = image.clone();
            forged["descriptor"]["closed_tails"]["regions"][at]["numeric"]["cold_aliases"] =
                changed;
            let encoded = serde_json::to_vec(&forged).unwrap();
            // SAFETY: metadata-only rejection test; a forgery is never executed.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                    .is_err()
            );
        }
        let mut absent = image.clone();
        absent["descriptor"]["closed_tails"]["regions"][at]["numeric"]
            .as_object_mut()
            .unwrap()
            .remove("cold_aliases");
        let encoded = serde_json::to_vec(&absent).unwrap();
        // SAFETY: metadata-only rejection test; a forgery is never executed.
        assert!(
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                .is_err()
        );
    }
}

const SCALAR_COLD_SOURCE: &str = r#"+> sequence
<tree-rank> := <((u8,u64),u64)>
<tree-state> := <([u8],tree-rank)>
tree-put(data<[u8]>, at<u64>, value<u8>) => (out<[u8]>) :=
  ~next := data
  next[at] = value
  out := next.
#TreeProbe(data<[u8]>, base<u64>, width<u64>, minimum<u8>) => <bool>
  | :Next(at<u64>, found<bool>)
  | :Done(found<bool>).
#TreeProbe(data, base, width, minimum) -> :Next(0u64, false)
  :Next(at, found)
    | at < width -> :Next(at + 1u64, found || (data[base + at + 1u64] > minimum))
    | * -> :Done(found)
  :Done(found) => found.
tree-lane(data<[u8]>, base<u64>, at<u64>, process<bool>, rank<tree-rank>) => (out<tree-state>) :=
  position := base + at
  count := data[position + 1u64]
  chosen := ((process && (count > rank.1.1))? | true => (count, position) | false => rank.1)
  next-rank := (chosen, rank.2 + 1u64)
  next := tree-put(data, position + 1u64, 0u8)
  out := (next, next-rank).
tree-child(data<[u8]>, base<u64>, at<u64>, width<u64>, process<bool>, rank<tree-rank>) => (out<tree-state>) :=
  more := at < width
  step := (more? | true => tree-lane(data, base, at, process, rank) | false => (data, rank))
  (next, next-rank) := step
  out := (more? | true => tree-child(next, base, at + 1u64, width, process, next-rank) | false => (next, next-rank)).
tree-parent(data<[u8]>, round<u64>, rounds<u64>, width<u64>, rank<tree-rank>) => (out<tree-state>) :=
  more := round < rounds
  base := round * width
  process := (more? | true => #TreeProbe(data, base, width, rank.1.1) | false => false)
  step := (more? | true => tree-child(data, base, 0u64, width, process, rank) | false => (data, rank))
  (next, next-rank) := step
  out := (more? | true => tree-parent(next, round + 1u64, rounds, width, next-rank) | false => (next, next-rank)).
result := tree-parent(bytes<[u8]>, 0u64, rounds<u64>, width<u64>, ((minimum<u8>, winner<u64>), marker<u64>))
(result.1, result.2.1.1, result.2.1.2, result.2.2, bytes)
"#;

#[test]
fn numeric_tree_images_rederive_exact_cold_scalar_frame_homes_and_kinds() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts_for(SCALAR_COLD_SOURCE) {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        let regions = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap();
        let (at, header) = regions.iter().enumerate().find_map(|(at, region)| {
            if region["numeric_tree"]["nodes"].as_array()?.len() < 3 { return None; }
            region["numeric"]["cold_scalars"].as_array()?.iter().enumerate()
                .find(|(_, row)| row[1].as_array().is_some_and(|bindings| !bindings.is_empty()))
                .map(|(header, _)| (at, header))
        }).expect("the real3node capture/probe/writer tree must prepare exact cold scalar-frame homes");
        let original = regions[at]["numeric"]["cold_scalars"].clone();
        // SAFETY: unchanged build-owned wire metadata; never foreign code.
        unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }.unwrap();
        let mut changes = Vec::new();
        let mut site = original.clone();
        site[header][0] = (site[header][0].as_u64().unwrap() + 1).into();
        changes.push(site);
        for field in ["value", "word", "physical", "steps"] {
            for replacement in [
                original[header][1][0][field].as_u64().unwrap() + 1,
                u64::MAX,
            ] {
                let mut changed = original.clone();
                changed[header][1][0][field] = replacement.into();
                changes.push(changed);
            }
        }
        for kind in ["Bool", "U8", "U64", "F64", "ForeignScalar"] {
            if original[header][1][0]["kind"].as_str() == Some(kind) {
                continue;
            }
            let mut changed = original.clone();
            changed[header][1][0]["kind"] = kind.into();
            changes.push(changed);
        }
        let mut missing = original.clone();
        missing[header][1].as_array_mut().unwrap().remove(0);
        changes.push(missing);
        let mut duplicate = original.clone();
        duplicate[header][1]
            .as_array_mut()
            .unwrap()
            .push(original[header][1][0].clone());
        changes.push(duplicate);
        let mut missing_header = original.clone();
        missing_header.as_array_mut().unwrap().remove(header);
        changes.push(missing_header);
        let mut duplicate_header = original.clone();
        duplicate_header
            .as_array_mut()
            .unwrap()
            .push(original[header].clone());
        changes.push(duplicate_header);
        let mut unknown = original.clone();
        unknown[header][1][0]["restore_any_frame"] = true.into();
        changes.push(unknown);
        changes.push(serde_json::Value::Null);
        for changed in changes {
            let mut forged = image.clone();
            forged["descriptor"]["closed_tails"]["regions"][at]["numeric"]["cold_scalars"] =
                changed;
            let encoded = serde_json::to_vec(&forged).unwrap();
            // SAFETY: a metadata-only negative; a forgery is never executed.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                    .is_err()
            );
        }
        let mut absent = image.clone();
        absent["descriptor"]["closed_tails"]["regions"][at]["numeric"]
            .as_object_mut()
            .unwrap()
            .remove("cold_scalars");
        let encoded = serde_json::to_vec(&absent).unwrap();
        // SAFETY: a metadata-only negative; a forgery is never executed.
        assert!(
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                .is_err()
        );
    }
}
