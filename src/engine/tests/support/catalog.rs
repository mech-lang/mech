use mech_core::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction,
    FunctionCatalog, InputPortLayout, InputPortPolicy, OperationContractDeclaration,
    OutputConstruction, OutputPortPolicy, ShapeRule,
};
use std::sync::Arc;

pub(crate) fn pure_test_operation_contract(input_count: usize) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                };
                input_count
            ]
            .into_boxed_slice(),
        ),
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
    }
}

/// Returns a new empty catalog for a bare engine instance.
pub fn empty_function_catalog() -> Arc<FunctionCatalog> {
    Arc::new(FunctionCatalog::empty())
}

#[cfg(all(test, feature = "semantic-compiler"))]
mod tests {
    use super::*;
    #[cfg(feature = "program")]
    use crate::{ExtensionFunctionId, FunctionEnvironment};
    #[test]
    fn empty_catalog_has_no_function_surface() {
        let catalog = empty_function_catalog();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    }

    #[test]
    fn empty_catalog_is_not_cached() {
        let first = empty_function_catalog();
        let second = empty_function_catalog();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[cfg(feature = "program")]
    #[test]
    fn canonical_catalogs_have_independent_name_environments() {
        let first = empty_function_catalog();
        let second = empty_function_catalog();
        let mut first_environment = FunctionEnvironment::from_catalog_defaults(&first).unwrap();
        let second_environment = FunctionEnvironment::from_catalog_defaults(&second).unwrap();
        let extension = ExtensionFunctionId::from_name("host/first-only");
        assert!(!Arc::ptr_eq(&first, &second));
        first_environment
            .bind_extension("host/first-only", "first-only", extension)
            .unwrap();
        assert_eq!(
            first_environment.resolve_name("first-only"),
            Some(crate::FunctionBinding::Extension(extension))
        );
        assert_eq!(second_environment.resolve_name("first-only"), None);
    }
}
