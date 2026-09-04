use super::*;
use crate::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction,
    FunctionValueRepresentation, InputPortLayout, InputPortPolicy, KindExpr, MechFunctionImpl,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
    ResolvedOperationDescriptor, SchemaBody, ShapeRule, SpecializationContext,
    SpecializationInvocation, SpecializedFunction, ValueCell,
};
use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;

static INDEX_COPY_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

static WRONG_ARITY_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(Box::new([])),
        outputs: INDEX_COPY_CONTRACT.outputs.clone(),
        interaction: ExternalInteraction::Pure,
    });

struct CatalogTestFunction;

impl MechFunctionImpl for CatalogTestFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn to_string(&self) -> String {
        "CatalogTestFunction".into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl crate::MechFunctionCompiler for CatalogTestFunction {
    fn compile(
        &self,
        _context: &mut dyn crate::BytecodeCompilerContext,
    ) -> MResult<crate::Register> {
        Ok(0)
    }
}

static FACTORY_CALLS: AtomicUsize = AtomicUsize::new(0);

struct IndexUnaryFactory;

impl MechFunctionFactory for IndexUnaryFactory {
    fn implementation_memory_class() -> crate::ImplementationMemoryClass {
        crate::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::Index,
        FunctionValueRepresentation::Index,
    );

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&INDEX_COPY_CONTRACT)
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        FACTORY_CALLS.fetch_add(1, Ordering::SeqCst);
        let (output, input) = invocation.expect_unary()?;
        let _: crate::Ref<usize> = output.try_ref()?;
        let _: crate::Ref<usize> = input.try_ref()?;
        Ok(Box::new(CatalogTestFunction))
    }
}

struct WrongContractFactory;

impl MechFunctionFactory for WrongContractFactory {
    fn implementation_memory_class() -> crate::ImplementationMemoryClass {
        crate::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = IndexUnaryFactory::SIGNATURE;

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&WRONG_ARITY_CONTRACT)
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        FACTORY_CALLS.fetch_add(1, Ordering::SeqCst);
        invocation.expect_unary()?;
        Ok(Box::new(CatalogTestFunction))
    }
}

struct DuplicateIndexUnaryFactory;

impl MechFunctionFactory for DuplicateIndexUnaryFactory {
    fn implementation_memory_class() -> crate::ImplementationMemoryClass {
        crate::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = IndexUnaryFactory::SIGNATURE;

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&INDEX_COPY_CONTRACT)
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_unary()?;
        Ok(Box::new(CatalogTestFunction))
    }
}

struct AnyUnaryFactory;

impl MechFunctionFactory for AnyUnaryFactory {
    fn implementation_memory_class() -> crate::ImplementationMemoryClass {
        crate::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_unary()?;
        Ok(Box::new(CatalogTestFunction))
    }
}

struct TestSpecializer;

impl CanonicalFunctionSpecializer for TestSpecializer {
    fn specialize_invocation(
        &self,
        _invocation: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        unreachable!("catalog indexing tests do not invoke source specialization")
    }
}

fn specializer() -> Arc<dyn CanonicalFunctionSpecializer> {
    Arc::new(TestSpecializer)
}

fn type_declaration() -> FunctionTypeDeclaration {
    FunctionTypeDeclaration::from_schemes(vec![
        KindScheme::new(
            Box::new([]),
            Box::new([]),
            InputKindScheme::Fixed(Box::new([])),
            vec![KindExpr::Index].into_boxed_slice(),
            Box::new([]),
        )
        .unwrap(),
    ])
}

fn contract(alias: RuntimeOutputAliasPolicy) -> RuntimeFunctionContract {
    RuntimeFunctionContract::no_matrix(alias)
}

fn export(name: &str, module: &str, item: &str) -> FunctionExport {
    FunctionExport {
        operation: OperationId::from_name(name),
        canonical_name: name.into(),
        module: Some(module.into()),
        item: Some(item.into()),
        exposure: FunctionExposure::ModuleOnly,
    }
}

