use super::super::NativeMatchAction as Scalar;
use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

const SOURCE: &str =
    "text/join([(entries<{string:string}>){piece} | piece <- source<string>], separator<string>)\n";

fn plans(source: &str) -> [Plan; 2] {
    let source = format!("+> text\n+> map\n{source}");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x454c4d50), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_runtime(&mut catalog).unwrap();
    crate::install_intrinsic_compiler_runtime(&mut catalog).unwrap();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    crate::install_intrinsic_source(&mut catalog).unwrap();
    let catalog = Arc::new(catalog.build().unwrap());
    let artifact = crate::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
        .map(|artifact| super::super::lower_native_program_artifact(&artifact, &catalog).unwrap())
}

#[test]
fn string_element_map_projection_is_selected_generic_and_semantically_rederived() {
    for plan in plans(SOURCE) {
        let projections = plan
            .collections()
            .iter()
            .filter_map(|c| c.string_element_map_projection())
            .collect::<Vec<_>>();
        assert_eq!(projections.len(), 1);
        let p = projections[0];
        assert_eq!(
            p.selection_template().primitive(),
            super::super::NativeCollectionPrimitive::MapKeyAccess
        );
        assert!(matches!(plan.blocks()[p.element_site() as usize].action(),
            Action::CollectionElement { iteration } if *iteration == p.iteration()));
        assert!(matches!(
            plan.blocks()[p.selection_site() as usize].action(),
            Action::ProjectStringMapEntry { .. }
        ));
        let semantic = super::super::fusion_semantics::semantic_actions(&plan).unwrap();
        assert!(matches!(&semantic[p.selection_site() as usize],
            Action::LibraryCall { operands, output, .. }
                if operands.as_ref() == [p.map(), p.element()] && *output == p.output()));
        let counts = super::super::fusion_semantics::semantic_use_counts(&plan).unwrap();
        assert_eq!(counts[p.element().get() as usize], 1);
        assert_eq!(counts[p.output().get() as usize], 1);
        plan.validate_current_visits().unwrap();
    }
}

#[test]
fn string_element_map_projection_rejects_detachment_wrong_visit_replay_and_escapes() {
    for plan in plans(SOURCE) {
        let (id, collection) = plan
            .collections()
            .iter()
            .enumerate()
            .find(|(_, c)| c.string_element_map_projection().is_some())
            .unwrap();
        let p = collection.string_element_map_projection().unwrap();
        for site in [p.element_site(), p.selection_site(), collection.push()] {
            let mut omitted = plan.clone();
            omitted.blocks[site as usize].action = Action::Scalar(Scalar::None);
            assert!(omitted.validate_current_visits().is_err());
        }
        let mut detached = plan.clone();
        detached.collections[id].string_element_map_projection = None;
        assert!(detached.validate_current_visits().is_err());
        let mut wrong_lane = plan.clone();
        wrong_lane.blocks[p.selection_site() as usize].action = Action::ProjectStringMapEntry {
            collection: u32::MAX,
        };
        assert!(wrong_lane.validate_current_visits().is_err());
        let mut wrong_scope = plan.clone();
        wrong_scope.blocks[p.selection_site() as usize].iteration = None;
        assert!(wrong_scope.validate_current_visits().is_err());
        let mut replay = plan.clone();
        replay.blocks[collection.push() as usize].action = Action::ProjectStringMapEntry {
            collection: id as u32,
        };
        assert!(replay.validate_current_visits().is_err());
        let mut ordinary = plan.clone();
        ordinary.blocks[p.selection_site() as usize].action = Action::LibraryCall {
            template: p.selection_template().clone(),
            operands: vec![p.map(), p.element()].into(),
            output: p.output(),
        };
        assert!(ordinary.validate_current_visits().is_err());
        for value in [p.element(), p.output()] {
            let mut escaping = plan.clone();
            escaping.blocks[p.selection_site() as usize].action = Action::Scalar(Scalar::Copy {
                source: value,
                output: if value == p.element() {
                    p.output()
                } else {
                    p.element()
                },
            });
            assert!(escaping.validate_current_visits().is_err());
        }
        let mut wrong_yield = plan.clone();
        wrong_yield.blocks[collection.push() as usize].action = Action::CollectionPush {
            collection: id as u32,
            value: p.element(),
        };
        assert!(wrong_yield.validate_current_visits().is_err());
        let mut forged = plan.clone();
        forged.collections[id]
            .string_element_map_projection
            .as_mut()
            .unwrap()
            .source = p.map();
        assert!(forged.validate_current_visits().is_err());
    }
}

#[test]
fn string_element_map_projection_rejects_same_kind_foreign_origin_template() {
    let source = format!("first := {SOURCE}second := {SOURCE}first + second\n");
    for mut plan in plans(&source) {
        let projections = plan
            .collections()
            .iter()
            .enumerate()
            .filter_map(|(id, c)| c.string_element_map_projection().map(|p| (id, p.clone())))
            .collect::<Vec<_>>();
        assert_eq!(projections.len(), 2);
        let (id, first) = &projections[0];
        let second = &projections[1].1;
        assert_eq!(
            first.selection_template().primitive(),
            second.selection_template().primitive()
        );
        assert_eq!(
            first.selection_template().selected_key(),
            second.selection_template().selected_key()
        );
        assert_eq!(
            first.selection_template().inputs(),
            second.selection_template().inputs()
        );
        assert_eq!(
            first.selection_template().output(),
            second.selection_template().output()
        );
        assert_eq!(
            first.selection_template().contract(),
            second.selection_template().contract()
        );
        assert_eq!(
            first.selection_template().program_revision(),
            second.selection_template().program_revision()
        );
        assert_ne!(
            first.selection_template().origin(),
            second.selection_template().origin()
        );
        plan.collections[*id]
            .string_element_map_projection
            .as_mut()
            .unwrap()
            .selection_template = second.selection_template().clone();
        assert!(plan.validate_current_visits().is_err());
    }
}

#[test]
fn string_element_map_projection_declines_observed_keys_other_work_and_non_string_sources() {
    for source in [
        "text/join([(entries<{string:string}>){piece} + piece | piece <- source<string>], separator<string>)\n",
        "text/join([(entries<{string:string}>){piece} + \"extra\" | piece <- source<string>], separator<string>)\n",
        "text/join([(entries<{string:string}>){piece} | piece <- source<string>, piece != \"\"] , separator<string>)\n",
        "text/join([(entries<{string:string}>){piece} | piece <- words<[string]>], separator<string>)\n",
        "pieces := [(entries<{string:string}>){piece} | piece <- source<string>]\ntext/join(pieces, separator<string>)\npieces\n",
    ] {
        for plan in plans(source) {
            assert!(
                plan.collections()
                    .iter()
                    .all(|c| c.string_element_map_projection().is_none()),
                "{source}"
            );
            plan.validate_current_visits().unwrap();
        }
    }
}
