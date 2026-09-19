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
            let expected = self.schema_draft_of(base)?;
            return self.conform_assignment_value(replacement, expected, value_syntax);
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
        if items.len() > 1
            && arithmetic.is_some()
            && matches!(&schema.body, SchemaBody::Matrix { element, .. }
                if !matches!(element.as_ref(), SchemaBody::Matrix { .. } | SchemaBody::Record(_) | SchemaBody::Table { .. } | SchemaBody::Tuple(_) | SchemaBody::Map { .. }))
            && items.iter().all(|item| {
                matches!(
                    item,
                    SubscriptItemSyntax::Bracket(_) | SubscriptItemSyntax::Brace(_)
                )
            })
        {
            return self.document_nested_matrix_update(
                base,
                items,
                replacement,
                arithmetic.unwrap(),
                statement,
                value_syntax,
            );
        }
        let read_selection = !remaining.is_empty()
            || (arithmetic.is_some() && !matches!(schema.body, SchemaBody::Matrix { .. }));
        let (selected, selected_schema, selectors, operation) = match item {
            SubscriptItemSyntax::Bracket(bracket) => {
                let selectors = self.subscript_values(&bracket.values())?;
                self.document_assignment_selection(base, selectors, item.syntax(), read_selection)?
            }
            SubscriptItemSyntax::Brace(brace) => {
                let selectors = self.subscript_values(&brace.values())?;
                self.document_assignment_selection(base, selectors, item.syntax(), read_selection)?
            }
            SubscriptItemSyntax::Dot(dot) => {
                let field = self.required(dot.identifier(), dot.syntax(), "a selected field")?;
                let name = node_text(field.syntax())?;
                let selected = self.select_field(base, &name, item.syntax())?;
                let selector = self.constant_exact(
                    SchemaBody::Id,
                    ValueDataDraft::Id(mech_core::hash_str(&name)),
                );
                (
                    Some(selected),
                    self.schema_draft_of(selected)?,
                    vec![selector],
                    "core/assign/collection-entry",
                )
            }
            SubscriptItemSyntax::DotInteger(dot) => {
                let integer = self.required(dot.integer(), dot.syntax(), "a selected ordinal")?;
                let text = canonical_numeric_text(integer.syntax())?;
                let suffix = integer_literal_suffix(&integer)?;
                let (schema, data) = decode_number(&text, None, suffix)
                    .ok_or_else(|| missing_kind_child(dot.syntax(), "a valid selected ordinal"))?;
                let selector = self.constant(schema, data);
                self.document_assignment_selection(
                    base,
                    vec![Some(selector)],
                    item.syntax(),
                    read_selection,
                )?
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
        if remaining.is_empty()
            && let Some(arithmetic) = arithmetic
            && matches!(schema.body, SchemaBody::Matrix { .. })
            && matches!(
                operation,
                "core/assign/indexed-axis"
                    | "core/assign/indexed-rows"
                    | "core/assign/indexed-columns"
                    | "core/assign/indexed-rectangle"
            )
        {
            // A gather followed by arithmetic and replacement loses repeated
            // selector occurrences. Carry the update into the addressed RMW
            // owner so each occurrence reads the current candidate value.
            let declaration = self.source_type_declaration(arithmetic).map_err(|_| {
                internal(
                    SourceSemanticAnchor::for_node(statement),
                    format!("assignment operation {arithmetic} has no maintained type declaration"),
                )
            })?;
            let (inputs, _) = self.resolve_declared_call_with_destination(
                arithmetic,
                vec![base, replacement],
                statement,
                declaration,
                Some(selected_schema),
            )?;
            let replacement = inputs[1];
            let operation = format!("{operation}/{}", arithmetic.strip_prefix("math/").unwrap());
            let mut inputs = vec![base, replacement];
            inputs.extend(selectors);
            return Ok(self.emit_with_schema_draft(
                &operation,
                inputs,
                schema,
                statement,
                "state-update",
                None,
            ));
        }
        let replacement = if remaining.is_empty() && arithmetic.is_none() {
            self.conform_assignment_value(replacement, selected_schema, value_syntax)?
        } else {
            self.document_selected_update(
                selected.expect("nested and arithmetic assignments retain the selected read"),
                remaining,
                replacement,
                arithmetic,
                statement,
                value_syntax,
            )?
        };
        let mut inputs = vec![base, replacement];
        inputs.extend(selectors);
        Ok(self.emit_with_schema_draft(operation, inputs, schema, statement, "state-update", None))
    }

    fn document_selection_order(
        &mut self,
        value: PendingValue,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let mut schema = self.schema_draft_of(value)?;
        let SchemaBody::Matrix { dimensions, .. } = &mut schema.body else {
            return Ok(value);
        };
        *dimensions = vec![
            DimensionExpr::Multiply(dimensions.clone()),
            DimensionExpr::Constant(1),
        ]
        .into_boxed_slice();
        Ok(self.emit_with_schema_draft(
            "core/assign/selection-order",
            vec![value],
            schema,
            syntax,
            "selection-addresses",
            None,
        ))
    }

    fn document_nested_matrix_update(
        &mut self,
        base: PendingValue,
        items: &[SubscriptItemSyntax],
        replacement: PendingValue,
        arithmetic: &str,
        statement: &SyntaxNode,
        value_syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let schema = self.schema_draft_of(base)?;
        let SchemaBody::Matrix { element, .. } = &schema.body else {
            unreachable!()
        };
        let mut index_schema = schema.clone();
        if let SchemaBody::Matrix { element, .. } = &mut index_schema.body {
            *element = Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64));
        }
        let mut indices = self.emit_with_schema_draft(
            "core/assign/identity-indices",
            vec![base],
            index_schema,
            statement,
            "selection-addresses",
            None,
        );
        for item in items {
            indices = self.select(indices, item)?;
        }
        let mut selected_schema = self.schema_draft_of(indices)?;
        match &mut selected_schema.body {
            SchemaBody::Matrix {
                element: selected, ..
            } => *selected = element.clone(),
            body => *body = *element.clone(),
        }
        let incoming = self.schema_draft_of(replacement)?;
        let incoming_element = match &incoming.body {
            SchemaBody::Matrix { element, .. } => element.as_ref(),
            scalar => scalar,
        };
        let replacement = if incoming_element == element.as_ref() {
            let replacement = self.document_assignment_broadcast(
                replacement,
                selected_schema.clone(),
                value_syntax,
            )?;
            self.conform_assignment_value(replacement, selected_schema.clone(), value_syntax)?
        } else {
            let mut selected = base;
            for item in items {
                selected = self.select(selected, item)?;
            }
            let Some((inputs, _)) =
                self.resolve_maintained_call(arithmetic, vec![selected, replacement], statement)?
            else {
                return Err(internal(
                    SourceSemanticAnchor::for_node(statement),
                    format!("assignment operation {arithmetic} has no maintained type declaration"),
                ));
            };
            inputs[1]
        };
        let replacement =
            self.document_assignment_broadcast(replacement, selected_schema, value_syntax)?;
        let indices = self.document_selection_order(indices, statement)?;
        let replacement = self.document_selection_order(replacement, value_syntax)?;
        let operation = format!(
            "core/assign/indexed-axis/{}",
            arithmetic.strip_prefix("math/").unwrap()
        );
        Ok(self.emit_with_schema_draft(
            &operation,
            vec![base, replacement, indices],
            schema,
            statement,
            "state-update",
            None,
        ))
    }

    fn document_assignment_broadcast(
        &mut self,
        value: PendingValue,
        mut selected: SchemaDraft,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        let incoming = self.schema_draft_of(value)?;
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = &incoming.body
        else {
            return Ok(value);
        };
        let SchemaBody::Matrix {
            element: target,
            dimensions: target_dimensions,
        } = &mut selected.body
        else {
            return Ok(value);
        };
        if dimensions == target_dimensions {
            return Ok(value);
        }
        // Preserve the arithmetic operand kind and materialize its maintained
        // broadcast before flattening nested selection addresses.
        *target = element.clone();
        Ok(self.emit_with_schema_draft(
            "core/assign/broadcast",
            vec![value],
            selected,
            syntax,
            "assignment-broadcast",
            None,
        ))
    }

    fn conform_assignment_value(
        &mut self,
        replacement: PendingValue,
        mut expected: SchemaDraft,
        syntax: &SyntaxNode,
    ) -> Result<PendingValue, SourceSemanticError> {
        // Maintained matrix assignment broadcasts scalar replacement values.
        if !matches!(
            self.schema_draft_of(replacement)?.body,
            SchemaBody::Matrix { .. }
        ) && let SchemaBody::Matrix { element, .. } = &expected.body
        {
            expected.body = *element.clone();
        }
        self.conform_schema_draft(
            replacement,
            &expected,
            syntax,
            "source-semantics/incompatible-assignment-kind",
            "assignment value does not satisfy the selected binding's schema",
        )
    }

    fn document_assignment_selection(
        &mut self,
        base: PendingValue,
        selectors: Vec<Option<PendingValue>>,
        syntax: &SyntaxNode,
        read: bool,
    ) -> Result<
        (
            Option<PendingValue>,
            SchemaDraft,
            Vec<PendingValue>,
            &'static str,
        ),
        SourceSemanticError,
    > {
        // Validate once through the read-selection authority, but emit a read
        // only when nested or mixed-kind arithmetic actually consumes it.
        // Whole-value writes preserve the destination geometry.
        let selection = if matches!(selectors.as_slice(), [None] | [None, None]) {
            PendingSelection {
                operation: None,
                inputs: vec![base],
                schema: self.schema_draft_of(base)?,
            }
        } else {
            self.prepare_selection(base, selectors.clone(), syntax)?
        };
        let selected_schema = selection.schema.clone();
        let selected =
            (read || selection.operation.is_none()).then(|| self.emit_selection(selection, syntax));
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
            selected_schema,
            selectors.into_iter().flatten().collect(),
            operation,
        ))
    }
}
