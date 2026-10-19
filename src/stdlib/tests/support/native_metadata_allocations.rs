//! Diagnostic-only windows around actual maintained public bind/invoke APIs.
//! No frontend/catalog/input preparation or printing occurs inside a window.
//! Invoke observations include real output payload/ownership allocations:
//! these are NOT isolated metadata bytes, timing, or an analytical bound.
use super::{admit_value, artifacts};
use mech_core::{
    ManagedMemoryBudget, SchemaBody, Value, ValueDataDraft, ValueDraft,
    snapshot::SnapshotValidationContext,
};
use mech_engine::native_artifact::{
    NativeIterationAction, NativeLibraryWorkBudget, NativeProgramLibraryTemplate,
    bind_native_program_library_call, lower_native_program_artifact,
    prepare_native_program_library_call,
};
use std::{
    alloc::{GlobalAlloc, Layout},
    cell::RefCell,
};

// Reuse the existing successful test-suite allocator delegate and scoped
// allocation request/byte counters; add only an opt-in peak observer here.
#[path = "../../../core/tests/support/allocation_probe.rs"]
mod allocation_probe;

#[test]
fn shared_probe_count_only_window_resets_before_byte_window() {
    let (data, requests) = allocation_probe::measured(|| Vec::<u8>::with_capacity(64));
    assert!(requests > 0);
    assert!(data.capacity() >= 64);
    drop(data);
    let (data, requests, bytes) =
        allocation_probe::measured_with_bytes(|| Vec::<u8>::with_capacity(64));
    assert!(requests > 0);
    assert!(bytes >= 64);
    assert!(data.capacity() >= 64);
}

struct NativeMetadataAllocator;
#[global_allocator]
static ALLOCATOR: NativeMetadataAllocator = NativeMetadataAllocator;

// This preallocated ledger cannot grow while observation is active. It tracks
// only pointers born/reallocated inside the window; dropping preexisting input
// or compiler allocations does not create a negative or invented live count.
const MAX_LIVE_ALLOCATIONS: usize = 32_768;
struct Peak {
    live: Vec<(usize, usize)>,
    current: usize,
    maximum: usize,
    largest_request: usize,
    complete: bool,
}
impl Peak {
    fn allocated(&mut self, pointer: *mut u8, bytes: usize) {
        if pointer.is_null() {
            return;
        }
        if self.live.len() == self.live.capacity() {
            self.complete = false;
            return;
        }
        let Some(current) = self.current.checked_add(bytes) else {
            self.complete = false;
            return;
        };
        self.live.push((pointer as usize, bytes));
        self.current = current;
        self.maximum = self.maximum.max(current);
        self.largest_request = self.largest_request.max(bytes);
    }
    fn freed(&mut self, pointer: *mut u8) {
        if let Some(index) = self
            .live
            .iter()
            .position(|(saved, _)| *saved == pointer as usize)
        {
            let (_, bytes) = self.live.swap_remove(index);
            if let Some(current) = self.current.checked_sub(bytes) {
                self.current = current;
            } else {
                self.complete = false;
            }
        }
    }
}
thread_local! {
    static PEAK: RefCell<Option<Peak>> = const { RefCell::new(None) };
}
fn observe(change: impl FnOnce(&mut Peak)) {
    let _ = PEAK.try_with(|slot| {
        // The observer never allocates, formats, or dereferences tracked bytes.
        // A reentrant teardown hook is ignored after the window is removed.
        if let Ok(mut slot) = slot.try_borrow_mut() {
            if let Some(peak) = slot.as_mut() {
                change(peak);
            }
        }
    });
}
// SAFETY: every allocation request/pointer/layout is delegated unchanged to
// the existing test probe (which delegates to System). Successful pointers
// are recorded only as integer identities, never inspected or dereferenced.
unsafe impl GlobalAlloc for NativeMetadataAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { allocation_probe::ProbeAllocator.alloc(layout) };
        observe(|peak| peak.allocated(pointer, layout.size()));
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { allocation_probe::ProbeAllocator.alloc_zeroed(layout) };
        observe(|peak| peak.allocated(pointer, layout.size()));
        pointer
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, bytes: usize) -> *mut u8 {
        let next = unsafe { allocation_probe::ProbeAllocator.realloc(pointer, layout, bytes) };
        if !next.is_null() {
            observe(|peak| {
                peak.freed(pointer);
                peak.allocated(next, bytes);
            });
        }
        next
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        observe(|peak| peak.freed(pointer));
        unsafe { allocation_probe::ProbeAllocator.dealloc(pointer, layout) };
    }
}

