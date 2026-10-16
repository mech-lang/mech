#![cfg(feature = "full_compiler")]

use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, BuiltinScalarKind, CanonicalFunctionSpecializer,
    ChangeDetectionPolicy, DeliveryMode, ExternalInteraction, FunctionCatalog,
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, FunctionTypeDeclaration,
    InputKindScheme, InputPortLayout, InputPortPolicy, KindExpr, KindScheme, MResult,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ReactiveInstanceId,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentValueMut, ResidentValueRef, SchemaBody, ShapeRule, SpecializationContext,
    SpecializationInvocation, SpecializedFunction, ValueData,
};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn compile(source: &str, catalog: Arc<FunctionCatalog>) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x504f5354), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}\n{:?}",
        parsed.diagnostics
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let compiled = CanonicalSourceFrontend
        .compile_document_with_catalog(&document, catalog)
        .unwrap_or_else(|error| panic!("{source}\n{error:?}"));
    let output = compiled
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    (compiled.compile_artifact().unwrap(), output)
}

fn check(source: &str, expected: impl Fn(&ValueData)) {
    let catalog = mech_stdlib::source_catalog();
    let (original, output) = compile(source, catalog.clone());
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x504f5354, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        instance
            .turn(&[])
            .unwrap_or_else(|error| panic!("{source}\n{error:?}"));
        expected(instance.copied_output(output).unwrap().data());
    }
}

#[test]
fn returned_strings_matrices_and_records_use_existing_typed_selectors() {
    check(
        "Text() => (out<string>) := out := \"é👩🏽‍💻Z\".\nText()[2u64]\n",
        |value| {
            assert!(
                matches!(value, ValueData::String(text) if text.as_ref() == "👩🏽‍💻"),
                "{value:?}"
            );
        },
    );
    check(
        "Matrix() => (out<[u64]:2,2>) := out := [11u64 12u64; 21u64 22u64].\nMatrix()[2u64, 1u64]\n",
        |value| {
            assert!(matches!(value, ValueData::U64(21)), "{value:?}");
        },
    );
    check(
        "Record() => (out<{text<string>}>) := out := {text: \"éZ\"}.\nRecord().text[1u64]\n",
        |value| {
            assert!(
                matches!(value, ValueData::String(text) if text.as_ref() == "é"),
                "{value:?}"
            );
        },
    );
    check("(\"é👩🏽‍💻Z\")[2u64..=3u64]\n", |value| {
        assert!(
            matches!(value, ValueData::String(text) if text.as_ref() == "👩🏽‍💻Z"),
            "{value:?}"
        );
    });
}

#[test]
fn selected_scalar_expectation_does_not_constrain_a_matrix_call_base() {
    let source = "+> stats\ncounts := [2.0 3.0; 5.0 7.0]\ntotal<f64> := stats/sum/column(counts)[1u64]\ntotal\n";
    let (artifact, _) = compile(source, mech_stdlib::source_catalog());
    let operation = artifact
        .nodes()
        .iter()
        .filter_map(|node| node.as_operation())
        .find(|node| node.operation.canonical_name() == "stats/sum/column")
        .unwrap();
    let mech_core::ResolvedOperationContract::Declared(contract) =
        artifact.contracts().get(operation.contract).unwrap()
    else {
        panic!("unknown operation contract")
    };
    assert!(matches!(
        artifact
            .schemas()
            .get(contract.outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Matrix { .. }
    ));
    check(source, |value| {
        assert!(
            matches!(value, ValueData::F64(value) if value.to_f64() == 5.0),
            "{value:?}"
        )
    });
}

static BASE_CALLS: AtomicUsize = AtomicUsize::new(0);
struct ProbeSpecializer;
impl CanonicalFunctionSpecializer for ProbeSpecializer {
    fn specialize_invocation(
        &self,
        _: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        panic!(
            "this provider supports artifact resident execution; direct runtime specialization is not used"
        )
    }
}
fn probe_execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let delegate = kernel
        .retained_state::<BoundResidentKernel>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    BASE_CALLS.fetch_add(1, Ordering::SeqCst);
    // Keep the maintained producer's contract checks and managed String
    // materialization admission; this wrapper observes only call count.
    delegate.execute(&ProbeInputs(inputs), output)
}
struct ProbeInputs<'a>(&'a dyn ResidentKernelInputs);
impl ResidentKernelInputs for ProbeInputs<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        self.0.get(index)
    }
}
fn probe_bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let catalog = mech_stdlib::source_catalog();
    let entry = catalog
        .resident_factory(&["string".to_owned()], "concat")
        .ok_or(ResidentKernelBindError::UnsupportedContract)?;
    let delegate = (entry.factory)(request)?;
    Ok(BoundResidentKernel::new(probe_execute, Box::new([]))
        .with_retained_state(Arc::new(delegate)))
}
fn probe_catalog() -> Arc<FunctionCatalog> {
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::from_catalog(&maintained);
    let contract = OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(Box::new([
            InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
        ])),
        outputs: Box::new([OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::ExactScalar,
        }]),
        interaction: ExternalInteraction::Pure,
    };
    let scheme = KindScheme::new(
        Box::new([]),
        Box::new([]),
        InputKindScheme::Fixed(Box::new([
            KindExpr::Named(BuiltinScalarKind::String.kind_id()),
            KindExpr::Named(BuiltinScalarKind::String.kind_id()),
        ])),
        Box::new([KindExpr::Named(BuiltinScalarKind::String.kind_id())]),
        Box::new([]),
    )
    .unwrap();
    let operation = builder
        .insert_canonical_specializer_with_contract(
            "probe/text",
            FunctionTypeDeclaration::from_schemes(vec![scheme]),
            contract,
            Arc::new(ProbeSpecializer),
        )
        .unwrap();
    builder
        .insert_export(FunctionExport {
            operation,
            canonical_name: "probe/text".into(),
            module: Some("probe".into()),
            item: Some("text".into()),
            exposure: FunctionExposure::ModuleOnly,
        })
        .unwrap();
    builder
        .insert_resident_factory(
            ["probe"],
            "text",
            maintained
                .resident_factory(&["string".to_owned()], "concat")
                .unwrap()
                .implementation_memory,
            probe_bind,
        )
        .unwrap();
    Arc::new(builder.build().unwrap())
}

#[test]
fn a_selected_call_base_is_lowered_and_executed_once() {
    let catalog = probe_catalog();
    let (original, output) = compile(
        "+> probe\nprobe/text(text<string>, \"\")[2u64]\n",
        catalog.clone(),
    );
    assert_eq!(
        original
            .nodes()
            .iter()
            .filter_map(|node| node.as_operation())
            .filter(|node| node.operation.canonical_name() == "probe/text")
            .count(),
        1
    );
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        BASE_CALLS.store(0, Ordering::SeqCst);
        let mut instance = activate(
            ReactiveInstanceId::new(0x504f5355, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(BASE_CALLS.load(Ordering::SeqCst), 0);
        for (turn, (text, expected)) in [("é👩🏽‍💻Z", "👩🏽‍💻"), ("aβc", "β")].into_iter().enumerate()
        {
            let input = [text.to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap();
            assert_eq!(BASE_CALLS.load(Ordering::SeqCst), turn + 1);
            assert!(
                matches!(instance.copied_output(output).unwrap().data(), ValueData::String(text) if text.as_ref() == expected)
            );
        }
    }
}
