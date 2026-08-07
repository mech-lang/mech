use super::super::{
    MechRuntime, MechSourceCode, ObjectId, ObjectRecord, ResourceBudgetExceededError,
    RuntimeConfig, RuntimeEventKind,
};
#[cfg(all(feature = "compiler", feature = "matrix", feature = "f64"))]
use mech_core::matrix::Matrix;
#[cfg(feature = "compiler")]
use mech_core::{
    BytecodeInstruction, MechError, ParsedProgram, ReactiveCellId, ReactiveDependencyKind,
    ReactiveNodeKind, Ref, Value, hash_str,
};
#[cfg(feature = "compiler")]
use mech_engine::MechProgram;
#[cfg(feature = "compiler")]
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[cfg(feature = "compiler")]
use crate::runtime::test_support::{
    capabilities::{grant_host_call, grant_read, grant_write},
    providers::{
        DeliberateHostCallError, RecordingTestOutput, TestAfterCommitEffect, TestOutputProvider,
        TestResourceProvider, test_provider_with, test_runtime_builder, test_runtime_with_host,
    },
};
#[cfg(feature = "compiler")]
use crate::{
    CapabilityId, InMemoryDocsProvider, PlannedPureHostFunction, PreparedRuntimeEffect,
    RuntimeCallContext, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostInputDriver,
    RuntimeHostInputSource, RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
    RuntimeValueSnapshot,
};

#[cfg(feature = "compiler")]
fn compile_bytecode_for_runtime(runtime: &MechRuntime, source: &str) -> Vec<u8> {
    let mut program = MechProgram::with_function_catalog(
        runtime.program.config.clone(),
        Arc::clone(&runtime.function_catalog),
    );
    program.run_string(source).unwrap();
    program.compile_bytecode().unwrap()
}

#[cfg(feature = "compiler")]
#[derive(Debug, PartialEq, Eq)]
struct ExternalPlanDescriptor {
    function: String,
    kind: ReactiveNodeKind,
    inputs: Vec<(usize, ReactiveDependencyKind)>,
    outputs: Vec<usize>,
}

#[cfg(feature = "compiler")]
fn external_plan_descriptors(runtime: &MechRuntime) -> Vec<ExternalPlanDescriptor> {
    fn canonical_cell(
        cells: &mut BTreeMap<u64, usize>,
        next_cell: &mut usize,
        cell: ReactiveCellId,
    ) -> usize {
        *cells.entry(cell.get()).or_insert_with(|| {
            let canonical = *next_cell;
            *next_cell += 1;
            canonical
        })
    }

    let plan = runtime.program.interpreter().plan();
    let plan = plan.borrow();
    let mut cells = BTreeMap::new();
    let mut next_cell = 0;
    plan.nodes
        .iter()
        .filter_map(|node| {
            let function = node.function.to_string();
            if !function.starts_with("External") {
                return None;
            }
            let inputs = node
                .inputs
                .iter()
                .map(|dependency| {
                    (
                        canonical_cell(&mut cells, &mut next_cell, dependency.cell),
                        dependency.kind,
                    )
                })
                .collect();
            let outputs = node
                .outputs
                .iter()
                .map(|cell| canonical_cell(&mut cells, &mut next_cell, *cell))
                .collect();
            Some(ExternalPlanDescriptor {
                function,
                kind: node.kind,
                inputs,
                outputs,
            })
        })
        .collect()
}

#[cfg(feature = "compiler")]
const PLANNING_WRITE_BASE_URI: &str = "counting://sink";

#[cfg(feature = "compiler")]
#[derive(Debug, Default)]
struct PlanningWriteCounters {
    assign_preflights: AtomicUsize,
    send_preflights: AtomicUsize,
    prepares: AtomicUsize,
    deliveries: AtomicUsize,
}

#[cfg(feature = "compiler")]
#[derive(Debug)]
struct PlanningWriteProvider {
    counters: Arc<PlanningWriteCounters>,
}