#[derive(Debug)]
struct Observation {
    requests: usize,
    requested_bytes: usize,
    peak_live_bytes: usize,
    live_at_return: usize,
    largest_request: usize,
    complete: bool,
}
fn measured<T>(run: impl FnOnce() -> T) -> (T, Observation) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            let old = PEAK.with(|slot| slot.borrow_mut().take());
            drop(old);
        }
    }
    let peak = Peak {
        live: Vec::with_capacity(MAX_LIVE_ALLOCATIONS),
        current: 0,
        maximum: 0,
        largest_request: 0,
        complete: true,
    };
    PEAK.with(|slot| assert!(slot.borrow_mut().replace(peak).is_none()));
    let reset = Reset;
    let (result, requests, requested_bytes) = allocation_probe::measured_with_bytes(run);
    let peak = PEAK.with(|slot| slot.borrow_mut().take().unwrap());
    let observation = Observation {
        requests,
        requested_bytes,
        peak_live_bytes: peak.maximum,
        live_at_return: peak.current,
        largest_request: peak.largest_request,
        complete: peak.complete,
    };
    drop((peak, reset));
    (result, observation)
}
fn report(
    case: &str,
    decoded: bool,
    phase: &str,
    observation: &Observation,
    work: (u64, u64),
    ok: bool,
) {
    assert!(
        observation.complete,
        "diagnostic ledger exhausted: {case}/{phase}"
    );
    eprintln!(
        "native-allocation-diagnostic {}",
        serde_json::json!({
            "case":case, "decoded":decoded, "phase":phase,
            "allocation_requests":observation.requests,
            "requested_bytes":observation.requested_bytes,
            "observed_peak_live_bytes":observation.peak_live_bytes,
            "observed_live_bytes_at_return":observation.live_at_return,
            "largest_successful_request_bytes":observation.largest_request,
            "compute_work_delta":work.0, "comparison_work_delta":work.1,
            "success":ok,
            "scope":"calling thread only; allocations born/reallocated inside window; logical requested-layout peak at allocator-call boundaries, not allocator-internal realloc overlap",
            "classification":"diagnostic, not timing or an analytical admission witness"
        })
    );
}
fn observe_metadata<T>(case: &str, decoded: bool, phase: &str, run: impl FnOnce() -> T) {
    let (value, observation) = measured(run);
    report(case, decoded, phase, &observation, (0, 0), true);
    // Retire the observed metadata outside the allocation window. Retirement
    // does not allocate here, and live-at-return describes exactly this clone.
    drop(value);
}
fn work_delta(before: (u64, u64), after: (u64, u64)) -> (u64, u64) {
    (
        after.0.checked_sub(before.0).unwrap(),
        after.1.checked_sub(before.1).unwrap(),
    )
}
fn args(
    artifact: &mech_engine::ProgramArtifact,
    template: &NativeProgramLibraryTemplate,
    memory: &ManagedMemoryBudget,
) -> Vec<Option<Value>> {
    let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
        panic!("declared call")
    };
    template
        .inputs()
        .iter()
        .zip(&contract.inputs)
        .enumerate()
        .map(|(index, (schema, port))| {
            let (shape_values, data): (Box<[u64]>, ValueDataDraft) = match schema.body() {
                SchemaBody::String => (
                    Box::new([]) as Box<[u64]>,
                    ValueDataDraft::String(format!("{index}A🧑‍🚀e\u{301}")),
                ),
                SchemaBody::Bool => (Box::new([]), ValueDataDraft::Bool(true)),
                SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64) => {
                    (Box::new([]), ValueDataDraft::U64(1))
                }
                SchemaBody::Matrix { element, .. }
                    if matches!(
                        element.as_ref(),
                        SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
                    ) =>
                {
                    let shape = mech_core::shape_for_resolved_extents(schema, &[1, 3]).unwrap();
                    (
                        shape.parameter_values().to_vec().into_boxed_slice(),
                        ValueDataDraft::Matrix(
                            vec![
                                ValueDataDraft::U64(1),
                                ValueDataDraft::U64(2),
                                ValueDataDraft::U64(1),
                            ]
                            .into_boxed_slice(),
                        ),
                    )
                }
                body => panic!("unsupported diagnostic argument {body:?}"),
            };
            Some(admit_value(
                ValueDraft {
                    schema: port.schema,
                    shape_values,
                    data,
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap(),
                memory,
            ))
        })
        .collect()
}
fn metadata_sizes() {
    use core::mem::size_of;
    let sizes = [
        (
            "NativeCollectionCall",
            size_of::<mech_engine::native_artifact::NativeCollectionCall>(),
        ),
        (
            "NativePreparedLibraryCall",
            size_of::<mech_engine::native_artifact::NativePreparedLibraryCall>(),
        ),
        (
            "NativeProgramLibraryTemplate",
            size_of::<NativeProgramLibraryTemplate>(),
        ),
        (
            "ProgramMemoryPlan",
            size_of::<mech_engine::memory_planner::ProgramMemoryPlan>(),
        ),
        (
            "TurnMemoryPlan",
            size_of::<mech_engine::memory_planner::TurnMemoryPlan>(),
        ),
        (
            "TurnMemoryFacts",
            size_of::<mech_engine::memory_planner::TurnMemoryFacts>(),
        ),
        ("CallMemoryPlan", size_of::<mech_core::CallMemoryPlan>()),
        ("BoundCall", size_of::<mech_core::BoundCall>()),
        (
            "BoundResidentKernel",
            size_of::<mech_core::BoundResidentKernel>(),
        ),
        ("PortMemoryPlan", size_of::<mech_core::PortMemoryPlan>()),
        (
            "ResidentPortLayout",
            size_of::<mech_core::ResidentPortLayout>(),
        ),
        (
            "ResolvedValueDescriptor",
            size_of::<mech_core::ResolvedValueDescriptor>(),
        ),
        (
            "ResolvedOperationDescriptor",
            size_of::<mech_core::ResolvedOperationDescriptor>(),
        ),
        (
            "ResolvedOperationContract",
            size_of::<mech_core::ResolvedOperationContract>(),
        ),
        (
            "ResidentKernelBindRequest",
            size_of::<mech_core::ResidentKernelBindRequest<'static>>(),
        ),
        (
            "CallMemoryPlanningRequest",
            size_of::<mech_core::CallMemoryPlanningRequest<'static>>(),
        ),
        ("ResolvedType", size_of::<mech_core::ResolvedType>()),
        ("ValueLayoutPlan", size_of::<mech_core::ValueLayoutPlan>()),
        ("AxisCapacityPlan", size_of::<mech_core::AxisCapacityPlan>()),
        ("AllocationPlan", size_of::<mech_core::AllocationPlan>()),
        ("ArenaPlan", size_of::<mech_core::ArenaPlan>()),
        (
            "TransactionRequirement",
            size_of::<mech_core::TransactionRequirement>(),
        ),
        (
            "DeferredMemoryWitness",
            size_of::<mech_core::DeferredMemoryWitness>(),
        ),
        (
            "CurrentMemoryFootprint",
            size_of::<mech_core::CurrentMemoryFootprint>(),
        ),
        ("ResourceDemand", size_of::<mech_core::ResourceDemand>()),
        (
            "PhysicalStorageDescriptor",
            size_of::<mech_core::PhysicalStorageDescriptor>(),
        ),
        ("SchemaTable", size_of::<mech_core::SchemaTable>()),
        ("SchemaEntry", size_of::<mech_core::SchemaEntry>()),
        ("Schema", size_of::<mech_core::Schema>()),
        ("Value", size_of::<Value>()),
        ("RegionAccessPlan", size_of::<mech_core::RegionAccessPlan>()),
        (
            "ResidentValueRef",
            size_of::<mech_core::ResidentValueRef<'static>>(),
        ),
    ];
    eprintln!(
        "native-metadata-size-diagnostic {}",
        serde_json::json!({
            "sizes": sizes, "largest_inline_type": sizes.iter().max_by_key(|(_, bytes)| *bytes),
            "boundary":"inline Rust sizes only; boxed trees and opaque BTree heap nodes excluded"
        })
    );
}

