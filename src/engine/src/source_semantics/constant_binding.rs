use super::*;

impl CanonicalSourceProgram {
    /// Replace selected canonical inputs with immutable values completed by
    /// already-resolved source dependencies.
    ///
    /// The program and supplied values are rebound to a canonical schema union
    /// and interned in a rebuilt constant store. Every remaining input is
    /// renumbered in one pass, so the resulting artifact has no hidden initializer side table or
    /// runtime dependency on a compiler-owned cell.
    pub fn bind_input_constants(
        mut self,
        bindings: &[(u32, Value)],
    ) -> Result<Self, SourceSemanticError> {
        if bindings.is_empty() {
            return Ok(self);
        }
        let anchor = |input: usize| {
            self.source_map
                .inputs
                .get(input)
                .copied()
                .unwrap_or(SourceSemanticAnchor {
                    document: DocumentId(0),
                    revision: Revision(0),
                    range: TextRange::empty(mech_syntax::document::TextSize::ZERO),
                })
        };
        let mut selected_values = BTreeMap::new();
        for (input, value) in bindings {
            let input = *input as usize;
            let Some(declaration) = self.program.inputs.get(input) else {
                return Err(SourceSemanticError {
                    code: "source-semantics/input-binding-out-of-range",
                    message: format!("canonical input binding {input} is out of range"),
                    anchor: anchor(input),
                });
            };
            if selected_values.contains_key(&input) {
                return Err(SourceSemanticError {
                    code: "source-semantics/duplicate-input-binding",
                    message: format!(
                        "canonical input {:?} was bound more than once",
                        declaration.name
                    ),
                    anchor: anchor(input),
                });
            }
            selected_values.insert(input, value.clone());
        }
        // Detached values may own schemas that sort before existing entries.
        // Build the union through the canonical table builder, then relocate
        // every program and constant reference before publishing the result.
        let mut schemas = SchemaTableBuilder::new();
        for entry in self.schemas.entries() {
            schemas
                .insert(entry.schema().clone())
                .map_err(|error| SourceSemanticError {
                    code: "source-semantics/import-value-schema-mismatch",
                    message: format!("unable to retain a canonical program schema: {error:?}"),
                    anchor: anchor(0),
                })?;
        }
        for value in selected_values.values() {
            let imported = value.schemas().ok_or_else(|| SourceSemanticError {
                code: "source-semantics/import-value-schema-mismatch",
                message: "resolved canonical import value has no detached schema owner".to_owned(),
                anchor: anchor(0),
            })?;
            for entry in imported.entries() {
                schemas
                    .insert(entry.schema().clone())
                    .map_err(|error| SourceSemanticError {
                        code: "source-semantics/import-value-schema-mismatch",
                        message: format!(
                            "unable to retain a resolved canonical import schema: {error:?}"
                        ),
                        anchor: anchor(0),
                    })?;
            }
        }
        let (schemas, schema_ids) = schemas
            .finish()
            .map_err(|error| SourceSemanticError {
                code: "source-semantics/import-value-schema-mismatch",
                message: format!("unable to finalize canonical import schemas: {error:?}"),
                anchor: anchor(0),
            })?
            .into_parts();
        self.schemas = schemas;
        let selected = selected_values
            .into_iter()
            .map(|(input, value)| {
                let declaration = &self.program.inputs[input];
                let schema = schema_ids[declaration.schema.get() as usize];
                let target = self
                    .schemas
                    .get(schema)
                    .expect("canonical input schema is validated");
                // A Dynamic input already carries its payload identity. Only a
                // concrete supplied value needs the boundary's Dynamic wrapper.
                let rebound = if matches!(target.body(), SchemaBody::Dynamic)
                    && !matches!(value.data(), mech_core::ValueData::Dynamic(_))
                {
                    let concrete_schema = self
                        .schemas
                        .find_by_key(value.schema_key())
                        .ok_or_else(|| SourceSemanticError {
                            code: "source-semantics/import-value-schema-mismatch",
                            message: format!(
                                "resolved value schema {:?} is absent from the canonical program",
                                value.schema_key()
                            ),
                            anchor: anchor(input),
                        })?;
                    let concrete = value
                        .rebind(concrete_schema, value.shape(), &self.schemas)
                        .map_err(|error| SourceSemanticError {
                            code: "source-semantics/import-value-schema-mismatch",
                            message: format!(
                                "resolved value for canonical input {:?} has an incompatible schema: {error:?}",
                                declaration.name
                            ),
                            anchor: anchor(input),
                        })?;
                    ValueDraft {
                        schema,
                        shape_values: Box::new([]),
                        data: ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                            schema: concrete_schema,
                            shape_values: concrete
                                .shape()
                                .parameter_values()
                                .to_vec()
                                .into_boxed_slice(),
                            data: concrete.canonical_data_draft().map_err(|error| {
                                SourceSemanticError {
                                    code: "source-semantics/import-value-schema-mismatch",
                                    message: format!(
                                        "unable to retain canonical import data: {error:?}"
                                    ),
                                    anchor: anchor(input),
                                }
                            })?,
                        }))),
                    }
                    .finalize(&SnapshotValidationContext::new(&self.schemas))
                    .map_err(|error| SourceSemanticError {
                        code: "source-semantics/import-value-schema-mismatch",
                        message: format!(
                            "unable to materialize dynamic canonical import value: {error:?}"
                        ),
                        anchor: anchor(input),
                    })?
                } else {
                    // A closed planning schema owns no dimension parameters,
                    // even when the supplied snapshot uses a parameterized
                    // representation of the same concrete shape.
                    let shape = if target.dimension_parameters().is_empty() {
                        target.instantiate_shape(Box::new([])).map_err(|error| {
                            SourceSemanticError {
                                code: "source-semantics/import-value-schema-mismatch",
                                message: format!("unable to instantiate input shape: {error:?}"),
                                anchor: anchor(input),
                            }
                        })?
                    } else {
                        let source_schemas = value.schemas().expect("bound values retain their schemas");
                        let source_schema = source_schemas.get(value.schema()).expect("bound value schema is retained");
                        let error = |message| SourceSemanticError {
                            code: "source-semantics/import-value-schema-mismatch",
                            message,
                            anchor: anchor(input),
                        };
                        let closed = source_schema.closed_body(value.shape())
                            .map_err(|failure| error(format!("invalid bound value shape: {failure:?}")))?;
                        mech_core::shape_for_schema_components(
                            target, &[(target.body(), closed)], None,
                        ).map_err(|failure| error(format!("incompatible bound input shape: {failure}")))?
                    };
                    value
                        .rebind(schema, &shape, &self.schemas)
                        .map_err(|error| SourceSemanticError {
                            code: "source-semantics/import-value-schema-mismatch",
                            message: format!(
                                "resolved value for canonical input {:?} has an incompatible schema: {error:?}",
                                declaration.name
                            ),
                            anchor: anchor(input),
                        })?
                };
                Ok((input, rebound))
            })
            .collect::<Result<BTreeMap<_, _>, SourceSemanticError>>()?;

        let mut constants = ConstantStoreBuilder::new(&self.schemas);
        let old_handles = (0..self.constants.len())
            .map(|ordinal| {
                let id = ConstantId::new(ordinal as u32);
                let value = self
                    .constants
                    .get(id)
                    .expect("constant ordinal is bounded by the store length");
                let value = value.rebind(
                    schema_ids[value.schema().get() as usize],
                    value.shape(),
                    &self.schemas,
                )?;
                constants.insert(value)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SourceSemanticError {
                code: "source-semantics/import-constant-invalid",
                message: format!("unable to retain canonical constants: {error:?}"),
                anchor: anchor(0),
            })?;
        let bound_handles = selected
            .iter()
            .map(|(input, value)| {
                constants
                    .insert(value.clone())
                    .map(|handle| (*input, handle))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(|error| SourceSemanticError {
                code: "source-semantics/import-constant-invalid",
                message: format!("unable to retain canonical import value: {error:?}"),
                anchor: anchor(0),
            })?;
        let build = constants.finish().map_err(|error| SourceSemanticError {
            code: "source-semantics/import-constant-invalid",
            message: format!("unable to finalize canonical import values: {error:?}"),
            anchor: anchor(0),
        })?;
        let old_constants = old_handles
            .into_iter()
            .map(|handle| build.resolve(handle))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SourceSemanticError {
                code: "source-semantics/import-constant-invalid",
                message: format!("unable to remap canonical constants: {error:?}"),
                anchor: anchor(0),
            })?;
        let bound_constants = bound_handles
            .into_iter()
            .map(|(input, handle)| build.resolve(handle).map(|constant| (input, constant)))
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(|error| SourceSemanticError {
                code: "source-semantics/import-constant-invalid",
                message: format!("unable to remap canonical import values: {error:?}"),
                anchor: anchor(0),
            })?;

        let remap = |value: &mut SourceValue| match value {
            SourceValue::Constant(old) => {
                *old = old_constants[old.get() as usize];
            }
            SourceValue::Input(old) => {
                let input = *old as usize;
                if let Some(constant) = bound_constants.get(&input) {
                    *value = SourceValue::Constant(*constant);
                } else {
                    let removed = bound_constants.range(..input).count() as u32;
                    *old -= removed;
                }
            }
            SourceValue::State(_) | SourceValue::NodeOutput { .. } => {}
        };
        let relocation = BindingRelocation {
            schemas: &schema_ids,
            constants: &old_constants,
        };
        for input in &mut self.program.inputs {
            relocation.schema(&mut input.schema);
        }
        for state in &mut self.program.states {
            relocation.schema(&mut state.schema);
            if let Some(initializer) = &mut state.initializer {
                remap(initializer);
            }
        }
        for node in &mut self.program.nodes {
            for output in &mut node.outputs {
                match output {
                    SourceNodeOutput::Derived { schema } => relocation.schema(schema),
                    SourceNodeOutput::State(_) => {}
                }
            }
            match &mut node.body {
                crate::SourceNodeBody::Match(declaration) => {
                    relocation.match_declaration(declaration)
                }
                crate::SourceNodeBody::Comprehension(declaration) => {
                    relocation.comprehension(declaration)
                }
                crate::SourceNodeBody::Operation { .. } | crate::SourceNodeBody::Fsm(_) => {}
            }
            for input in &mut node.inputs {
                remap(input);
            }
        }
        for output in &mut self.program.outputs {
            relocation.schema(&mut output.schema);
            remap(&mut output.source);
        }
        for constraint in &mut self.program.constraints {
            for input in &mut constraint.inputs {
                remap(input);
            }
        }
        self.program.inputs = self
            .program
            .inputs
            .into_vec()
            .into_iter()
            .enumerate()
            .filter_map(|(input, declaration)| {
                (!bound_constants.contains_key(&input)).then_some(declaration)
            })
            .collect();
        self.source_map.inputs = self
            .source_map
            .inputs
            .into_vec()
            .into_iter()
            .enumerate()
            .filter_map(|(input, source)| (!bound_constants.contains_key(&input)).then_some(source))
            .collect();
        self.constants = build.store;
        Ok(self)
    }
}

