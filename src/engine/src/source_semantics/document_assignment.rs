//! Selected state updates use the maintained selection and assignment operations.

use super::*;

impl SemanticBuilder {
    pub(super) fn document_selected_update(
        &mut self,
        base: PendingValue,
        items: &[SubscriptItemSyntax],
        mut replacement: PendingValue,
        arithmetic: Option<&str>,
        statement: &SyntaxNode,
        value_syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let Some((item, remaining)) = items.split_first() else {
            if let Some(operation) = arithmetic {
                let Some((inputs, schema)) =
                    self.resolve_maintained_call(operation, vec![base, replacement], statement)?
                else {
                    return Err(internal(
                        SourceSemanticAnchor::for_node(statement),
                        format!(
                            "assignment operation {operation} has no maintained type declaration"
                        ),
                    ));
                };
                replacement = self.emit_with_schema_draft(
                    operation,
                    inputs,
                    schema,
                    statement,
                    "state-update",
                    None,
                );
            }
            let mut expected = self.schema_draft_of(base)?;
            // Maintained matrix assignment broadcasts scalar replacement values.
            if !matches!(
                self.schema_draft_of(replacement)?.body,
                SchemaBody::Matrix { .. }
            ) && let SchemaBody::Matrix { element, .. } = &expected.body
            {
                expected.body = *element.clone();
            }
            return self.conform_schema_draft(
                replacement,
                &expected,
                value_syntax,
                "source-semantics/incompatible-assignment-kind",
                "assignment value does not satisfy the selected binding's schema",
            );
        };
        let schema = self.schema_draft_of(base)?;
        if !matches!(
            schema.body,
            SchemaBody::Matrix { .. }
                | SchemaBody::Record(_)
                | SchemaBody::Table { .. }
                | SchemaBody::Tuple(_)
                | SchemaBody::Map { .. }
        ) {
            return Err(SourceSemanticError {
                code: "source-semantics/unsupported-assignment-target",
                message: "assignment selection requires a mutable collection".to_owned(),
                anchor: SourceSemanticAnchor::for_node(item.syntax()),
            });
        }
        let (selected, selectors, operation) = match item {
            SubscriptItemSyntax::Bracket(bracket) => {
                let selectors = self.subscript_values(&bracket.values())?;
                self.document_assignment_selection(base, selectors, item.syntax())?
            }
            SubscriptItemSyntax::Brace(brace) => {
                let selectors = self.subscript_values(&brace.values())?;
                self.document_assignment_selection(base, selectors, item.syntax())?
            }
            SubscriptItemSyntax::Dot(dot) => {
                let field = self.required(dot.identifier(), dot.syntax(), "a selected field")?;
                let name = node_text(field.syntax())?;
                let selected = self.select_field(base, &name, item.syntax())?;
                let selector = self.constant_exact(
                    SchemaBody::Id,
                    ValueDataDraft::Id(mech_core::hash_str(&name)),
                );
                (selected, vec![selector], "core/assign/collection-entry")
            }
            SubscriptItemSyntax::DotInteger(dot) => {
                let integer = self.required(dot.integer(), dot.syntax(), "a selected ordinal")?;
                let text = canonical_numeric_text(integer.syntax())?;
                let suffix = integer_literal_suffix(&integer)?;
                let (schema, data) = decode_number(&text, None, suffix)
                    .ok_or_else(|| missing_kind_child(dot.syntax(), "a valid selected ordinal"))?;
                let selector = self.constant(schema, data);
                self.document_assignment_selection(base, vec![Some(selector)], item.syntax())?
            }
            SubscriptItemSyntax::Swizzle(swizzle) => {
                let selected = self.select(base, item)?;
                let replacement = self.document_selected_update(
                    selected,
                    remaining,
                    replacement,
                    arithmetic,
                    statement,
                    value_syntax,
                )?;
                let mut updated = base;
                for (ordinal, name) in swizzle.identifiers().enumerate() {
                    let name = node_text(name.syntax())?;
                    let ordinal = self.constant_exact(
                        SchemaBody::Index,
                        ValueDataDraft::Index((ordinal + 1) as u64),
                    );
                    let value =
                        self.select_values(replacement, vec![Some(ordinal)], item.syntax())?;
                    let field = self.constant_exact(
                        SchemaBody::Id,
                        ValueDataDraft::Id(mech_core::hash_str(&name)),
                    );
                    updated = self.emit_with_schema_draft(
                        "core/assign/collection-entry",
                        vec![updated, value, field],
                        schema.clone(),
                        statement,
                        "state-update",
                        None,
                    );
                }
                return Ok(updated);
            }
        };
        let replacement = self.document_selected_update(
            selected,
            remaining,
            replacement,
            arithmetic,
            statement,
            value_syntax,
        )?;
        let mut inputs = vec![base, replacement];
        inputs.extend(selectors);
        Ok(self.emit_with_schema_draft(operation, inputs, schema, statement, "state-update", None))
    }

    fn document_assignment_selection(
        &mut self,
        base: PendingValue,
        selectors: Vec<Option<PendingValue>>,
        syntax: &SyntaxNode,
    ) -> Result<(PendingValue, Vec<PendingValue>, &'static str), SourceSemanticError> {
        let selected = self.select_values(base, selectors.clone(), syntax)?;
        // Whole-value writes retain the base geometry even though the read
        // spelling matrix[:] exposes a flattened selection view.
        let selected = if matches!(selectors.as_slice(), [None] | [None, None]) {
            base
        } else {
            selected
        };
        let operation = match selectors.as_slice() {
            [None] | [None, None] => "core/assign/whole-value",
            [Some(_), None] => "core/assign/indexed-rows",
            [None, Some(_)] => "core/assign/indexed-columns",
            [Some(_), Some(_)] => "core/assign/indexed-rectangle",
            [Some(_)] => match self.schema_draft_of(base)?.body {
                SchemaBody::Map { .. } => "core/assign/collection-entry",
                SchemaBody::Tuple(_) => "core/assign/single-element",
                _ => "core/assign/indexed-axis",
            },
            _ => unreachable!("selection validation checks arity"),
        };
        Ok((
            selected,
            selectors.into_iter().flatten().collect(),
            operation,
        ))
    }
}