#[test]
#[ignore = "explicit allocator diagnostics only; not timing or a smaller resource witness"]
fn observe_actual_public_leaf_binding_and_invocation_allocations() {
    metadata_sizes();
    // Confirm that the observer handles both a realloc and the release of a
    // preexisting allocation without attributing the latter to this window.
    let preexisting = vec![0u8; 37];
    let (retained, probe) = measured(|| {
        drop(preexisting);
        let mut bytes = Vec::<u8>::with_capacity(64);
        bytes.resize(64, 0);
        bytes.reserve_exact(512);
        bytes
    });
    assert!(probe.complete && probe.requests >= 2 && probe.peak_live_bytes >= retained.capacity());
    assert_eq!(probe.live_at_return, retained.capacity());
    drop(retained);

    let mut cases = vec![
        (
            "string-length".to_owned(),
            "+> sequence\nsequence/length(text<string>)\n".to_owned(),
            "sequence/length",
        ),
        (
            "string-concat".to_owned(),
            "left<string> + right<string>\n".to_owned(),
            "string/concat",
        ),
        (
            "string-gather".to_owned(),
            "text := input<string>\nindices := selectors<[u64]>\ntext[indices]\n".to_owned(),
            "access/range",
        ),
        (
            "matrix-gather".to_owned(),
            "values := input<[u64]>\nindices := selectors<[u64]>\nvalues[indices]\n".to_owned(),
            "access/range",
        ),
        (
            "tuple-pack".to_owned(),
            "(number<u64>, label<string>, flag<bool>, number)\n".to_owned(),
            "core/composite-pack",
        ),
    ];
    for entries in [2, 8, 64] {
        let fields = (0..entries)
            .map(|i| format!("\"key{i}\": base<u64> + {i}u64"))
            .collect::<Vec<_>>()
            .join(", ");
        cases.push((
            format!("map-pack-{}-ports", 2 * entries),
            format!("{{{fields}}}\n"),
            "core/composite-pack",
        ));
    }
    for (case, source, operation) in cases {
        for (ordinal, artifact) in artifacts(&source).into_iter().enumerate() {
            let catalog = mech_stdlib::source_catalog();
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let template = plan
                .blocks()
                .iter()
                .find_map(|block| match block.action() {
                    NativeIterationAction::LibraryCall { template, .. }
                        if template.primitive().name() == operation =>
                    {
                        Some(template)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing actual maintained {case}/{operation}"));
            let prepared =
                prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let arguments = args(&artifact, template, &memory);
            let parameters = arguments
                .iter()
                .map(|value| value.as_ref().unwrap().shape().parameter_values())
                .collect::<Vec<_>>();
            let output_parameters = vec![0; template.output().dimension_parameters().len()];
            // Exact existing finite qualification policy; no limit is raised
            // for large constructors. A denial is retained and reported.
            let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
                .unwrap()
                .limits;
            limits.max_compute_work = Some(134_217_728);
            limits.max_comparison_work = Some(1_000_000);
            limits.max_temporary_bytes = Some(128 * 1024 * 1024);
            let (call, observed) = measured(|| {
                bind_native_program_library_call(
                    &artifact,
                    template,
                    &catalog,
                    &parameters,
                    &output_parameters,
                    limits,
                )
            });
            report(
                &case,
                ordinal == 1,
                "public-bind-current-shape",
                &observed,
                (0, 0),
                call.is_ok(),
            );
            let call = match call {
                Ok(call) => call,
                Err(error) => {
                    eprintln!("native-diagnostic-denial {case}/public-bind: {error:?}");
                    drop(arguments);
                    assert_eq!(
                        memory.used_bytes(),
                        0,
                        "denied diagnostic bind leaked {case}"
                    );
                    continue;
                }
            };
            // These windows isolate the actual immutable/current-shape objects
            // cloned along bind/invoke. They are NOT summed into an admission
            // witness: independent requested-byte observations do not establish
            // coexistence, allocator growth, or every unobserved failure path.
            observe_metadata(&case, ordinal == 1, "clone-immutable-template", || {
                (*template).clone()
            });
            observe_metadata(&case, ordinal == 1, "clone-current-bound-call", || {
                call.bound_call().clone()
            });
            observe_metadata(
                &case,
                ordinal == 1,
                "clone-current-call-memory-plan",
                || call.memory_plan().clone(),
            );
            observe_metadata(
                &case,
                ordinal == 1,
                "clone-shared-schema-table-wrapper",
                || artifact.schemas().clone(),
            );
            let key = template.selected_key();
            let factory = catalog
                .resident_factory(&key.module_path, &key.operation_name)
                .unwrap()
                .factory();
            let request = mech_core::ResidentKernelBindRequest {
                contract: template.contract(),
                schemas: artifact.schemas(),
                inputs: call.inputs(),
                output: call.output().clone(),
            };
            // The exact maintained factory at the actual selected site. This
            // diagnostic only constructs/drops its unbound result: no execute,
            // synthetic semantic witness, or alternate leaf implementation.
            let (kernel, observed) = measured(|| factory(&request));
            report(
                &case,
                ordinal == 1,
                "actual-selected-factory-binding-only",
                &observed,
                (0, 0),
                kernel.is_ok(),
            );
            if let Err(error) = &kernel {
                eprintln!("native-diagnostic-denial {case}/factory-bind: {error:?}");
            }
            drop(kernel);
            let actual = call.memory_plan();
            let ports = actual.inputs.len() + actual.outputs.len();
            let descriptor_heap_bound = template
                .inputs()
                .iter()
                .chain(core::iter::once(template.output()))
                .map(|schema| schema.clone_allocation_bound_bytes().unwrap())
                .try_fold(0u64, |total, bytes| total.checked_add(bytes))
                .unwrap();
            eprintln!(
                "native-plan-diagnostic {}",
                serde_json::json!({
                    "case":case, "decoded":ordinal == 1, "primitive":format!("{:?}", template.primitive()),
                    "ports":ports, "allocations":actual.allocations.len(),
                    "deferred_witnesses":actual.deferred_witnesses.len(), "transactions":actual.transactions.len(),
                    "prepared_binding_bytes":prepared.binding_metadata_bytes(), "prepared_binding_work":prepared.binding_work(),
                    "schema_arena_retention_bound":artifact.schemas().clone_allocation_bound_bytes(),
                    "schema_wrapper_clone_bound":mech_core::SchemaTable::clone_metadata_allocation_bound_bytes(),
                    "port_schema_heap_clone_bound_sum":descriptor_heap_bound,
                    "existing_binding_model": {
                        "maximum_entries":3 * ports + 16,
                        "schema_copies":4 * ports + 4,
                        "structural_entry_envelope_bytes":2048,
                        "growth_and_coexistence_factor":16,
                        "work_per_reserved_binding_byte":8,
                        "fixed_binding_work":256,
                        "classification":"decomposition of existing policy, not measured allocation or a proposed smaller bound"
                    },
                })
            );
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let mut output = None;
            for phase in [
                "public-invoke-first-publication",
                "public-invoke-existing-publication",
            ] {
                let before = work.used();
                let (result, observed) = measured(|| call.invoke(&arguments, &mut output, &work));
                report(
                    &case,
                    ordinal == 1,
                    phase,
                    &observed,
                    work_delta(before, work.used()),
                    result.is_ok(),
                );
                if let Err(error) = result {
                    eprintln!("native-diagnostic-denial {case}/{phase}: {error:?}");
                    break;
                }
            }
            drop(output);
            // A separate logical invocation keeps this comparison independent
            // of the prior two prebound calls. Caps are IDENTICAL; each pair
            // still shares its cumulative account across first/replacement.
            let prepared_work =
                NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let mut prepared_output = None;
            for phase in [
                "public-prepared-first-bind-and-invoke-total",
                "public-prepared-replacement-bind-and-invoke-total",
            ] {
                let before = prepared_work.used();
                let (result, observed) = measured(|| {
                    prepared.invoke_current(
                        &arguments,
                        &mut prepared_output,
                        limits,
                        &prepared_work,
                    )
                });
                report(
                    &case,
                    ordinal == 1,
                    phase,
                    &observed,
                    work_delta(before, prepared_work.used()),
                    result.is_ok(),
                );
                if let Err(error) = result {
                    eprintln!("native-diagnostic-denial {case}/{phase}: {error:?}");
                    break;
                }
            }
            drop((arguments, prepared_output, work, prepared_work));
            assert_eq!(memory.used_bytes(), 0, "diagnostic did not release {case}");
        }
    }
}
