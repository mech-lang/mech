use crate::function::environment::{FunctionBinding, FunctionEnvironment};
use crate::function::extensions::{
    ExtensionFunctionId, FunctionExtensionEntry, FunctionExtensionUnavailable, FunctionExtensions,
};
#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use mech_core::{
    ConversionPlan, ConversionStep, FunctionCatalog, FunctionDefinition,
    FunctionOperationUnavailable, FunctionSpecializerEntry, FunctionTypeDeclaration,
    FunctionTypeOverload, MResult, MechError, OperationId, ResolvedCall, SourceInputKind,
    SourceTypeAuthority, SpecializationInput, SpecializationInvocation, SpecializedFunction,
    TypeConstraintFailure, TypeConstraintOrigin, TypeOverloadCandidate, TypeResolutionError,
    UserFunctionTable, exact_type_equal, hash_str, resolve_type_overloads,
};
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::string::String;

/// A named source function could not be resolved in the current environment.
#[derive(Debug, Clone)]
pub struct MissingFunctionError {
    pub function_id: u64,
    pub function_name: String,
}

impl MissingFunctionError {
    pub fn named(name: impl Into<String>) -> Self {
        let function_name = name.into();
        Self {
            function_id: hash_str(&function_name),
            function_name,
        }
    }
}

impl mech_core::MechErrorKind for MissingFunctionError {
    fn name(&self) -> &str {
        "MissingFunction"
    }

    fn message(&self) -> String {
        format!(
            "Function `{}` not found (id {})",
            self.function_name, self.function_id,
        )
    }
}

pub struct FunctionResolver<'a> {
    catalog: &'a FunctionCatalog,
    environment: &'a FunctionEnvironment,
    extensions: &'a FunctionExtensions,
    user_functions: &'a UserFunctionTable,
}