fn runtime_entry(id: RuntimeFunctionId, name: &str) -> RuntimeFunctionEntry {
    RuntimeFunctionEntry {
        id,
        name: name.into(),
        invocation_factory: IndexUnaryFactory::new_invocation,
        signature: IndexUnaryFactory::SIGNATURE,
        contract: contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
        operation_contracts: BTreeMap::new(),
        operation_binding: RuntimeOperationBinding::CompilerResolved(RuntimeFamilyId::from_name(
            name,
        )),
        execution_targets: ExecutionTargetSet::DIRECT_RUNTIME,
        implementation_memory: ImplementationMemoryClass::NoAdditionalScratch,
        #[cfg(feature = "native-plan")]
        native_linkage: None,
    }
}

#[test]
fn stable_operation_and_runtime_ids_are_preserved() {
    assert_eq!(
        OperationId::from_name("math/add").raw(),
        0x00cc_5290_41cb_60c3
    );
    assert_eq!(
        RuntimeFunctionId::from_name("AddSS<f64>").raw(),
        0x000a_2c77_6884_86f3
    );
}

#[test]
fn runtime_ids_reject_collisions_and_duplicate_registrations() {
    let mut builder = FunctionCatalogBuilder::new();
    let id = RuntimeFunctionId::from_raw(42);
    builder
        .insert_runtime_entry(runtime_entry(id, "first"))
        .unwrap();

    let collision = builder
        .insert_runtime_entry(runtime_entry(id, "second"))
        .unwrap_err();
    assert_eq!(collision.kind_name(), "FunctionCatalogRuntimeIdCollision");

    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory::<IndexUnaryFactory>(
            "IndexUnary",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            RuntimeFamilyId::from_name("IndexUnary"),
        )
        .unwrap();
    let duplicate = builder
        .insert_runtime_factory::<IndexUnaryFactory>(
            "IndexUnary",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            RuntimeFamilyId::from_name("IndexUnary"),
        )
        .unwrap_err();
    assert_eq!(
        duplicate.kind_name(),
        "FunctionCatalogDuplicateRuntimeFactory"
    );
}

#[test]
fn runtime_entries_preserve_the_factory_memory_class() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory::<IndexUnaryFactory>(
            "IndexUnaryMemoryClass",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            RuntimeFamilyId::from_name("IndexUnaryMemoryClass"),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name("IndexUnaryMemoryClass"))
        .unwrap();
    assert_eq!(
        entry.implementation_memory_class(),
        ImplementationMemoryClass::NoAdditionalScratch
    );
}

#[test]
fn duplicate_exact_operation_target_signatures_are_rejected() {
    let operation = OperationId::from_name("test/index-copy");
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory_with_semantic_contract::<IndexUnaryFactory>(
            "IndexUnaryPrimary",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            operation,
            &INDEX_COPY_CONTRACT,
        )
        .unwrap();
    let duplicate = builder
        .insert_runtime_factory_with_semantic_contract::<DuplicateIndexUnaryFactory>(
            "IndexUnaryDuplicate",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            operation,
            &INDEX_COPY_CONTRACT,
        )
        .unwrap_err();

    assert_eq!(
        duplicate.kind_name(),
        "FunctionCatalogDuplicateRuntimeCapability"
    );
}

#[test]
fn generated_runtime_capability_matrix_does_not_infer_backend_support() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory::<IndexUnaryFactory>(
            "IndexUnaryCapability",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            RuntimeFamilyId::from_name("IndexUnaryCapability"),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let capabilities = catalog.runtime_execution_capabilities().collect::<Vec<_>>();
    let [capability] = capabilities.as_slice() else {
        panic!("one registered runtime factory must produce one capability row")
    };
    assert_eq!(
        capability.runtime_factory,
        RuntimeFunctionId::from_name("IndexUnaryCapability")
    );
    assert_eq!(capability.signature, IndexUnaryFactory::SIGNATURE);
    assert_eq!(
        capability.operation_binding,
        RuntimeOperationBinding::CompilerResolved(RuntimeFamilyId::from_name(
            "IndexUnaryCapability"
        ))
    );
    assert_eq!(
        capability.targets.iter().collect::<Vec<_>>(),
        vec![ExecutionTarget::DirectRuntime]
    );
}