#[cfg(feature = "compiler")]
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

    fn preflight_write(
        &self,
        request: RuntimeResourceWritePreflightRequest,
    ) -> mech_core::MResult<()> {
        assert_eq!(request.base_uri, PLANNING_WRITE_BASE_URI);
        assert_eq!(request.context_name, "sink");
        match request.intent {
            RuntimeResourceWriteIntent::Assign => {
                assert_eq!(request.path, "assigned");
                self.counters
                    .assign_preflights
                    .fetch_add(1, Ordering::SeqCst);
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

#[cfg(feature = "compiler")]
fn planning_runtime_with_write_counters() -> (MechRuntime, Arc<PlanningWriteCounters>) {
    let counters = Arc::new(PlanningWriteCounters::default());
    let runtime = test_runtime_builder()
        .planning()
        .resource_provider(Box::new(PlanningWriteProvider {
            counters: Arc::clone(&counters),
        }))
        .build()
        .unwrap();
    (runtime, counters)
}

#[cfg(feature = "compiler")]
const MODE_READ_BASE_URI: &str = "mode-read://input";

#[cfg(feature = "compiler")]
const BROAD_READ_BASE_URI: &str = "test://broad-values";

#[cfg(feature = "compiler")]
#[derive(Debug, Default)]
struct ExecutionModeCounters {
    host_plans: AtomicUsize,
    host_invocations: AtomicUsize,
    resource_plans: AtomicUsize,
    resource_reads: AtomicUsize,
}

#[cfg(feature = "compiler")]
#[derive(Debug)]
struct ExecutionModeReadProvider {
    counters: Arc<ExecutionModeCounters>,
}

#[cfg(feature = "compiler")]
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
        Ok(Value::F64(Ref::new(22.0)))
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> mech_core::MResult<Value> {
        assert_eq!(request.base_uri, MODE_READ_BASE_URI);
        assert_eq!(request.path, "value");
        self.counters.resource_plans.fetch_add(1, Ordering::SeqCst);
        Ok(Value::F64(Ref::new(11.0)))
    }
}

#[cfg(feature = "compiler")]
fn execute_source_resource_value(path: &str, value: Value) -> Value {
    let provider = TestResourceProvider::new().with_value(BROAD_READ_BASE_URI, path, value);
    let mut runtime = test_runtime_builder()
        .resource_provider(Box::new(provider))
        .build()
        .unwrap();
    grant_read(&mut runtime, BROAD_READ_BASE_URI, path);

    runtime
        .run_string(&format!(
            "@input := {BROAD_READ_BASE_URI}{{:read({path})}}\n\
             result := @input/{path}\n\
             result"
        ))
        .unwrap()
        .into_value()
}

#[cfg(all(feature = "compiler", feature = "u8"))]
#[test]
fn execute_source_resource_read_accepts_non_planner_scalar_values() {
    match execute_source_resource_value("scalar", Value::U8(Ref::new(17))) {
        Value::U8(value) => assert_eq!(*value.borrow(), 17),
        other => panic!("expected u8 resource value, got {other:?}"),
    }
}

#[cfg(all(feature = "compiler", feature = "matrix", feature = "f64"))]
#[test]
fn execute_source_resource_read_accepts_matrix_values() {
    let result = execute_source_resource_value(
        "matrix",
        Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 2, 2)),
    );

    match result {
        Value::MatrixF64(value) => {
            assert_eq!(value.rows(), 2);
            assert_eq!(value.cols(), 2);
            assert_eq!(value.as_vec(), vec![1.0, 2.0, 3.0, 4.0]);
        }
        other => panic!("expected f64 matrix resource value, got {other:?}"),
    }
}

#[cfg(feature = "compiler")]
#[derive(Debug, Default)]
struct LiveReadDriverCounters {
    attaches: AtomicUsize,
    starts: AtomicUsize,
}

#[cfg(feature = "compiler")]
#[derive(Debug)]
struct CountingLiveReadDriver {
    counters: Arc<LiveReadDriverCounters>,
    live: bool,
}

#[cfg(feature = "compiler")]
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

