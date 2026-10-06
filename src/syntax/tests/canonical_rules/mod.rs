//! Exact-rule parser contracts owned by the private canonical test module.

mod canonical_base_contracts;
mod canonical_declaration_properties;
mod canonical_declaration_rules;
mod canonical_declaration_typed_views;
mod canonical_document_conformance;
mod canonical_document_root;
mod canonical_document_statement_views;
mod canonical_escaped_character_parity;
mod canonical_executable_ambiguity;
mod canonical_executable_boundary_regressions;
mod canonical_executable_complexity;
mod canonical_executable_conformance;
mod canonical_executable_continuation_review;
mod canonical_executable_kind_suffix_recovery;
mod canonical_executable_later_continuations;
mod canonical_executable_operator_owner_recovery;
mod canonical_executable_owner_restarts;
mod canonical_executable_piece_backed;
mod canonical_executable_recognition;
mod canonical_executable_recovered_comprehension_views;
mod canonical_executable_recovered_owner_views;
mod canonical_executable_recovery;
mod canonical_executable_resource_limits;
mod canonical_executable_review_regressions;
mod canonical_executable_rule_surface;
mod canonical_executable_selected_owner_views;
mod canonical_executable_structure_review;
mod canonical_executable_structure_suffix_recovery;
mod canonical_executable_token_comparison;
mod canonical_executable_typed_views;
mod canonical_grammar_properties;
mod canonical_graphemes;
mod canonical_literal_closed_rules;
mod canonical_literal_properties;
mod canonical_literal_recovery;
mod canonical_literal_typed_views;
mod canonical_mechdown_closed_rules;
mod canonical_mechdown_payloads;
mod canonical_mechdown_properties;
mod canonical_module_import_properties;
mod canonical_module_import_recovery;
mod canonical_module_import_rules;
mod canonical_module_import_typed_views;
mod canonical_operator_closed_rules;
mod canonical_operator_properties;
mod canonical_operator_typed_views;
mod canonical_primitive_kind_rules;
mod canonical_primitive_properties;
mod canonical_recursion_recovery;
mod canonical_source_import_rules;
mod canonical_source_import_validation;
mod canonical_structure_properties;

/// Lexical cases shared by the fixed examples and UTF-8 totality property.
fn lexical_rules() -> Vec<(&'static str, crate::document::RuleId)> {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/design/grammar-audit/ports.tsv"
    ))
    .lines()
    .skip(1)
    .filter_map(|line| {
        let fields = line.split('\t').collect::<Vec<_>>();
        let name = fields[0];
        (fields[6] == "lexical"
            && (fields[1] == "base"
                || matches!(
                    name,
                    "left-angle" | "right-angle" | "box-drawing-char" | "box-drawing-emoji" | "tag"
                )))
        .then(|| {
            (
                name,
                crate::document::parser::canonical_rule_id(name).unwrap(),
            )
        })
    })
    .collect()
}
