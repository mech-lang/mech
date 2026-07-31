//! Compatibility lowering for the closed Phase 2H structure shell.

use alloc::string::String;

use mech_core::{Map, Set, TableRow};

use crate::document::ast::structure_shell::{
    EmptyMapSyntax, EmptySetSyntax, TableRowSeparatorSyntax,
};
use crate::document::{AstNode, DiagnosticStore, SyntaxKind, SyntaxNode};

use super::common;

/// Direct compatibility values emitted by the node-valued Phase 2H leaves.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LegacyStructureShellValue {
    TableRow(TableRow),
    EmptyMap(Map),
    EmptySet(Set),
}

/// Lower a structural row separator to the legacy empty table row.
pub fn lower_legacy_table_row_separator(
    syntax: &TableRowSeparatorSyntax,
) -> Result<TableRow, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::TableRowSeparator,
        "table row separator",
        || TableRow { columns: vec![] },
    )
}

/// Lower a closed empty map to its compatibility collection value.
pub fn lower_legacy_empty_map(syntax: &EmptyMapSyntax) -> Result<Map, DiagnosticStore> {
    lower_value(syntax.syntax(), SyntaxKind::EmptyMap, "empty map", || Map {
        elements: vec![],
    })
}

/// Lower every accepted empty-set spelling to the same compatibility value.
pub fn lower_legacy_empty_set(syntax: &EmptySetSyntax) -> Result<Set, DiagnosticStore> {
    lower_value(syntax.syntax(), SyntaxKind::EmptySet, "empty set", || Set {
        elements: vec![],
    })
}

/// Lower a node-valued Phase 2H leaf for direct differential coverage without
/// introducing a complete structure parent lowerer.
pub(crate) fn lower_phase_2h_structure_shell_value(
    syntax: &SyntaxNode,
) -> Result<LegacyStructureShellValue, DiagnosticStore> {
    let lowered = match syntax.kind() {
        SyntaxKind::TableRowSeparator => checked_value(
            syntax,
            SyntaxKind::TableRowSeparator,
            "table row separator",
            || LegacyStructureShellValue::TableRow(TableRow { columns: vec![] }),
        ),
        SyntaxKind::EmptyMap => checked_value(syntax, SyntaxKind::EmptyMap, "empty map", || {
            LegacyStructureShellValue::EmptyMap(Map { elements: vec![] })
        }),
        SyntaxKind::EmptySet => checked_value(syntax, SyntaxKind::EmptySet, "empty set", || {
            LegacyStructureShellValue::EmptySet(Set { elements: vec![] })
        }),
        _ => Err(String::from(
            "expected a node-valued Phase 2H structure-shell primitive",
        )),
    };
    lowered.map_err(|message| {
        common::failure_store(syntax, "lowering/invalid-structure-shell-syntax", message)
    })
}

fn lower_value<T>(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &'static str,
    value: impl FnOnce() -> T,
) -> Result<T, DiagnosticStore> {
    checked_value(syntax, expected_kind, name, value).map_err(|message| {
        common::failure_store(syntax, "lowering/invalid-structure-shell-syntax", message)
    })
}

fn checked_value<T>(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &'static str,
    value: impl FnOnce() -> T,
) -> Result<T, String> {
    common::validate_node(syntax, expected_kind, name)?;
    Ok(value())
}