#[cfg(feature = "compiler")]
fn run_execution_mode_live_source(
    planning: bool,
) -> (
    MechRuntime,
    Arc<ExecutionModeCounters>,
    Arc<LiveReadDriverCounters>,
    Value,
) {
    let resource_counters = Arc::new(ExecutionModeCounters::default());
    let driver_counters = Arc::new(LiveReadDriverCounters::default());
    let builder = test_runtime_builder()
        .test_input_driver(CountingLiveReadDriver {
            counters: Arc::clone(&driver_counters),
            live: false,
        })
        .resource_provider(Box::new(ExecutionModeReadProvider {
            counters: Arc::clone(&resource_counters),
        }));
    let builder = if planning {
        builder.planning()
    } else {
        builder
    };
    let mut runtime = builder.build().unwrap();
    grant_read(&mut runtime, MODE_READ_BASE_URI, "value");
    let result = runtime
        .run_string(
            "@live := mode-read://input{:read(value)}\n\
             live-result := @live/value\n\
             live-result",
        )
        .unwrap()
        .into_value();
    runtime.start_input_drivers().unwrap();

    (runtime, resource_counters, driver_counters, result)
}

#[cfg(feature = "compiler")]
fn run_execution_mode_source(planning: bool) -> (MechRuntime, Arc<ExecutionModeCounters>, Value) {
    let counters = Arc::new(ExecutionModeCounters::default());
    let host_plan_counters = Arc::clone(&counters);
    let host_invoke_counters = Arc::clone(&counters);
    let builder = test_runtime_builder()
        .resource_provider(Box::new(ExecutionModeReadProvider {
            counters: Arc::clone(&counters),
        }))
        .host_function(PlannedPureHostFunction::new(
            "test/execution-mode",
            move |_context: &RuntimeCallContext, _arguments: &[RuntimeValueSnapshot]| {
                host_plan_counters.host_plans.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeValueSnapshot::try_capture(&Value::F64(Ref::new(
                    33.0,
                )))?)
            },
            move |_context: &RuntimeCallContext, _arguments: Vec<RuntimeValueSnapshot>| {
                host_invoke_counters
                    .host_invocations
                    .fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeValueSnapshot::try_capture(&Value::F64(Ref::new(
                    44.0,
                )))?)
            },
        ))
        .unwrap();
    let builder = if planning {
        builder.planning()
    } else {
        builder
    };
    let mut runtime = builder.build().unwrap();
    grant_read(&mut runtime, MODE_READ_BASE_URI, "value");
    grant_host_call(&mut runtime, CapabilityId(889), "test/execution-mode");

    let result = runtime
        .run_string(
            "@input := mode-read://input{:read(value)}\n\
             resource-result := @input/value\n\
             host-result := test/execution-mode()\n\
             host-result",
        )
        .unwrap()
        .into_value();

    (runtime, counters, result)
}

#[cfg(feature = "compiler")]
fn assert_f64_value(value: Value, expected: f64, label: &str) {
    match value {
        Value::F64(value) => assert_eq!(*value.borrow(), expected, "{label}"),
        other => panic!("expected {label} to be F64({expected}), got {other:?}"),
    }
}

#[cfg(feature = "compiler")]
#[test]
fn execute_source_uses_actual_host_and_resource_values_once() {
    let (runtime, counters, result) = run_execution_mode_source(false);

    assert_f64_value(result, 44.0, "source result");
    assert_f64_value(
        runtime
            .program
            .root_symbol_value("resource-result")
            .unwrap(),
        22.0,
        "resource result",
    );
    assert_f64_value(
        runtime.program.root_symbol_value("host-result").unwrap(),
        44.0,
        "host result",
    );
    assert_eq!(counters.host_plans.load(Ordering::SeqCst), 1);
    assert_eq!(counters.host_invocations.load(Ordering::SeqCst), 1);
    assert_eq!(counters.resource_plans.load(Ordering::SeqCst), 0);
    assert_eq!(counters.resource_reads.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "compiler")]
#[test]
fn plan_source_uses_only_host_and_resource_planning_values_once() {
    let (runtime, counters, result) = run_execution_mode_source(true);

    assert_f64_value(result, 33.0, "source result");
    assert_f64_value(
        runtime
            .program
            .root_symbol_value("resource-result")
            .unwrap(),
        11.0,
        "resource result",
    );
    assert_f64_value(
        runtime.program.root_symbol_value("host-result").unwrap(),
        33.0,
        "host result",
    );
    assert_eq!(counters.host_plans.load(Ordering::SeqCst), 1);
    assert_eq!(counters.host_invocations.load(Ordering::SeqCst), 0);
    assert_eq!(counters.resource_plans.load(Ordering::SeqCst), 1);
    assert_eq!(counters.resource_reads.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "compiler")]
