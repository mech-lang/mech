//! Unary underscore sugar over the public registration/signature query.
//! The selected call still uses ordinary conversions/contracts/canonical IR.
use super::*;

#[derive(Clone, Copy)]
enum AddressBase {
    Binding(PendingBinding),
    Input(u32),
    DeclaredInput,
}
struct AddressSelection {
    base_name: String,
    base: AddressBase,
    canonical_name: String,
    declaration: mech_core::FunctionTypeDeclaration,
}
impl SemanticBuilder {
    fn exact_source_name(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
            || self.input_by_name.contains_key(name)
            || self.input_schema_overrides.contains_key(name)
            || self.input_declarations.contains_key(name)
    }
    fn address_base(
        &self,
        name: &str,
    ) -> Result<Option<(AddressBase, SchemaDraft)>, SourceSemanticError> {
        if let Some(binding) = self.bindings.get(name).copied() {
            let value = match binding {
                PendingBinding::Value(value) => value,
                PendingBinding::MutableState(state) => self.current_state_value(state),
            };
            return Ok(Some((
                AddressBase::Binding(binding),
                self.schema_draft_of(value)?,
            )));
        }
        if let Some(&input) = self.input_by_name.get(name) {
            return Ok(Some((
                AddressBase::Input(input),
                self.inputs[input as usize].schema.clone(),
            )));
        }
        Ok(self
            .input_schema_overrides
            .get(name)
            .or_else(|| self.input_declarations.get(name))
            .cloned()
            .map(|schema| (AddressBase::DeclaredInput, schema)))
    }
    fn select_underscore_address(
        &self,
        name: &str,
        syntax: &SyntaxNode,
    ) -> Result<Option<AddressSelection>, SourceSemanticError> {
        if self.exact_source_name(name) || !name.contains('_') {
            return Ok(None);
        }
        let (Some(catalog), Some(environment)) = (
            self.function_catalog.as_ref(),
            self.function_environment.as_ref(),
        ) else {
            // Catalog-free probes must not invent a second registry or scope.
            return Ok(None);
        };
        let excluded = self
            .resolved_source_modules
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        // Ordinary calls and applications prefer an exact document function
        // before consulting the catalog environment. Supply that same lexical
        // shadowing here, without disabling unshadowed aliases of an operation.
        let excluded_names = self
            .local_functions
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let mut selections = BTreeMap::new();
        let mut named = BTreeSet::new();
        let mut unresolved = false;
        for (at, _) in name.char_indices().filter(|(_, ch)| *ch == '_') {
            let (base_name, suffix) = (&name[..at], &name[at + 1..]);
            if base_name.is_empty() || suffix.is_empty() {
                continue;
            }
            let Some((base, schema)) = self.address_base(base_name)? else {
                continue;
            };
            let receiver = if matches!(schema.body, SchemaBody::Dynamic) {
                None
            } else {
                Some(
                    ResolvedType::from_schema_body(&schema.body, &schema.dimension_parameters)
                        .map_err(|error| {
                            internal(
                                SourceSemanticAnchor::for_node(syntax),
                                format!("invalid address receiver kind: {error}"),
                            )
                        })?,
                )
            };
            for member in environment.receiver_members(
                catalog,
                receiver.as_ref(),
                crate::ReceiverMemberQuery {
                    role: crate::ReceiverMemberRole::UnaryPureProjection,
                    exact_name: Some(suffix),
                    excluded_module_roots: &excluded,
                    excluded_names: &excluded_names,
                    include_unavailable: true,
                },
            ) {
                named.insert(member.canonical_name.clone());
                match member.compatibility {
                    crate::ReceiverMemberCompatibility::Available { .. } => {}
                    crate::ReceiverMemberCompatibility::PrefixCompatible { .. }
                    | crate::ReceiverMemberCompatibility::ArgumentsRequired => continue,
                    crate::ReceiverMemberCompatibility::UnresolvedReceiver => {
                        unresolved = true;
                        continue;
                    }
                    crate::ReceiverMemberCompatibility::Ambiguous(error) => {
                        return Err(SourceSemanticError {
                            code: "source-semantics/ambiguous-underscore-address",
                            message: format!(
                                "{} has ambiguous receiver overloads: {error}",
                                member.canonical_name
                            ),
                            anchor: SourceSemanticAnchor::for_node(syntax),
                        });
                    }
                    crate::ReceiverMemberCompatibility::Incompatible
                    | crate::ReceiverMemberCompatibility::NotVisible => continue,
                }
                selections
                    .entry((base_name.to_owned(), member.canonical_name.clone()))
                    .or_insert_with(|| AddressSelection {
                        base_name: base_name.to_owned(),
                        base,
                        canonical_name: member.canonical_name,
                        declaration: member.declaration,
                    });
            }
        }
        if selections.len() > 1 {
            let choices = selections
                .values()
                .map(|choice| format!("{}({})", choice.canonical_name, choice.base_name))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(SourceSemanticError {
                code: "source-semantics/ambiguous-underscore-address",
                message: format!(
                    "{name} has multiple type-compatible unary addresses: {choices}; use an explicit qualified call"
                ),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        if let Some((_, selection)) = selections.into_iter().next() {
            return Ok(Some(selection));
        }
        if named.is_empty() {
            // Not a registered unary address: retain ordinary underscore names.
            return Ok(None);
        }
        Err(SourceSemanticError {
            code: if unresolved {
                "source-semantics/unresolved-underscore-address"
            } else {
                "source-semantics/unavailable-underscore-address"
            },
            message: format!(
                "{name} has no visible unary projection for its receiver kind; candidates: {}; import the intended module and give its receiver a concrete kind, or use an explicit qualified call",
                named.into_iter().collect::<Vec<_>>().join(", ")
            ),
            anchor: SourceSemanticAnchor::for_node(syntax),
        })
    }
    pub(super) fn underscore_address(
        &mut self,
        name: &str,
        syntax: &SyntaxNode,
    ) -> Result<Option<PendingValue>, SourceSemanticError> {
        let Some(selection) = self.select_underscore_address(name, syntax)? else {
            return Ok(None);
        };
        let value = match selection.base {
            AddressBase::Binding(binding) => self.read_document_binding(binding, syntax)?,
            AddressBase::Input(input) => {
                self.require_function_local_binding(&selection.base_name, syntax)?;
                PendingValue::Input(input)
            }
            AddressBase::DeclaredInput => {
                self.require_function_local_binding(&selection.base_name, syntax)?;
                let schema = self
                    .input_schema_overrides
                    .get(&selection.base_name)
                    .or_else(|| self.input_declarations.get(&selection.base_name))
                    .cloned()
                    .ok_or_else(|| {
                        internal(
                            SourceSemanticAnchor::for_node(syntax),
                            "selected receiver disappeared".to_owned(),
                        )
                    })?;
                let input = u32::try_from(self.inputs.len()).map_err(|_| SourceSemanticError {
                    code: "source-semantics/input-identity-exhausted",
                    message: "canonical input count exceeds SourceProgram identity space"
                        .to_owned(),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                })?;
                self.input_by_name
                    .insert(selection.base_name.clone(), input);
                self.inputs.push(PendingInput {
                    name: selection.base_name,
                    schema,
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
                PendingValue::Input(input)
            }
        };
        let (inputs, output) = self.resolve_declared_call(
            &selection.canonical_name,
            vec![value],
            syntax,
            selection.declaration,
        )?;
        Ok(Some(self.emit_with_schema_draft(
            &selection.canonical_name,
            inputs,
            output,
            syntax,
            "call",
            Some(format!("underscore-address:{name}")),
        )))
    }
}