/// Schema and constant identities share one relocation pass, including lexical
/// control declarations whose operands do not appear in SourceNode::inputs.
struct BindingRelocation<'a> {
    schemas: &'a [SchemaId],
    constants: &'a [ConstantId],
}

impl BindingRelocation<'_> {
    fn schema(&self, schema: &mut SchemaId) {
        *schema = self.schemas[schema.get() as usize];
    }

    fn constant(&self, constant: &mut ConstantId) {
        *constant = self.constants[constant.get() as usize];
    }

    fn control_value(&self, value: &mut crate::ControlValue) {
        match value {
            crate::ControlValue::Constant(id) => self.constant(id),
            crate::ControlValue::Parameter { .. } | crate::ControlValue::Local { .. } => {}
        }
    }

    fn block(&self, block: &mut crate::ControlBlock<OperationContractDeclaration>) {
        for parameter in &mut block.parameters {
            self.schema(&mut parameter.schema);
        }
        for operation in &mut block.operations {
            self.schema(&mut operation.schema);
            for input in &mut operation.inputs {
                self.control_value(input);
            }
            match &mut operation.body {
                crate::ControlOperationBody::Match(declaration) => {
                    self.match_declaration(declaration)
                }
                crate::ControlOperationBody::Comprehension(declaration) => {
                    self.comprehension(declaration)
                }
                crate::ControlOperationBody::Operation { .. } => {}
            }
        }
        self.control_value(&mut block.yield_value);
    }

    fn match_declaration(
        &self,
        declaration: &mut crate::MatchDeclaration<OperationContractDeclaration>,
    ) {
        for capture in &mut declaration.captures {
            self.schema(&mut capture.schema);
        }
        for arm in &mut declaration.arms {
            match &mut arm.pattern {
                crate::MatchPattern::Literal(id) => self.constant(id),
                crate::MatchPattern::Wildcard | crate::MatchPattern::Bind => {}
                crate::MatchPattern::Structural(pattern) => self.match_structural_pattern(pattern),
            }
            if let Some(guard) = &mut arm.guard {
                self.block(guard);
            }
            self.block(&mut arm.body);
        }
    }

    fn match_structural_pattern(
        &self,
        pattern: &mut crate::CollectionPattern<mech_core::SchemaId, crate::MatchPatternValue>,
    ) {
        match pattern {
            crate::CollectionPattern::Wildcard => {}
            crate::CollectionPattern::Bind { schema, .. } => self.schema(schema),
            crate::CollectionPattern::Equal(crate::MatchPatternValue::Literal(id)) => {
                self.constant(id)
            }
            crate::CollectionPattern::Equal(crate::MatchPatternValue::Binding(_)) => {}
            crate::CollectionPattern::Enum { payload, .. } => {
                if let Some(payload) = payload {
                    self.match_structural_pattern(payload);
                }
            }
            crate::CollectionPattern::Tuple(items) => {
                for item in items {
                    self.match_structural_pattern(item);
                }
            }
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                for item in prefix.iter_mut().chain(suffix.iter_mut()) {
                    self.match_structural_pattern(item);
                }
                if let Some(rest) = rest {
                    self.match_structural_pattern(rest);
                }
            }
        }
    }

    fn comprehension_value(&self, value: &mut crate::ComprehensionValue) {
        match value {
            crate::ComprehensionValue::Constant(id) => self.constant(id),
            crate::ComprehensionValue::Input(_) | crate::ComprehensionValue::Local(_) => {}
        }
    }

    fn collection_pattern(&self, pattern: &mut crate::CollectionPattern) {
        match pattern {
            crate::CollectionPattern::Wildcard => {}
            crate::CollectionPattern::Bind { schema, .. } => self.schema(schema),
            crate::CollectionPattern::Equal(value) => self.comprehension_value(value),
            crate::CollectionPattern::Enum { payload, .. } => {
                if let Some(payload) = payload {
                    self.collection_pattern(payload);
                }
            }
            crate::CollectionPattern::Tuple(items) => {
                for item in items {
                    self.collection_pattern(item);
                }
            }
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                for item in prefix.iter_mut().chain(suffix.iter_mut()) {
                    self.collection_pattern(item);
                }
                if let Some(rest) = rest {
                    self.collection_pattern(rest);
                }
            }
        }
    }

    fn comprehension(
        &self,
        declaration: &mut crate::ComprehensionDeclaration<OperationContractDeclaration>,
    ) {
        for step in &mut declaration.steps {
            match step {
                crate::ComprehensionStep::Generator { source, pattern } => {
                    self.comprehension_value(source);
                    self.collection_pattern(pattern);
                }
                crate::ComprehensionStep::Operation(operation) => {
                    self.schema(&mut operation.schema);
                    for input in &mut operation.inputs {
                        self.comprehension_value(input);
                    }
                    match &mut operation.body {
                        crate::ControlOperationBody::Operation { .. } => {}
                        crate::ControlOperationBody::Match(declaration) => {
                            self.match_declaration(declaration)
                        }
                        crate::ControlOperationBody::Comprehension(declaration) => {
                            self.comprehension(declaration)
                        }
                    }
                }
                crate::ComprehensionStep::Filter(value) => self.comprehension_value(value),
            }
        }
        self.comprehension_value(&mut declaration.yield_value);
    }
}