#[test]
fn execute_source_live_read_reads_once_retains_binding_and_starts_driver() {
    let (runtime, resource_counters, driver_counters, result) =
        run_execution_mode_live_source(false);

    assert_f64_value(result, 22.0, "execute live result");
    assert_eq!(resource_counters.resource_reads.load(Ordering::SeqCst), 1);
    assert_eq!(resource_counters.resource_plans.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.live_input_binding_count(), 1);
    assert_eq!(driver_counters.attaches.load(Ordering::SeqCst), 1);
    assert_eq!(driver_counters.starts.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "compiler")]
#[test]
fn plan_source_live_read_only_plans_without_binding_or_driver_effects() {
    let (runtime, resource_counters, driver_counters, result) =
        run_execution_mode_live_source(true);

    assert_f64_value(result, 11.0, "planned live result");
    assert_eq!(resource_counters.resource_plans.load(Ordering::SeqCst), 1);
    assert_eq!(resource_counters.resource_reads.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.live_input_binding_count(), 0);
    assert_eq!(driver_counters.attaches.load(Ordering::SeqCst), 0);
    assert_eq!(driver_counters.starts.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "compiler")]
#[test]
fn planning_top_level_writes_preflight_once_through_external_initialization() {
    let (mut runtime, counters) = planning_runtime_with_write_counters();
    grant_write(&mut runtime, PLANNING_WRITE_BASE_URI, "assigned");
    grant_write(&mut runtime, PLANNING_WRITE_BASE_URI, "sent");

    runtime
        .run_string(
            r#"@out := counting://sink{:write(assigned), :write(sent)}
@out/assigned = 1.0
@out/sent <- 2.0
"#,
        )
        .unwrap();

    assert_eq!(counters.assign_preflights.load(Ordering::SeqCst), 1);
    assert_eq!(counters.send_preflights.load(Ordering::SeqCst), 1);
    assert_eq!(counters.prepares.load(Ordering::SeqCst), 0);
    assert_eq!(counters.deliveries.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "compiler")]
#[test]
fn planning_activation_send_preflights_once_without_external_initialization() {
    let (mut runtime, counters) = planning_runtime_with_write_counters();
    grant_write(&mut runtime, PLANNING_WRITE_BASE_URI, "sent");

    runtime
        .run_string(
            r#"@out := counting://sink{:write(sent)}
trigger := 0.0
~> trigger {
  @out/sent <- trigger
}
"#,
        )
        .unwrap();

    assert_eq!(counters.assign_preflights.load(Ordering::SeqCst), 0);
    assert_eq!(counters.send_preflights.load(Ordering::SeqCst), 1);
    assert_eq!(counters.prepares.load(Ordering::SeqCst), 0);
    assert_eq!(counters.deliveries.load(Ordering::SeqCst), 0);
}

#[test]
fn run_string_with_context_emits_profile_event_when_enabled() {
    let mut config = RuntimeConfig::default();
    config.diagnostics.profile_enabled = true;
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .config(config)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();

    runtime
        .run_string_with_context(&mut context, "profiled := 1")
        .unwrap();

    assert!(context.events.iter().any(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::ProgramProfiled { duration_ns, .. } if duration_ns > 0
        )
    }));
}

#[test]
fn run_string_with_context_emits_profile_event_on_failure_when_enabled() {
    let mut config = RuntimeConfig::default();
    config.diagnostics.profile_enabled = true;
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    assert!(
        runtime
            .run_string_with_context(&mut context, "1 +")
            .is_err()
    );

    assert!(context.events.iter().any(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::ProgramProfiled { duration_ns, .. } if duration_ns > 0
        )
    }));
}

#[test]
fn max_source_bytes_rejects_string_source() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .run_string_with_context(&mut context, "1234")
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.used, 0);
    assert_eq!(budget.requested, 4);
    assert_eq!(budget.max, Some(3));
}

