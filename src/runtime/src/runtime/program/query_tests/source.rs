#[cfg(feature = "semantic-compiler")]
use crate::ProgramCompiler;
#[cfg(feature = "semantic-compiler")]
use mech_core::{
    ExternalInteraction, ParsedProgram, ResolvedOperationContract, TransactionalEffectProtocol,
    TransactionalExternalContract, Value, ValueCell,
};
#[cfg(feature = "semantic-compiler")]
use mech_engine::decode_program_artifact_sections;
#[cfg(feature = "semantic-compiler")]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[cfg(feature = "semantic-compiler")]
use crate::runtime::test_support::providers::{TestAfterCommitEffect, test_runtime_builder};
#[cfg(feature = "semantic-compiler")]
use crate::{
    PreparedRuntimeEffect, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostInputDriver,
    RuntimeHostInputSource, RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

#[cfg(feature = "semantic-compiler")]
const PLANNING_WRITE_BASE_URI: &str = "counting://sink";

#[cfg(feature = "semantic-compiler")]
#[derive(Debug, Default)]
struct PlanningWriteCounters {
    send_preflights: AtomicUsize,
    payload_plans: AtomicUsize,
    prepares: AtomicUsize,
    deliveries: AtomicUsize,
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct PlanningWriteProvider {
    counters: Arc<PlanningWriteCounters>,
}

#[cfg(feature = "semantic-compiler")]
impl RuntimeResourceProvider for PlanningWriteProvider {
    fn scheme(&self) -> &str {
        "counting"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![PLANNING_WRITE_BASE_URI.to_string()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> mech_core::MResult<Value> {
        panic!("planning write fixture must not read {request:?}")
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static mech_core::OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then(crate::prepare_commit_compensate_contract)
    }

    fn plan_write(&self, request: crate::RuntimeResourceWriteCommand) -> mech_core::MResult<()> {
        let mech_core::ValueData::F64(value) = request.value.data() else {
            panic!("planning must preserve the concrete payload kind");
        };
        self.counters.payload_plans.fetch_add(1, Ordering::SeqCst);
        if value.to_f64() < 0.0 {
            return Err(mech_core::MechError::new(
                crate::RuntimeInvalidOperationError {
                    operation: "plan_write",
                    reason: "fixture rejects negative payloads".to_owned(),
                },
                None,
            ));
        }
        assert_eq!(value.to_f64(), 2.0);
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })
    }

    fn preflight_write(
        &self,
        request: RuntimeResourceWritePreflightRequest,
    ) -> mech_core::MResult<()> {
        assert_eq!(request.base_uri, PLANNING_WRITE_BASE_URI);
        assert_eq!(request.context_name, "sink");
        match request.intent {
            RuntimeResourceWriteIntent::Assign => {
                panic!("resident send-contract fixture must not preflight assignment")
            }
            RuntimeResourceWriteIntent::Send => {
                assert_eq!(request.path, "sent");
                self.counters.send_preflights.fetch_add(1, Ordering::SeqCst);
            }
        }
        Ok(())
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> mech_core::MResult<PreparedRuntimeEffect> {
        self.counters.prepares.fetch_add(1, Ordering::SeqCst);
        let deliveries = Arc::clone(&self.counters);
        let metadata = RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: self.scheme().to_string(),
            },
            request.operation.name(),
        )
        .with_resource(format!("{}/{}", request.base_uri, request.path));
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            TestAfterCommitEffect::new(metadata, move || {
                deliveries.deliveries.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        )))
    }
}

#[cfg(feature = "semantic-compiler")]
fn compiler_with_write_counters() -> (ProgramCompiler, Arc<PlanningWriteCounters>) {
    let counters = Arc::new(PlanningWriteCounters::default());
    let runtime = test_runtime_builder()
        .function_catalog(mech_stdlib::source_catalog())
        .resource_provider(Box::new(PlanningWriteProvider {
            counters: Arc::clone(&counters),
        }))
        .build_compiler()
        .unwrap();
    (runtime, counters)
}

#[cfg(feature = "semantic-compiler")]
const MODE_READ_BASE_URI: &str = "mode-read://input";

#[cfg(feature = "semantic-compiler")]
const EXECUTION_MODE_LIVE_SOURCE: &str = "@live := mode-read://input{:read(value)}\n\
     live-result := @live/value\n\
     live-result";