pub enum ResolvedNamedFunction<'a> {
    User(&'a FunctionDefinition),
    Catalog(&'a FunctionSpecializerEntry),
    Extension(&'a FunctionExtensionEntry),
}

impl<'a> FunctionResolver<'a> {
    pub fn new(
        catalog: &'a FunctionCatalog,
        environment: &'a FunctionEnvironment,
        extensions: &'a FunctionExtensions,
        user_functions: &'a UserFunctionTable,
    ) -> Self {
        Self {
            catalog,
            environment,
            extensions,
            user_functions,
        }
    }

    pub fn resolve_named(&self, name: &str) -> MResult<ResolvedNamedFunction<'a>> {
        if let Some(definition) = self.user_functions.resolve_name(name) {
            return Ok(ResolvedNamedFunction::User(definition));
        }

        match self.environment.resolve_name(name) {
            Some(FunctionBinding::CatalogOperation(operation)) => self
                .catalog
                .specializer(operation)
                .map(ResolvedNamedFunction::Catalog)
                .ok_or_else(|| {
                    MechError::new(
                        FunctionOperationUnavailable {
                            operation,
                            canonical_name: Some(String::from(name)),
                        },
                        None,
                    )
                    .with_compiler_loc()
                }),
            Some(FunctionBinding::Extension(extension)) => self
                .extensions
                .entry(extension)
                .map(ResolvedNamedFunction::Extension)
                .ok_or_else(|| extension_unavailable(extension, Some(name))),
            None => {
                Err(MechError::new(MissingFunctionError::named(name), None).with_compiler_loc())
            }
        }
    }

    pub fn specialize_operation(
        &self,
        operation: OperationId,
        invocation: &SpecializationInvocation,
    ) -> MResult<SpecializedFunction> {
        self.specialize_operation_named(operation, None, invocation)
    }

    /// Answers whether one named catalog operation accepts this invocation by
    /// semantic scheme alone. Physical runtime representations and factories
    /// are deliberately not consulted here.
    #[cfg(all(
        feature = "source",
        feature = "functions",
        feature = "string_concat",
        feature = "math_add"
    ))]
    pub(crate) fn operation_semantically_accepts(
        &self,
        operation: OperationId,
        canonical_name: &str,
        invocation: &SpecializationInvocation,
    ) -> MResult<bool> {
        let Some(entry) = self.catalog.operation_specializer(operation) else {
            return Ok(false);
        };
        self.environment
            .require_operation_enabled(operation, Some(canonical_name))?;
        let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
            return Ok(false);
        };
        match resolve_declared_call(entry, declaration, invocation) {
            Ok(_) => Ok(true),
            Err(error) if error.kind_name() == "TypeIncompatibility" => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn specialize_operation_named(
        &self,
        operation: OperationId,
        canonical_name: Option<&str>,
        invocation: &SpecializationInvocation,
    ) -> MResult<SpecializedFunction> {
        self.specialize_operation_named_with(operation, canonical_name, invocation, |cell, plan| {
            if matches!(plan.step, ConversionStep::Identity) {
                Ok(cell.clone())
            } else {
                Err(MechError::new(
                    mech_core::ConversionExecutionError::ConversionExecutionUnsupported,
                    None,
                )
                .with_compiler_loc())
            }
        })
    }

    pub(crate) fn specialize_operation_named_with(
        &self,
        operation: OperationId,
        canonical_name: Option<&str>,
        invocation: &SpecializationInvocation,
        mut materialize_conversion: impl FnMut(
            &mech_core::ValueCell,
            &ConversionPlan,
        ) -> MResult<mech_core::ValueCell>,
    ) -> MResult<SpecializedFunction> {
        let entry = self
            .catalog
            .operation_specializer(operation)
            .ok_or_else(|| {
                MechError::new(
                    FunctionOperationUnavailable {
                        operation,
                        canonical_name: canonical_name.map(String::from),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;

        self.environment
            .require_operation_enabled(operation, Some(&entry.canonical_name))?;
        match &entry.type_authority {
            SourceTypeAuthority::SyntaxDirectedIntrinsic => {
                let mut context = mech_core::SpecializationContext::for_invocation(
                    invocation,
                    Some(self.catalog),
                )?;
                let function = entry
                    .specializer
                    .specialize_invocation(invocation, &mut context)
                    .map_err(|error| semantic_operation_error(error, &entry.canonical_name))?;
                function.output().resolved_type()?;
                Ok(function.with_semantic_operation(entry.canonical_name.clone()))
            }
            SourceTypeAuthority::Schemes(declaration) => {
                let resolved = resolve_declared_call(entry, declaration, invocation)?;
                let converted = materialize_resolved_inputs(
                    invocation,
                    &resolved,
                    &mut materialize_conversion,
                )?;
                let mut context = mech_core::SpecializationContext::for_resolved_invocation(
                    &converted,
                    Some(self.catalog),
                    entry.canonical_name.clone(),
                    resolved.clone(),
                )?;
                let function = entry
                    .specializer
                    .specialize_invocation(&converted, &mut context)
                    .map_err(|error| semantic_operation_error(error, &entry.canonical_name))?;
                validate_resolved_output(entry, &resolved, &function)?;
                Ok(function.with_semantic_operation(entry.canonical_name.clone()))
            }
        }
    }

    pub fn specialize_named_extension(
        &self,
        extension: ExtensionFunctionId,
        invocation: &SpecializationInvocation,
    ) -> MResult<SpecializedFunction> {
        let entry = self
            .extensions
            .entry(extension)
            .ok_or_else(|| extension_unavailable(extension, None))?;
        let mut context =
            mech_core::SpecializationContext::for_invocation(invocation, Some(self.catalog))?;
        entry
            .specializer
            .specialize_invocation(invocation, &mut context)
            .map(|function| function.with_semantic_operation(entry.canonical_name.clone()))
    }
}

fn semantic_operation_error(mut error: MechError, operation: &str) -> MechError {
    if !error.display_message().contains(operation) {
        error.message = Some(format!(
            "semantic operation `{operation}` failed: {}",
            error.display_message(),
        ));
    }
    error
}

fn resolve_declared_call(
    entry: &FunctionSpecializerEntry,
    declaration: &FunctionTypeDeclaration,
    invocation: &SpecializationInvocation,
) -> MResult<ResolvedCall> {
    let instantiated = declaration
        .template
        .map(|template| {
            let inputs = invocation
                .inputs()
                .iter()
                .map(|input| input.cell()?.resolved_type())
                .collect::<MResult<Vec<_>>>()?;
            let schemes = mech_core::instantiate_source_scheme_template(template, &inputs)?;
            Ok::<_, MechError>(FunctionTypeDeclaration::from_schemes(schemes))
        })
        .transpose()?;
    let declaration = instantiated.as_ref().unwrap_or(declaration);
    let matching = declaration
        .overloads
        .iter()
        .filter(|overload| control_layout_matches(overload, invocation))
        .collect::<Vec<_>>();
    let Some(first) = matching.first() else {
        return Err(MechError::from(TypeResolutionError::incompatible(
            entry.canonical_name.clone(),
            TypeConstraintFailure::Arity {
                expected: declaration
                    .overloads
                    .iter()
                    .map(|overload| overload.input_layout.len().to_string())
                    .collect::<Vec<_>>()
                    .join(" or "),
                actual: invocation.len(),
            },
        )));
    };
    let originals = value_inputs_for_layout(&first.input_layout, invocation)?
        .into_iter()
        .map(|cell| cell.resolved_type())
        .collect::<MResult<Vec<_>>>()?;
    let candidates = matching
        .iter()
        .map(|overload| TypeOverloadCandidate {
            id: u64::from(overload.id),
            scheme: &overload.scheme,
        })
        .collect::<Vec<_>>();
    let resolved = resolve_type_overloads(
        TypeConstraintOrigin::new(entry.canonical_name.clone(), None),
        &candidates,
        &originals,
        None,
    )
    .map_err(MechError::from)?;
    let overload_id = u32::try_from(resolved.candidate_ids[0]).map_err(|_| {
        MechError::from(TypeResolutionError::incompatible(
            entry.canonical_name.clone(),
            TypeConstraintFailure::InvalidScheme {
                reason: "resolved overload ID exceeds the declared u32 domain".into(),
            },
        ))
    })?;
    let output_schema_rules = matching
        .iter()
        .find(|overload| overload.id == overload_id)
        .map(|overload| overload.output_schema_rules.clone())
        .ok_or_else(|| {
            MechError::from(TypeResolutionError::incompatible(
                entry.canonical_name.clone(),
                TypeConstraintFailure::InvalidScheme {
                    reason: "the selected overload has no declared output schema rules".into(),
                },
            ))
        })?;
    let conversions = resolved.conversions.into_vec();
    if conversions.len() != originals.len() {
        return Err(MechError::from(TypeResolutionError::incompatible(
            entry.canonical_name.clone(),
            TypeConstraintFailure::InvalidScheme {
                reason: "the solver did not return exactly one conversion plan per Value input"
                    .into(),
            },
        )));
    }
    let converted_inputs = conversions
        .iter()
        .map(|plan| plan.target.clone())
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok(ResolvedCall {
        operation: entry.operation,
        overload_id,
        original_inputs: originals.into_boxed_slice(),
        converted_inputs,
        input_conversions: conversions.into_boxed_slice(),
        outputs: resolved.outputs,
        output_schema_rules,
    })
}

fn control_layout_matches(
    overload: &FunctionTypeOverload,
    invocation: &SpecializationInvocation,
) -> bool {
    match overload.scheme.inputs() {
        mech_core::InputKindScheme::Fixed(_) => {
            overload.input_layout.len() == invocation.len()
                && overload
                    .input_layout
                    .iter()
                    .zip(invocation.inputs())
                    .all(|(expected, actual)| control_input_matches(*expected, actual))
        }
        mech_core::InputKindScheme::Variadic {
            prefix,
            min_repetitions,
            ..
        } => {
            let repeated_layout = overload.input_layout.get(prefix.len()).copied();
            let minimum = prefix.len().saturating_add(*min_repetitions as usize);
            invocation.len() >= minimum
                && overload.input_layout.len() == prefix.len() + 1
                && overload.input_layout[..prefix.len()]
                    .iter()
                    .zip(&invocation.inputs()[..prefix.len()])
                    .all(|(expected, actual)| control_input_matches(*expected, actual))
                && repeated_layout.is_some_and(|expected| {
                    invocation.inputs()[prefix.len()..]
                        .iter()
                        .all(|actual| control_input_matches(expected, actual))
                })
        }
    }
}

fn control_input_matches(expected: SourceInputKind, actual: &SpecializationInput) -> bool {
    matches!(
        (expected, actual),
        (SourceInputKind::Value, SpecializationInput::Cell(_))
            | (SourceInputKind::Absent, SpecializationInput::Absent)
            | (
                SourceInputKind::MatrixAllSelection,
                SpecializationInput::MatrixAllSelection
            )
    )
}

fn value_inputs_for_layout<'a>(
    layout: &[SourceInputKind],
    invocation: &'a SpecializationInvocation,
) -> MResult<Vec<&'a mech_core::ValueCell>> {
    if layout.len() == 1
        && layout[0] == SourceInputKind::Value
        && invocation
            .inputs()
            .iter()
            .all(|input| matches!(input, SpecializationInput::Cell(_)))
    {
        return invocation
            .inputs()
            .iter()
            .map(SpecializationInput::cell)
            .collect();
    }
    layout
        .iter()
        .zip(invocation.inputs())
        .filter_map(|(expected, input)| {
            (*expected == SourceInputKind::Value).then_some(input.cell())
        })
        .collect()
}

fn materialize_resolved_inputs(
    invocation: &SpecializationInvocation,
    resolved: &ResolvedCall,
    materialize: &mut impl FnMut(
        &mech_core::ValueCell,
        &ConversionPlan,
    ) -> MResult<mech_core::ValueCell>,
) -> MResult<SpecializationInvocation> {
    let mut value_index = 0;
    let inputs = invocation
        .inputs()
        .iter()
        .map(|input| match input {
            SpecializationInput::Cell(cell) => {
                let plan = &resolved.input_conversions[value_index];
                value_index += 1;
                let converted = materialize(cell, plan)?;
                let actual = converted.resolved_type()?;
                if !exact_type_equal(&plan.target, &actual) {
                    return Err(MechError::from(TypeResolutionError::incompatible(
                        "input conversion",
                        TypeConstraintFailure::OutputTypeMismatch {
                            expected: plan.target.semantic_name(),
                            actual: actual.semantic_name(),
                        },
                    )));
                }
                Ok(SpecializationInput::Cell(converted))
            }
            control => Ok(control.clone()),
        })
        .collect::<MResult<Vec<_>>>()?;
    Ok(SpecializationInvocation::new(inputs.into_boxed_slice()))
}

fn validate_resolved_output(
    entry: &FunctionSpecializerEntry,
    resolved: &ResolvedCall,
    function: &SpecializedFunction,
) -> MResult<()> {
    let actual = function.output().resolved_type()?;
    let Some(expected) = resolved.outputs.first() else {
        return Err(MechError::from(TypeResolutionError::incompatible(
            entry.canonical_name.clone(),
            TypeConstraintFailure::InvalidScheme {
                reason: "the selected overload declares no output".into(),
            },
        )));
    };
    if exact_type_equal(expected, &actual) {
        return Ok(());
    }
    Err(MechError::from(TypeResolutionError::incompatible(
        entry.canonical_name.clone(),
        TypeConstraintFailure::OutputTypeMismatch {
            expected: expected.semantic_name(),
            actual: actual.semantic_name(),
        },
    )))
}

fn extension_unavailable(id: ExtensionFunctionId, canonical_name: Option<&str>) -> MechError {
    MechError::new(
        FunctionExtensionUnavailable {
            id,
            canonical_name: canonical_name.map(String::from),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "semantic-compiler")]
    use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
    use mech_core::{
        CanonicalFunctionSpecializer, FunctionCatalogBuilder, FunctionDefine, FunctionExport,
        FunctionExposure, FunctionInstance, FunctionInvocation, MechFunctionImpl,
        SpecializationContext, SpecializationInvocation, SpecializedFunction, ValueCell,
        internal_pattern_value_identifier,
    };
    use std::sync::Arc;

    struct TestFunction(&'static str);

    impl MechFunctionImpl for TestFunction {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn reactive_output_value_cells(&self) -> Vec<mech_core::ValueCell> {
            vec![mech_core::ValueCell::unit()]
        }

        fn to_string(&self) -> String {
            String::from(self.0)
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for TestFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct TestSpecializer(&'static str);

    impl CanonicalFunctionSpecializer for TestSpecializer {
        fn specialize_invocation(
            &self,
            invocation: &SpecializationInvocation,
            _: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            let inputs = invocation
                .inputs()
                .iter()
                .filter_map(|input| match input {
                    SpecializationInput::Cell(cell) => Some(cell.clone()),
                    SpecializationInput::Absent | SpecializationInput::MatrixAllSelection => None,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let output = inputs.first().cloned().unwrap_or_else(ValueCell::unit);
            let invocation = FunctionInvocation::variadic(output, inputs);
            Ok(SpecializedFunction::new(FunctionInstance::new(
                Box::new(TestFunction(self.0)),
                invocation,
            )))
        }
    }

    fn test_catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = builder
            .insert_canonical_specializer(
                "math/add",
                mech_core::maintained_source_type_declaration("math/add").unwrap(),
                Arc::new(TestSpecializer("catalog")),
            )
            .unwrap();
        builder
            .insert_export(FunctionExport {
                operation,
                canonical_name: String::from("math/add"),
                module: None,
                item: None,
                exposure: FunctionExposure::Prelude,
            })
            .unwrap();
        builder.build().unwrap()
    }

    fn empty_user_function(name: &str) -> FunctionDefinition {
        FunctionDefinition::new(
            hash_str(name),
            String::from(name),
            FunctionDefine {
                name: internal_pattern_value_identifier(name),
                input: Vec::new(),
                output: Vec::new(),
                statements: Vec::new(),
                match_arms: Vec::new(),
            },
        )
    }

    #[test]
    fn user_definition_wins_over_the_current_environment_binding() {
        let catalog = test_catalog();
        let environment = FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
        let extensions = FunctionExtensions::default();
        let mut user_functions = UserFunctionTable::default();
        let user = empty_user_function("math/add");
        user_functions.insert_or_replace(user).unwrap();
        let resolver = FunctionResolver::new(&catalog, &environment, &extensions, &user_functions);

        match resolver.resolve_named("math/add").unwrap() {
            ResolvedNamedFunction::User(definition) => {
                assert_eq!(definition.name, "math/add");
            }
            _ => panic!("user definition must have highest named-call precedence"),
        }
    }

    #[test]
    fn named_binding_can_select_catalog_or_extension_entries() {
        let catalog = test_catalog();
        let mut environment = FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
        let mut extensions = FunctionExtensions::default();
        let extension_name = "host/add";
        let extension = ExtensionFunctionId::from_name(extension_name);
        extensions
            .insert_or_replace(FunctionExtensionEntry::new(
                extension_name,
                Arc::new(TestSpecializer("extension")),
            ))
            .unwrap();
        environment
            .bind_extension(extension_name, "host-add", extension)
            .unwrap();
        let users = UserFunctionTable::default();
        let resolver = FunctionResolver::new(&catalog, &environment, &extensions, &users);

        assert!(matches!(
            resolver.resolve_named("math/add").unwrap(),
            ResolvedNamedFunction::Catalog(_),
        ));
        assert!(matches!(
            resolver.resolve_named("host-add").unwrap(),
            ResolvedNamedFunction::Extension(_),
        ));
    }

    #[test]
    fn syntax_operation_ignores_same_named_user_and_extension_bindings() {
        let catalog = test_catalog();
        let mut environment = FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
        let mut extensions = FunctionExtensions::default();
        let extension_name = "host/replacement-add";
        let extension = ExtensionFunctionId::from_name(extension_name);
        extensions
            .insert_or_replace(FunctionExtensionEntry::new(
                extension_name,
                Arc::new(TestSpecializer("extension")),
            ))
            .unwrap();
        environment
            .bind_extension(extension_name, "math/add", extension)
            .unwrap();
        let mut users = UserFunctionTable::default();
        users
            .insert_or_replace(empty_user_function("math/add"))
            .unwrap();
        let resolver = FunctionResolver::new(&catalog, &environment, &extensions, &users);

        assert!(matches!(
            resolver.resolve_named("math/add").unwrap(),
            ResolvedNamedFunction::User(_),
        ));
        assert_eq!(
            resolver
                .specialize_operation(
                    OperationId::from_name("math/add"),
                    &SpecializationInvocation::from_cells(
                        vec![
                            ValueCell::from_exact(1.0_f64).unwrap(),
                            ValueCell::from_exact(2.0_f64).unwrap(),
                        ]
                        .into_boxed_slice(),
                    ),
                )
                .unwrap()
                .instance()
                .implementation()
                .to_string(),
            "catalog",
        );
    }

    #[test]
    fn disabled_catalog_operations_fail_before_specialization() {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = builder
            .insert_canonical_specializer(
                "stats/mean",
                mech_core::FunctionTypeDeclaration::from_schemes(vec![
                    mech_core::exact_unary(mech_core::KindExpr::Index, mech_core::KindExpr::Index)
                        .unwrap(),
                ]),
                Arc::new(TestSpecializer("catalog")),
            )
            .unwrap();
        builder
            .insert_export(FunctionExport {
                operation,
                canonical_name: String::from("stats/mean"),
                module: Some(String::from("stats")),
                item: Some(String::from("mean")),
                exposure: FunctionExposure::ModuleOnly,
            })
            .unwrap();
        let catalog = builder.build().unwrap();
        let environment = FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
        let extensions = FunctionExtensions::default();
        let users = UserFunctionTable::default();
        let resolver = FunctionResolver::new(&catalog, &environment, &extensions, &users);

        let error = match resolver
            .specialize_operation(operation, &SpecializationInvocation::new(Box::new([])))
        {
            Ok(_) => panic!("disabled catalog operation unexpectedly specialized"),
            Err(error) => error,
        };
        assert_eq!(error.kind_name(), "FunctionOperationNotVisible");
    }

    #[test]
    fn missing_named_functions_do_not_fall_through_inside_the_final_resolver() {
        let catalog = test_catalog();
        let environment = FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
        let extensions = FunctionExtensions::default();
        let users = UserFunctionTable::default();
        let resolver = FunctionResolver::new(&catalog, &environment, &extensions, &users);

        let error = match resolver.resolve_named("missing") {
            Ok(_) => panic!("missing function unexpectedly resolved"),
            Err(error) => error,
        };
        assert_eq!(error.kind_name(), "MissingFunction");
        assert!(
            error
                .display_message()
                .contains("Function `missing` not found"),
            "{}",
            error.display_message(),
        );
    }
}