#[test]
fn direct_string_source_limit_uses_borrowed_length() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let source = String::from("1234");

    let error = runtime
        .run_string_with_context(&mut context, &source)
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.requested, 4);
    assert_eq!(source, "1234");
}

#[test]
fn direct_bytecode_source_limit_uses_borrowed_length() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let bytecode = vec![1, 2, 3, 4];

    let error = runtime
        .evaluate_bytecode_once_with_context(&mut context, &bytecode)
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.requested, 4);
    assert_eq!(bytecode, vec![1, 2, 3, 4]);
}

#[test]
fn context_event_retention_is_bounded() {
    crate::runtime::gate_a_probe::reset_gate_a_costs();
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(2);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .put_object_with_context(&mut context, ObjectRecord::text(ObjectId(1), "text", "one"))
        .unwrap();
    runtime
        .put_object_with_context(&mut context, ObjectRecord::text(ObjectId(2), "text", "two"))
        .unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(3), "text", "three"),
        )
        .unwrap();
    let object_ids = context
        .events
        .iter()
        .filter_map(|event| match event.kind {
            RuntimeEventKind::ObjectCreated { object_id } => Some(object_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(object_ids, vec![ObjectId(2), ObjectId(3)]);
    let costs = crate::runtime::gate_a_probe::gate_a_cost_snapshot();
    assert!(costs.context_event_compaction_count > 0);
    assert_eq!(
        costs.context_event_compaction_moved_items,
        costs.context_event_compaction_count * 2,
    );
}

#[test]
fn max_source_bytes_rejects_program_aggregate() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let source = MechSourceCode::Program(vec![
        MechSourceCode::String("1".to_string()),
        MechSourceCode::String("22".to_string()),
        MechSourceCode::String("3".to_string()),
    ]);

    let error = runtime
        .run_source_with_context(&mut context, &source)
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.requested, 4);
    assert_eq!(budget.max, Some(3));
}

#[test]
fn max_memory_bytes_rejects_large_source_buffer() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(100);
    config.limits.max_memory_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .run_string_with_context(&mut context, "1234")
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "bytes");
    assert_eq!(budget.used, 0);
    assert_eq!(budget.requested, 4);
    assert_eq!(budget.max, Some(3));
}

#[test]
fn tree_source_without_known_size_is_not_rejected_by_source_byte_limit() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(1);
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .config(config)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let tree = mech_syntax::parser::parse("tree-value := 1").unwrap();
    let source = MechSourceCode::Tree(tree);

    runtime
        .run_source_with_context(&mut context, &source)
        .unwrap();
}

#[cfg(feature = "compiler")]
#[test]
fn retained_bytecode_install_replaces_the_root_program() {
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .build()
        .unwrap();
    runtime.run_string("old-root := 1.0").unwrap();
    let bytecode = compile_bytecode_for_runtime(&runtime, "installed-root := 2.0");
    let mut context = runtime.runtime_context().unwrap();

    runtime
        .install_bytecode_with_context(&mut context, &bytecode)
        .unwrap();

    assert!(runtime.program.root_symbol_value("old-root").is_err());
    match runtime.program.root_symbol_value("installed-root").unwrap() {
        Value::F64(value) => assert_eq!(*value.borrow(), 2.0),
        other => panic!("expected f64 installed root, got {other:?}"),
    }
}

#[cfg(feature = "compiler")]
#[test]
fn explicit_repeated_bytecode_installs_restore_the_owned_original_on_abort() {
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .build()
        .unwrap();
    runtime.run_string("original-root := 1.0").unwrap();
    let original = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("original-root"))
        .unwrap();
    let original_address = original.as_ptr();
    let first = compile_bytecode_for_runtime(&runtime, "first-installed-root := 2.0");
    let second = compile_bytecode_for_runtime(&runtime, "second-installed-root := 3.0");
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .install_bytecode_with_context(&mut context, &first)
        .unwrap();
    assert!(runtime.program.root_symbol_value("original-root").is_err());
    assert!(
        runtime
            .program
            .root_symbol_value("first-installed-root")
            .is_ok()
    );
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .program
            .as_ref()
            .unwrap()
            .replacement_predecessors
            .len(),
        1,
    );

    runtime
        .install_bytecode_with_context(&mut context, &second)
        .unwrap();
    assert!(
        runtime
            .program
            .root_symbol_value("first-installed-root")
            .is_err()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("second-installed-root")
            .is_ok()
    );
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .program
            .as_ref()
            .unwrap()
            .replacement_predecessors
            .len(),
        2,
    );

    runtime
        .abort_runtime_transaction(&mut context, "discard repeated installs")
        .unwrap();

    let restored = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("original-root"))
        .unwrap();
    assert_eq!(restored.as_ptr(), original_address);
    assert!(
        runtime
            .program
            .root_symbol_value("first-installed-root")
            .is_err()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("second-installed-root")
            .is_err()
    );
    assert_eq!(runtime.program_transaction_owner, None);
}