#[cfg(feature = "semantic-compiler")]
#[derive(Debug, Default)]
struct ExecutionModeCounters {
    resource_plans: AtomicUsize,
    resource_reads: AtomicUsize,
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct ExecutionModeReadProvider {
    counters: Arc<ExecutionModeCounters>,
}

#[cfg(feature = "semantic-compiler")]
impl RuntimeResourceProvider for ExecutionModeReadProvider {
    fn scheme(&self) -> &str {
        "mode-read"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![MODE_READ_BASE_URI.to_string()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> mech_core::MResult<Value> {
        assert_eq!(request.base_uri, MODE_READ_BASE_URI);
        assert_eq!(request.path, "value");
        self.counters.resource_reads.fetch_add(1, Ordering::SeqCst);
        ValueCell::from_exact(22.0_f64)?.snapshot()
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> mech_core::MResult<Value> {
        assert_eq!(request.base_uri, MODE_READ_BASE_URI);
        assert_eq!(request.path, "value");
        self.counters.resource_plans.fetch_add(1, Ordering::SeqCst);
        ValueCell::from_exact(11.0_f64)?.snapshot()
    }
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug, Default)]
struct LiveReadDriverCounters {
    attaches: AtomicUsize,
    starts: AtomicUsize,
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct CountingLiveReadDriver {
    counters: Arc<LiveReadDriverCounters>,
    live: bool,
}

#[cfg(feature = "semantic-compiler")]
impl RuntimeHostInputDriver for CountingLiveReadDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == MODE_READ_BASE_URI && source.path() == "value"
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> mech_core::MResult<()> {
        self.counters.attaches.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn start(&mut self) -> mech_core::MResult<()> {
        self.counters.starts.fetch_add(1, Ordering::SeqCst);
        self.live = true;
        Ok(())
    }

    fn stop(&mut self) -> mech_core::MResult<()> {
        self.live = false;
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.live
    }
}

#[cfg(feature = "semantic-compiler")]
fn compile_execution_mode_live_source() -> (Arc<ExecutionModeCounters>, Arc<LiveReadDriverCounters>)
{
    let resource_counters = Arc::new(ExecutionModeCounters::default());
    let driver_counters = Arc::new(LiveReadDriverCounters::default());
    let mut compiler = test_runtime_builder()
        .test_input_driver(CountingLiveReadDriver {
            counters: Arc::clone(&driver_counters),
            live: false,
        })
        .resource_provider(Box::new(ExecutionModeReadProvider {
            counters: Arc::clone(&resource_counters),
        }))
        .build_compiler()
        .unwrap();
    compiler.compile_source(EXECUTION_MODE_LIVE_SOURCE).unwrap();
    (resource_counters, driver_counters)
}

#[cfg(feature = "semantic-compiler")]
#[test]
fn plan_source_live_read_only_plans_without_binding_or_driver_effects() {
    let (resource_counters, driver_counters) = compile_execution_mode_live_source();

    assert_eq!(resource_counters.resource_plans.load(Ordering::SeqCst), 1);
    assert_eq!(resource_counters.resource_reads.load(Ordering::SeqCst), 0);
    assert_eq!(driver_counters.attaches.load(Ordering::SeqCst), 0);
    assert_eq!(driver_counters.starts.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "semantic-compiler")]
#[test]
fn provider_transaction_contract_reaches_the_source_program_artifact() {
    let (mut compiler, counters) = compiler_with_write_counters();

    let bytecode = compiler
        .compile_source(
            r#"@out := counting://sink{:write(sent)}
@out/sent <- 2.0
"#,
        )
        .map(|product| product.into_parts().1)
        .unwrap();
    let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
    let artifact = decode_program_artifact_sections(&parsed.artifact).unwrap();
    assert!(artifact.nodes().iter().any(|node| {
        matches!(
            node.as_operation().and_then(|operation| artifact.contracts().get(operation.contract)),
            Some(ResolvedOperationContract::Declared(contract))
                if matches!(
                    contract.interaction,
                    ExternalInteraction::TransactionalExternal(TransactionalExternalContract {
                        protocol: TransactionalEffectProtocol::PrepareCommitCompensate,
                    })
                )
        )
    }));
    assert_eq!(counters.send_preflights.load(Ordering::SeqCst), 1);
    assert_eq!(counters.prepares.load(Ordering::SeqCst), 0);
    assert_eq!(counters.deliveries.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "semantic-compiler")]
#[test]
fn canonical_ordinary_and_interactive_compilation_preflight_payloads_without_live_effects() {
    for route in 0..5 {
        for (payload, accepted) in [("1 + 1", true), ("-1.0", false)] {
            let counters = Arc::new(PlanningWriteCounters::default());
            let source = format!(
                "@out := counting://sink{{:write(sent)}}\nvalue := {payload}\n@out/sent <- value\n"
            );
            let mut resolver = crate::InMemorySourceResolver::new();
            resolver.insert_string("main.mec", source.clone()).unwrap();
            let mut compiler = test_runtime_builder()
                .function_catalog(mech_stdlib::source_catalog())
                .source_resolver(resolver)
                .resource_provider(Box::new(PlanningWriteProvider {
                    counters: Arc::clone(&counters),
                }))
                .build_compiler()
                .unwrap();
            let document = crate::SourceDocument::parse_resolved(
                "test:canonical-write-planning",
                mech_syntax::document::Revision(0),
                Arc::<str>::from(source),
                mech_syntax::document::ParseConfig::default(),
            )
            .unwrap();
            let result = match route {
                0 => compiler.compile_document(&document).map(|_| ()),
                1 => compiler.compile_interactive_document(&document).map(|_| ()),
                2 => compiler
                    .compile_canonical_root(crate::SourceRequest::new("main.mec"))
                    .map(|_| ()),
                3 => compiler
                    .compile_canonical_interactive_root(crate::SourceRequest::new("main.mec"))
                    .map(|_| ()),
                4 => compiler
                    .compile_document_artifact_with_inputs(
                        &document,
                        &std::collections::BTreeMap::new(),
                        &std::collections::BTreeSet::new(),
                    )
                    .map(|_| ()),
                _ => unreachable!(),
            };
            assert_eq!(result.is_ok(), accepted, "route={route}: {result:?}");
            assert_eq!(counters.payload_plans.load(Ordering::SeqCst), 1);
            assert_eq!(
                counters.send_preflights.load(Ordering::SeqCst),
                usize::from(accepted)
            );
            assert_eq!(counters.prepares.load(Ordering::SeqCst), 0);
            assert_eq!(counters.deliveries.load(Ordering::SeqCst), 0);
        }
    }
}
