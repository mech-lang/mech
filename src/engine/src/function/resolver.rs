use crate::function::environment::{FunctionBinding, FunctionEnvironment};
use crate::function::extensions::{
    ExtensionFunctionId, FunctionExtensionEntry, FunctionExtensionUnavailable, FunctionExtensions,
};
#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::string::String;
use mech_core::{
    FunctionCatalog, FunctionDefinition, FunctionOperationUnavailable, FunctionSpecializerEntry,
    MResult, MechError, OperationId, SpecializationInvocation, SpecializedFunction,
    UserFunctionTable, hash_str,
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

    pub(crate) fn specialize_operation_named(
        &self,
        operation: OperationId,
        canonical_name: Option<&str>,
        invocation: &SpecializationInvocation,
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
        let mut context =
            mech_core::SpecializationContext::for_invocation(invocation, Some(self.catalog))?;
        entry
            .specializer
            .specialize_invocation(invocation, &mut context)
            .map(|function| function.with_semantic_operation(entry.canonical_name.clone()))
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
            _: &SpecializationInvocation,
            _: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            let invocation = FunctionInvocation::nullary(ValueCell::unit());
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
                    &SpecializationInvocation::new(Box::new([])),
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