#[cfg(feature = "compiler")]
#[test]
fn failed_retained_bytecode_install_restores_program_and_live_bindings() {
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .build()
        .unwrap();
    runtime.run_string("rollback-anchor := 1.0").unwrap();
    let source = crate::RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap();
    let target = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("rollback-anchor"))
        .unwrap();
    let target_address = target.as_ptr();
    runtime.live_input_bindings.insert(
        source.clone(),
        vec![crate::RuntimeLiveResourceBinding {
            interpreter_id: runtime.program.interpreter().id,
            source: source.clone(),
            target: target.clone(),
        }],
    );
    let mut context = runtime.runtime_context().unwrap();
    runtime.commit_live_context_candidate(&context);
    let mut bytecode = compile_bytecode_for_runtime(&runtime, "partial-install := 2.0");
    bytecode.pop();

    assert!(
        runtime
            .install_bytecode_with_context(&mut context, &bytecode)
            .is_err()
    );

    let restored = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("rollback-anchor"))
        .unwrap();
    assert_eq!(restored.as_ptr(), target_address);
    assert!(
        runtime
            .program
            .root_symbol_value("partial-install")
            .is_err()
    );
    assert!(runtime.live_context_template.is_some());
    let bindings = runtime.live_input_bindings.get(&source).unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].target.as_ptr(), target_address);
    match bindings[0].target.borrow().clone() {
        Value::F64(value) => assert_eq!(*value.borrow(), 1.0),
        other => panic!("expected f64 rollback anchor, got {other:?}"),
    }
}

