use crate::function::environment::{FunctionBinding, FunctionEnvironment};
use crate::function::extensions::{
    ExtensionFunctionId, FunctionExtensionEntry, FunctionExtensionUnavailable, FunctionExtensions,
};
use mech_core::{
    FunctionCatalog, FunctionDefinition, FunctionOperationUnavailable, FunctionSpecializerEntry,
    LegacyValue, MResult, MechError, MechFunction, OperationId, UserFunctionTable, hash_str,
};

/// A named source function could not be resolved in the current environment.
#[derive(Debug, Clone)]
pub struct MissingFunctionError {
    pub function_id: u64,
}

impl mech_core::MechErrorKind for MissingFunctionError {
    fn name(&self) -> &str {
        "MissingFunction"
    }

    fn message(&self) -> String {
        format!("Function with id {} not found", self.function_id)
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
        let name_id = hash_str(name);
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
            None => Err(MechError::new(
                MissingFunctionError {
                    function_id: name_id,
                },
                None,
            )
            .with_compiler_loc()),
        }
    }

    pub fn specialize_operation(
        &self,
        operation: OperationId,
        arguments: &[LegacyValue],
    ) -> MResult<Box<dyn MechFunction>> {
        self.specialize_operation_named(operation, None, arguments)
    }

    pub(crate) fn specialize_operation_named(
        &self,
        operation: OperationId,
        canonical_name: Option<&str>,
        arguments: &[LegacyValue],
    ) -> MResult<Box<dyn MechFunction>> {
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
        entry.specializer.specialize(arguments)
    }

    pub fn specialize_named_extension(
        &self,
        extension: ExtensionFunctionId,
        arguments: &[LegacyValue],
    ) -> MResult<Box<dyn MechFunction>> {
        let entry = self
            .extensions
            .entry(extension)
            .ok_or_else(|| extension_unavailable(extension, None))?;
        entry.specializer.specialize(arguments)
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
        FunctionCatalogBuilder, FunctionDefine, FunctionExport, FunctionExposure,
        FunctionSpecializer, MechFunctionImpl, internal_pattern_value_identifier,
    };
    use std::sync::Arc;

    struct TestFunction(&'static str);

    impl MechFunctionImpl for TestFunction {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            LegacyValue::Empty
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Ok(Vec::new())
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

    impl FunctionSpecializer for TestSpecializer {
        fn specialize(&self, _: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
            Ok(Box::new(TestFunction(self.0)))
        }
    }

    fn test_catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = builder
            .insert_specializer("math/add", Arc::new(TestSpecializer("catalog")))
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
                .specialize_operation(OperationId::from_name("math/add"), &[])
                .unwrap()
                .to_string(),
            "catalog",
        );
    }

    #[test]
    fn disabled_catalog_operations_fail_before_specialization() {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = builder
            .insert_specializer("stats/mean", Arc::new(TestSpecializer("catalog")))
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

        let error = match resolver.specialize_operation(operation, &[]) {
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
    }
}
