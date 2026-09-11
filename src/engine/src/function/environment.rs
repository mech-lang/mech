use crate::function::extensions::ExtensionFunctionId;
#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
};
use mech_core::{
    FunctionCatalog, FunctionExport, FunctionExposure, FunctionOperationNotVisible, MResult,
    MechError, MechErrorKind, OperationId, hash_str,
};
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::string::String;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionBinding {
    CatalogOperation(OperationId),
    Extension(ExtensionFunctionId),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FunctionEnvironment {
    enabled_operations: BTreeSet<OperationId>,
    bindings: BTreeMap<u64, FunctionBinding>,
    dictionary: BTreeMap<u64, String>,
}

impl FunctionEnvironment {
    pub fn from_catalog_defaults(catalog: &FunctionCatalog) -> MResult<Self> {
        let mut environment = Self::default();

        for entry in catalog.intrinsic_specializer_entries() {
            environment.enabled_operations.insert(entry.operation.id);
        }

        for export in catalog.all_exports() {
            match export.exposure {
                FunctionExposure::Internal => {
                    environment.enabled_operations.insert(export.operation);
                }
                FunctionExposure::Prelude => {
                    environment.bind_catalog_export(export, &export.canonical_name)?;
                }
                FunctionExposure::ModuleOnly => {}
            }
        }

        Ok(environment)
    }

    pub fn operation_is_enabled(&self, operation: OperationId) -> bool {
        self.enabled_operations.contains(&operation)
    }

    pub fn require_operation_enabled(
        &self,
        operation: OperationId,
        canonical_name: Option<&str>,
    ) -> MResult<()> {
        if self.operation_is_enabled(operation) {
            Ok(())
        } else {
            Err(MechError::new(
                FunctionOperationNotVisible {
                    operation,
                    canonical_name: canonical_name.map(String::from),
                },
                None,
            )
            .with_compiler_loc())
        }
    }

    pub fn resolve_name(&self, name: &str) -> Option<FunctionBinding> {
        let name_id = hash_str(name);
        match (self.dictionary.get(&name_id), self.bindings.get(&name_id)) {
            (Some(stored_name), Some(binding)) if stored_name == name => Some(*binding),
            _ => None,
        }
    }

    pub fn bind_catalog_export(
        &mut self,
        export: &FunctionExport,
        visible_name: &str,
    ) -> MResult<()> {
        validate_catalog_binding(export, visible_name)?;
        if export.exposure == FunctionExposure::Internal {
            return Err(invalid_binding(
                &export.canonical_name,
                visible_name,
                "internal operations cannot install source-callable names",
            ));
        }

        self.bind_name(
            visible_name,
            FunctionBinding::CatalogOperation(export.operation),
        )?;
        self.enabled_operations.insert(export.operation);
        Ok(())
    }

    pub fn bind_extension(
        &mut self,
        canonical_name: &str,
        visible_name: &str,
        extension: ExtensionFunctionId,
    ) -> MResult<()> {
        if canonical_name.is_empty() {
            return Err(invalid_binding(
                canonical_name,
                visible_name,
                "canonical name must not be empty",
            ));
        }
        if visible_name.is_empty() {
            return Err(invalid_binding(
                canonical_name,
                visible_name,
                "visible name must not be empty",
            ));
        }

        let expected = ExtensionFunctionId::from_name(canonical_name);
        if expected != extension {
            return Err(invalid_binding(
                canonical_name,
                visible_name,
                format!(
                    "canonical name hashes to 0x{:016x}, not extension ID 0x{:016x}",
                    expected.raw(),
                    extension.raw(),
                ),
            ));
        }

        self.bind_name(visible_name, FunctionBinding::Extension(extension))
    }

    fn bind_name(&mut self, visible_name: &str, binding: FunctionBinding) -> MResult<()> {
        let name_id = hash_str(visible_name);
        if let Some(existing_name) = self.dictionary.get(&name_id)
            && existing_name != visible_name
        {
            return Err(MechError::new(
                FunctionEnvironmentNameCollision {
                    id: name_id,
                    existing_name: existing_name.clone(),
                    incoming_name: String::from(visible_name),
                },
                None,
            )
            .with_compiler_loc());
        }

        // Exact-name replacement is deliberate: the latest explicit import or
        // program-local registration controls named non-user dispatch.
        self.bindings.insert(name_id, binding);
        self.dictionary.insert(name_id, String::from(visible_name));
        Ok(())
    }
}

fn validate_catalog_binding(export: &FunctionExport, visible_name: &str) -> MResult<()> {
    if export.canonical_name.is_empty() {
        return Err(invalid_binding(
            &export.canonical_name,
            visible_name,
            "canonical name must not be empty",
        ));
    }
    if visible_name.is_empty() {
        return Err(invalid_binding(
            &export.canonical_name,
            visible_name,
            "visible name must not be empty",
        ));
    }

    let expected = OperationId::from_name(&export.canonical_name);
    if expected != export.operation {
        return Err(invalid_binding(
            &export.canonical_name,
            visible_name,
            format!(
                "canonical name hashes to 0x{:016x}, not operation ID 0x{:016x}",
                expected.raw(),
                export.operation.raw(),
            ),
        ));
    }

    Ok(())
}

fn invalid_binding(
    canonical_name: &str,
    visible_name: &str,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        FunctionEnvironmentInvalidBinding {
            canonical_name: String::from(canonical_name),
            visible_name: String::from(visible_name),
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionEnvironmentInvalidBinding {
    pub canonical_name: String,
    pub visible_name: String,
    pub reason: String,
}

impl MechErrorKind for FunctionEnvironmentInvalidBinding {
    fn name(&self) -> &str {
        "FunctionEnvironmentInvalidBinding"
    }

    fn message(&self) -> String {
        format!(
            "cannot bind function {:?} as {:?}: {}",
            self.canonical_name, self.visible_name, self.reason,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionEnvironmentNameCollision {
    pub id: u64,
    pub existing_name: String,
    pub incoming_name: String,
}

impl MechErrorKind for FunctionEnvironmentNameCollision {
    fn name(&self) -> &str {
        "FunctionEnvironmentNameCollision"
    }

    fn message(&self) -> String {
        format!(
            "visible function names {:?} and {:?} collide at ID 0x{:016x}",
            self.existing_name, self.incoming_name, self.id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        AccessMode, AliasPolicy, CanonicalFunctionSpecializer, ChangeDetectionPolicy, DeliveryMode,
        ExternalInteraction, FunctionCatalogBuilder, InputPortLayout, InputPortPolicy,
        OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ShapeRule,
        SpecializationContext, SpecializationInvocation, SpecializedFunction,
    };
    use std::sync::Arc;

    struct TestSpecializer;

    impl CanonicalFunctionSpecializer for TestSpecializer {
        fn specialize_invocation(
            &self,
            _: &SpecializationInvocation,
            _: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            unreachable!("environment tests do not specialize functions")
        }
    }

    fn export(
        canonical_name: &str,
        module: Option<&str>,
        item: Option<&str>,
        exposure: FunctionExposure,
    ) -> FunctionExport {
        FunctionExport {
            operation: OperationId::from_name(canonical_name),
            canonical_name: String::from(canonical_name),
            module: module.map(String::from),
            item: item.map(String::from),
            exposure,
        }
    }

    fn catalog_with_all_exposures() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        let syntax_contract = OperationContractDeclaration {
            inputs: InputPortLayout::Variadic {
                prefix: Box::new([]),
                repeated: InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                min_repetitions: 0,
            },
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        };
        builder
            .insert_canonical_intrinsic_specializer(
                "assign",
                syntax_contract.clone(),
                Arc::new(TestSpecializer),
            )
            .unwrap();
        for name in ["syntax/internal", "math/add", "stats/mean"] {
            let declaration =
                mech_core::maintained_source_type_declaration(name).unwrap_or_else(|_| {
                    mech_core::FunctionTypeDeclaration::from_schemes(vec![
                        mech_core::exact_unary(
                            mech_core::KindExpr::Index,
                            mech_core::KindExpr::Index,
                        )
                        .unwrap(),
                    ])
                });
            builder
                .insert_canonical_specializer_with_contract(
                    name,
                    declaration,
                    syntax_contract.clone(),
                    Arc::new(TestSpecializer),
                )
                .unwrap();
        }
        builder
            .insert_export(export(
                "syntax/internal",
                None,
                None,
                FunctionExposure::Internal,
            ))
            .unwrap();
        builder
            .insert_export(export("math/add", None, None, FunctionExposure::Prelude))
            .unwrap();
        builder
            .insert_export(export(
                "math/add",
                Some("math"),
                Some("add"),
                FunctionExposure::ModuleOnly,
            ))
            .unwrap();
        builder
            .insert_export(export(
                "stats/mean",
                Some("stats"),
                Some("mean"),
                FunctionExposure::ModuleOnly,
            ))
            .unwrap();
        builder.build().unwrap()
    }

    #[test]
    fn catalog_defaults_apply_each_exposure_exactly() {
        let catalog = catalog_with_all_exposures();
        let environment = FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
        let internal = OperationId::from_name("syntax/internal");
        let prelude = OperationId::from_name("math/add");
        let module_only = OperationId::from_name("stats/mean");
        let intrinsic = OperationId::from_name("assign");

        assert!(environment.operation_is_enabled(intrinsic));
        assert_eq!(environment.resolve_name("assign"), None);
        assert!(environment.operation_is_enabled(internal));
        assert_eq!(environment.resolve_name("syntax/internal"), None);
        assert!(environment.operation_is_enabled(prelude));
        assert_eq!(
            environment.resolve_name("math/add"),
            Some(FunctionBinding::CatalogOperation(prelude)),
        );
        assert!(!environment.operation_is_enabled(module_only));
        assert_eq!(environment.resolve_name("stats/mean"), None);
    }

    #[test]
    fn module_import_enables_and_binds_its_operation() {
        let catalog = catalog_with_all_exposures();
        let mut environment = FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();
        let operation = OperationId::from_name("stats/mean");

        environment
            .bind_catalog_export(catalog.module_export("stats", "mean").unwrap(), "average")
            .unwrap();

        assert!(environment.operation_is_enabled(operation));
        assert_eq!(
            environment.resolve_name("average"),
            Some(FunctionBinding::CatalogOperation(operation)),
        );
    }

    #[test]
    fn latest_exact_name_binding_controls_non_user_dispatch() {
        let catalog = catalog_with_all_exposures();
        let export = catalog.module_export("stats", "mean").unwrap();
        let operation = export.operation;
        let extension = ExtensionFunctionId::from_name("host/average");
        let mut environment = FunctionEnvironment::from_catalog_defaults(&catalog).unwrap();

        environment
            .bind_extension("host/average", "average", extension)
            .unwrap();
        assert_eq!(
            environment.resolve_name("average"),
            Some(FunctionBinding::Extension(extension)),
        );

        environment.bind_catalog_export(export, "average").unwrap();
        assert_eq!(
            environment.resolve_name("average"),
            Some(FunctionBinding::CatalogOperation(operation)),
        );

        environment
            .bind_extension("host/average", "average", extension)
            .unwrap();
        assert_eq!(
            environment.resolve_name("average"),
            Some(FunctionBinding::Extension(extension)),
        );
    }

    #[test]
    fn name_collisions_fail_before_mutating_the_binding() {
        let requested = "average";
        let id = hash_str(requested);
        let old_operation = OperationId::from_name("math/add");
        let mut environment = FunctionEnvironment::default();
        environment
            .bindings
            .insert(id, FunctionBinding::CatalogOperation(old_operation));
        environment
            .dictionary
            .insert(id, String::from("a different colliding name"));

        let error = environment
            .bind_extension(
                "host/average",
                requested,
                ExtensionFunctionId::from_name("host/average"),
            )
            .unwrap_err();

        assert_eq!(error.kind_name(), "FunctionEnvironmentNameCollision");
        assert_eq!(
            environment.bindings.get(&id),
            Some(&FunctionBinding::CatalogOperation(old_operation)),
        );
    }

    #[test]
    fn internal_exports_cannot_be_bound_as_source_names() {
        let catalog = catalog_with_all_exposures();
        let internal =
            catalog.exports_for_operation(OperationId::from_name("syntax/internal"))[0].clone();
        let mut environment = FunctionEnvironment::default();

        let error = environment
            .bind_catalog_export(&internal, "internal")
            .unwrap_err();

        assert_eq!(error.kind_name(), "FunctionEnvironmentInvalidBinding");
        assert_eq!(environment.resolve_name("internal"), None);
    }
}