#[cfg(feature = "compiler")]
#[test]
fn runtime_source_external_operations_compile_and_reconstruct_an_equivalent_plan() {
    let host_invocations = Arc::new(AtomicUsize::new(0));
    let invocation_count = Arc::clone(&host_invocations);
    let output = RecordingTestOutput::default();
    let docs = InMemoryDocsProvider::new()
        .with_value("docs://manual", "item", Value::F64(Ref::new(0.0)))
        .unwrap();
    let mut runtime = test_runtime_builder()
        .resource_provider(Box::new(test_provider_with(
            "test://clock/ticks",
            "value",
            3.0,
        )))
        .resource_provider(Box::new(TestOutputProvider::new(output.clone())))
        .in_memory_docs(docs)
        .host_function(PlannedPureHostFunction::new(
            "test/external-equivalence",
            |_context: &RuntimeCallContext, arguments: &[RuntimeValueSnapshot]| {
                Ok(arguments[0].clone())
            },
            move |_context: &RuntimeCallContext, arguments: Vec<RuntimeValueSnapshot>| {
                invocation_count.fetch_add(1, Ordering::SeqCst);
                Ok(arguments[0].clone())
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_read(&mut runtime, "test://clock/ticks", "value");
    grant_write(&mut runtime, "docs://manual", "item");
    grant_write(&mut runtime, "test://effects/output", "line");
    grant_host_call(&mut runtime, CapabilityId(890), "test/external-equivalence");

    runtime
        .run_string(
            r#"@clock := test://clock/ticks{:read(value)}
@doc := docs://manual{:write(item)}
@out := test://effects/output{:write(line)}
hosted := test/external-equivalence(@clock/value)
@doc/item = hosted
@out/line <- hosted
"#,
        )
        .unwrap();
    assert_eq!(host_invocations.load(Ordering::SeqCst), 1);
    assert_eq!(output.lines().len(), 1);
    let source_plan = external_plan_descriptors(&runtime);
    assert_eq!(source_plan.len(), 4);

    let bytecode = runtime.compile_program_bytecode().unwrap();
    let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
    let external_instructions = parsed
        .instructions
        .iter()
        .filter_map(|instruction| match instruction {
            BytecodeInstruction::HostCall { .. } => Some("HostCall"),
            BytecodeInstruction::ResourceRead { .. } => Some("ResourceRead"),
            BytecodeInstruction::ResourceWrite { .. } => Some("ResourceWrite"),
            BytecodeInstruction::ResourceSend { .. } => Some("ResourceSend"),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        external_instructions,
        vec!["ResourceRead", "HostCall", "ResourceWrite", "ResourceSend"]
    );

    let mut context = runtime.runtime_context().unwrap();
    runtime
        .install_bytecode_with_context(&mut context, &bytecode)
        .unwrap();

    assert_eq!(host_invocations.load(Ordering::SeqCst), 2);
    assert_eq!(output.lines().len(), 2);
    assert_eq!(external_plan_descriptors(&runtime), source_plan);
}

#[cfg(feature = "compiler")]
#[test]
fn failed_valid_bytecode_install_removes_a_new_live_binding() {
    let mut compiler = test_runtime_with_host(
        test_provider_with("test://clock/ticks", "value", 1.0),
        PlannedPureHostFunction::new(
            "test/fail-after-live-read",
            |_context: &RuntimeCallContext, arguments: &[RuntimeValueSnapshot]| {
                Ok(arguments[0].clone())
            },
            |_context: &RuntimeCallContext, arguments: Vec<RuntimeValueSnapshot>| {
                Ok(arguments[0].clone())
            },
        ),
    );
    grant_read(&mut compiler, "test://clock/ticks", "value");
    grant_host_call(
        &mut compiler,
        CapabilityId(891),
        "test/fail-after-live-read",
    );
    compiler
        .run_string(
            "@clock := test://clock/ticks{:read(value)}\n\
             failed-result := test/fail-after-live-read(@clock/value)",
        )
        .unwrap();
    let bytecode = compiler.compile_program_bytecode().unwrap();
    let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
    let read_index = parsed
        .instructions
        .iter()
        .position(|instruction| matches!(instruction, BytecodeInstruction::ResourceRead { .. }))
        .unwrap();
    let host_index = parsed
        .instructions
        .iter()
        .position(|instruction| matches!(instruction, BytecodeInstruction::HostCall { .. }))
        .unwrap();
    assert!(read_index < host_index);

    let invocations = Arc::new(AtomicUsize::new(0));
    let invocation_count = Arc::clone(&invocations);
    let mut runtime = test_runtime_with_host(
        test_provider_with("test://clock/ticks", "value", 1.0),
        PlannedPureHostFunction::new(
            "test/fail-after-live-read",
            |_context: &RuntimeCallContext, arguments: &[RuntimeValueSnapshot]| {
                Ok(arguments[0].clone())
            },
            move |_context: &RuntimeCallContext, _arguments: Vec<RuntimeValueSnapshot>| {
                invocation_count.fetch_add(1, Ordering::SeqCst);
                Err(MechError::new(DeliberateHostCallError, None))
            },
        ),
    );
    grant_read(&mut runtime, "test://clock/ticks", "value");
    grant_host_call(&mut runtime, CapabilityId(892), "test/fail-after-live-read");
    runtime.run_string("rollback-anchor := 7.0").unwrap();
    assert_eq!(runtime.live_input_binding_count(), 0);
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .install_bytecode_with_context(&mut context, &bytecode)
        .unwrap_err();

    assert!(
        error.kind_as::<DeliberateHostCallError>().is_some(),
        "unexpected install failure: {error:?}"
    );
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
    assert_eq!(runtime.live_input_binding_count(), 0);
    assert!(runtime.program.root_symbol_value("failed-result").is_err());
    match runtime
        .program
        .root_symbol_value("rollback-anchor")
        .unwrap()
    {
        Value::F64(value) => assert_eq!(*value.borrow(), 7.0),
        other => panic!("expected f64 rollback anchor, got {other:?}"),
    }
}