#[test]
fn fixed_operation_registration_requires_a_preconstruction_contract() {
    let mut builder = FunctionCatalogBuilder::new();
    let error = builder
        .insert_runtime_factory_for_operations::<AnyUnaryFactory>(
            "UncontractedFixedRuntime",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            [OperationId::from_name("test/uncontracted")],
        )
        .unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeOperationBindingMismatch");
}

#[test]
fn operation_memory_contract_is_checked_before_factory_construction() {
    let operation = OperationId::from_name("test/wrong-contract");
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory_for_operations::<WrongContractFactory>(
            "WrongContractRuntime",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            [operation],
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name("WrongContractRuntime"))
        .unwrap();
    let before = FACTORY_CALLS.load(Ordering::SeqCst);
    let error = entry
        .bind_resolved_invocation(
            operation,
            ExecutionTarget::DirectRuntime,
            FunctionInvocation::unary(
                ValueCell::from_exact(1usize).unwrap(),
                ValueCell::from_exact(2usize).unwrap(),
            ),
        )
        .err()
        .expect("the wrong operation-memory contract must fail");
    assert_eq!(error.kind_name(), "RuntimeFunctionContractViolation");
    assert_eq!(FACTORY_CALLS.load(Ordering::SeqCst), before);
}

#[test]
fn operation_ids_reject_colliding_and_duplicate_specializers() {
    let operation = OperationId::from_raw(77);
    let mut builder = FunctionCatalogBuilder::new();
    let invalid = builder
        .insert_specializer_entry(FunctionSpecializerEntry {
            operation: ResolvedOperationDescriptor {
                id: operation,
                canonical_name: "first/op".into(),
                contract: INDEX_COPY_CONTRACT.clone(),
            },
            operation_contracts: vec![INDEX_COPY_CONTRACT.clone()].into_boxed_slice(),
            type_authority: SourceTypeAuthority::Schemes(type_declaration()),
            specializer: specializer(),
        })
        .unwrap_err();
    assert_eq!(invalid.kind_name(), "RuntimeOperationBindingMismatch");

    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_canonical_specializer_with_contract(
            "core/test",
            type_declaration(),
            INDEX_COPY_CONTRACT.clone(),
            specializer(),
        )
        .unwrap();
    let duplicate = builder
        .insert_canonical_specializer_with_contract(
            "core/test",
            type_declaration(),
            INDEX_COPY_CONTRACT.clone(),
            specializer(),
        )
        .unwrap_err();
    assert_eq!(duplicate.kind_name(), "FunctionCatalogDuplicateSpecializer");
}

#[test]
fn exports_are_validated_and_indexed_by_exact_module_item() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_canonical_specializer_with_contract(
            "math/add",
            maintained_source_type_declaration("math/add").unwrap(),
            INDEX_COPY_CONTRACT.clone(),
            specializer(),
        )
        .unwrap();
    builder
        .insert_export(export("math/add", "math", "add"))
        .unwrap();
    let catalog = builder.build().unwrap();

    assert_eq!(
        catalog.module_export("math", "add").unwrap().canonical_name,
        "math/add"
    );
    assert!(catalog.module_export("math", "Add").is_none());
    assert_eq!(
        catalog
            .exports_for_operation(OperationId::from_name("math/add"))
            .len(),
        1
    );

    let mut missing = FunctionCatalogBuilder::new();
    missing
        .insert_export(export("math/missing", "math", "missing"))
        .unwrap();
    let error = missing.build().err().expect("missing export must fail");
    assert_eq!(error.kind_name(), "FunctionCatalogMissingExportSpecializer");
}

#[test]
fn canonical_invocation_validation_fails_before_factory_dispatch() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory::<IndexUnaryFactory>(
            "IndexUnary",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            RuntimeFamilyId::from_name("IndexUnary"),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name("IndexUnary"))
        .unwrap();
    let output = ValueCell::from_exact(1usize).unwrap();
    let input = ValueCell::from_exact(2usize).unwrap();

    let before = FACTORY_CALLS.load(Ordering::SeqCst);
    let arity = entry
        .validate_invocation(&FunctionInvocation::nullary(output.clone()))
        .unwrap_err();
    assert!(arity.simple_message().contains("Expected 1 arguments"));
    assert_eq!(FACTORY_CALLS.load(Ordering::SeqCst), before);

    let wrong_type = entry
        .validate_invocation(&FunctionInvocation::unary(
            output.clone(),
            ValueCell::from_exact("wrong".to_string()).unwrap(),
        ))
        .unwrap_err();
    assert!(
        wrong_type.simple_message().contains("Input(0)"),
        "unexpected wrong-type error: {}",
        wrong_type.simple_message()
    );
    assert_eq!(FACTORY_CALLS.load(Ordering::SeqCst), before);

    let alias = entry
        .validate_invocation(&FunctionInvocation::unary(output.clone(), output.clone()))
        .unwrap_err();
    assert!(
        alias
            .simple_message()
            .contains("aliases canonical input cell 0"),
        "unexpected alias error: {}",
        alias.simple_message()
    );
    assert_eq!(FACTORY_CALLS.load(Ordering::SeqCst), before);

    entry
        .validate_invocation(&FunctionInvocation::unary(output, input))
        .unwrap();
    assert_eq!(FACTORY_CALLS.load(Ordering::SeqCst), before + 1);
}

#[cfg(feature = "matrix")]
#[test]
fn canonical_invocation_shape_validation_fails_closed() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory::<AnyUnaryFactory>(
            "SameShape",
            RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias),
            RuntimeFamilyId::from_name("SameShape"),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name("SameShape"))
        .unwrap();
    let output = ValueCell::dynamic_matrix(
        SchemaBody::Index,
        vec![2, 2].into_boxed_slice(),
        vec![crate::ValueDataDraft::Index(1); 4].into_boxed_slice(),
    )
    .unwrap();
    let input = ValueCell::dynamic_matrix(
        SchemaBody::Index,
        vec![3, 1].into_boxed_slice(),
        vec![crate::ValueDataDraft::Index(1); 3].into_boxed_slice(),
    )
    .unwrap();

    let error = entry
        .validate_invocation(&FunctionInvocation::unary(output, input))
        .unwrap_err();
    assert!(error.simple_message().contains("same_shape"));
}

#[cfg(feature = "native-plan")]
#[test]
fn native_linkage_is_preserved_and_invalid_metadata_is_rejected() {
    let linkage = NativeFunctionLinkage::for_factory::<IndexUnaryFactory>(
        "mech-core",
        "mech_core",
        "mech_core::__mech_native::install_index_unary",
        &[],
    )
    .unwrap();
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_runtime_factory_with_linkage::<IndexUnaryFactory>(
            "IndexUnaryLinked",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            linkage.clone(),
            RuntimeFamilyId::from_name("IndexUnaryLinked"),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let stored = catalog
        .runtime_entry(RuntimeFunctionId::from_name("IndexUnaryLinked"))
        .unwrap()
        .native_linkage
        .as_ref()
        .unwrap();
    assert_eq!(stored, &linkage);
    let capability = catalog
        .runtime_execution_capabilities()
        .next()
        .expect("linked factory has a capability row");
    assert!(capability.targets.contains(ExecutionTarget::DirectRuntime));
    assert!(capability.targets.contains(ExecutionTarget::Native));
    assert!(!capability.targets.contains(ExecutionTarget::ResidentCpu));
    assert!(!capability.targets.contains(ExecutionTarget::GpuBatch));

    let invalid = NativeFunctionLinkage {
        package: "Invalid Package",
        crate_name: "mech_core",
        installer_path: "mech_core::install",
        cargo_features: vec!["native-link", "runtime"],
    };
    let mut builder = FunctionCatalogBuilder::new();
    let error = builder
        .insert_runtime_factory_with_linkage::<IndexUnaryFactory>(
            "InvalidLinked",
            contract(RuntimeOutputAliasPolicy::DisallowInputAlias),
            invalid,
            RuntimeFamilyId::from_name("InvalidLinked"),
        )
        .unwrap_err();
    assert_eq!(error.kind_name(), "FunctionCatalogInvalidNativeLinkage");
}
