//! Compile the same checked numeric plan under an explicit entry contract.
use super::*;
use cfg_cleanup::composed_reads::{Admission, Capture, DenseTables, Literal, View};

pub(super) struct Composed {
    pub function: FuncId,
    pub admission: Admission,
    pub summary: serde_json::Value,
}

pub(super) struct Compiled {
    pub function: FuncId,
    pub entry_bounds: Vec<(usize, u64, bool)>,
    pub summary: serde_json::Value,
    pub code: Vec<u8>,
    pub composed: Option<Composed>,
}
pub(super) fn define<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    id: usize,
    observe: bool,
    input_words: usize,
    output_words: usize,
    fixed_count: usize,
    vectors: Option<&VectorContract>,
    private: Option<&PrivatePhase>,
    no_growth: Option<&NoGrowth>,
    literal_views: &BTreeMap<usize, Literal>,
    dense_tables: &mut DenseTables,
    suffix: &str,
) -> Result<Compiled, NativeCompileError> {
    let code = CodePlan::Program(prepared);
    let closed = &prepared.metadata().closed_tails;
    let region = &closed.regions[id];
    let header = region.tail.header() as usize;
    let proof = region
        .numeric
        .as_ref()
        .ok_or_else(|| backend("phase lacks checked numeric proof"))?;
    let controls = CompilationControls::new(OptimizationOptions::default());
    let report;
    let symbol = format!("mech_compact_numeric_{header}{suffix}");
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 3];
    signature.returns = vec![AbiParam::new(types::I64); 2];
    let mut context = module.make_context();
    context.func.signature = signature.clone();
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let begin = b.create_block();
        b.append_block_params_for_function_params(begin);
        b.switch_to_block(begin);
        let args = b.block_params(begin).to_vec();
        let zero = b.ins().iconst(types::I64, 0);
        let error = b.create_block();
        b.append_block_param(error, types::I64);
        b.set_cold_block(error);
        let done = b.create_block();
        let storage = Arrays::new(module, &mut b, args[0], observe)?;
        let native = numeric_minimal::Boundary::new(
            &mut b,
            args[1],
            args[2],
            region,
            Some(&storage),
            private,
            no_growth,
        )?;
        report = crate::native_artifact::program::codegen::numeric_compact::emit(
            &controls, &mut b, code, region, &native, vectors, private, no_growth, done, error,
        )?;
        b.switch_to_block(done);
        b.ins().return_(&[zero, zero]);
        b.switch_to_block(error);
        let status = b.block_params(error)[0];
        let at = b.ins().load(types::I64, MemFlags::new(), args[2], 0);
        b.ins().return_(&[status, at]);
        b.seal_all_blocks();
        b.finalize();
    }
    let function = module
        .declare_function(&symbol, Linkage::Local, &signature)
        .map_err(backend)?;
    eprintln!("compact-stage: numeric phase {symbol}");
    if let Err(error) = context.verify(module.isa()) {
        if let Some(path) = std::env::var_os("MECH_COMPACT_FAILURE_DIR") {
            let root = std::path::PathBuf::from(path);
            std::fs::create_dir_all(&root).map_err(backend)?;
            std::fs::write(
                root.join(format!("{symbol}.clif")),
                context.func.display().to_string(),
            )
            .map_err(backend)?;
            std::fs::write(root.join(format!("{symbol}.proof")), format!("{proof:#?}"))
                .map_err(backend)?;
        }
        return Err(backend(format!("{symbol}: {error:?}")));
    }
    // Numeric phases have their own sealed SSA; never match through packets.
    let mut bit_counts =
        crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
    let cleanup = cfg_cleanup::optimize(&controls, &mut context, module.isa())?;
    let word_loops = &report;
    // Resolve literal admission flags before allocating the already selected
    // capacity-proved reduction. Keep other native routines on the usual pass.
    if no_growth.is_some()
        && !word_loops.reduction_completions.is_empty()
        && cfg_cleanup::fold_bitwise_constants(&controls, &mut context.func) != 0
    {
        cfg_cleanup::optimize(&controls, &mut context, module.isa())?;
    }
    let equality = crate::native_artifact::program::codegen::wide_equal::optimize(
        &mut context,
        &word_loops.compact_word_loops,
        private.is_some()
            && vectors.is_some()
            && std::env::var_os("MECH_REUSE_EQUALITY_PREFIX").is_some(),
    );
    // Named experimental condition: growth state is absent from this callee.
    let threaded = if no_growth.is_some()
        && std::env::var_os("MECH_THREAD_AFTER_GROWTH_ISOLATION").is_some()
    {
        Some(cfg_cleanup::thread_loop_edges(
            &controls,
            &mut context,
            module.isa(),
        )?)
    } else {
        None
    };
    let pack_sites = word_loops.read_pack_sites.clone();
    let read_guards = word_loops.proven_scalar_read_guards.clone();
    let write_guards = word_loops.proven_scalar_write_guards.clone();
    let reused_reads = word_loops.reused_scalar_reads.clone();
    let reused_operands = word_loops.reused_scalar_operands.clone();
    if no_growth.is_some() && std::env::var_os("MECH_CHECKED_APPEND_RUN").is_some() {
        let outcome = crate::native_artifact::program::codegen::append_run::optimize(
            &mut context,
            word_loops.machine_header,
            std::env::var_os("MECH_SPLIT_APPEND_DESCRIPTORS")
                .map(|value| (input_words, value == "all")),
        );
        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} checked append run: {outcome:?}");
        }
    }
    if no_growth.is_some() && std::env::var_os("MECH_NARROW_MASKED_ARITHMETIC").is_some() {
        let narrowed =
            crate::native_artifact::program::codegen::low_word::narrow(&mut context.func);
        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} low-word operations: {narrowed}");
        }
    }
    // A completed ordered reduction names its own selected continuation.
    // Only those edges enter the existing acyclic forwarding proof.
    if no_growth.is_some() && !word_loops.reduction_completions.is_empty() {
        let report = cfg_cleanup::forward_identity_routes_from(
            &controls,
            &mut context,
            module.isa(),
            Some(&word_loops.reduction_completions),
        )?;
        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!(
                "{symbol} completed reduction routes: {}",
                serde_json::to_string(&report).unwrap()
            );
        }
    }
    let vector_predicates = if private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_PRIVATE_VECTOR_PREDICATES").is_some()
    {
        let report = cfg_cleanup::refine_current_predicates(&controls, &mut context, module.isa())?;
        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() && report.removed_branches != 0 {
            eprintln!(
                "{symbol} selected-edge predicates: {} branches",
                report.removed_branches
            );
        }
        Some(report)
    } else {
        None
    };
    let width_guards = if private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_PRIVATE_VECTOR_WIDTHS").is_some()
    {
        let removed = cfg_cleanup::fold_proved_widths(&controls, &mut context, module.isa())?;
        if removed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} proved-width guards: {removed}");
        }
        removed
    } else {
        0
    };
    let window_bounds = if private.is_some() && vectors.is_some() {
        cfg_cleanup::combine_window_bounds(&controls, &mut context, module.isa())?
    } else {
        0
    };
    let paired_spans = if private.is_some()
        && vectors.is_some()
        && !equality.reused.is_empty()
        && std::env::var_os("MECH_ORDERED_CAPTURE_SPANS").is_some()
    {
        cfg_cleanup::fold_ordered_capture_spans(&controls, &mut context, module.isa())?
    } else {
        0
    };
    if paired_spans != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
        eprintln!("{symbol} ordered capture spans: {paired_spans}");
    }
    if private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_ADJACENT_ADDRESSES").is_some()
    {
        let changed = cfg_cleanup::reuse_adjacent_addresses(&controls, &mut context, module.isa())?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} adjacent typed accesses: {changed}");
        }
    }
    if std::env::var_os("MECH_SHARE_REPEATED_BOUNDS").is_some() {
        let changed = cfg_cleanup::share_repeated_bounds(&controls, &mut context, module.isa())?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} repeated bounds checks: {changed}");
        }
    }
    if private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_DEFER_EXIT_PACKET_WORDS").is_some()
    {
        let changed = super::phase_storage::sink_exit_packet_words(
            &mut context.func,
            input_words,
            output_words,
        );
        if changed != 0 {
            context.verify(module.isa()).map_err(backend)?;
            if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                eprintln!("{symbol} deferred exit packet words: {changed}");
            }
        }
    }
    if private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_REUSE_DOMINATING_WINDOW_FACTS").is_some()
    {
        let changed =
            cfg_cleanup::reuse_dominating_window_facts(&controls, &mut context, module.isa())?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} reused dominating window facts: {changed}");
        }
    }
    if private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_PROVE_SCALAR_SPANS").is_some()
    {
        let changed = cfg_cleanup::prove_scalar_spans(&controls, &mut context, module.isa())?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} proved scalar spans: {changed}");
        }
    }
    if private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_REUSE_SELECTED_READ_BOUND").is_some()
    {
        let changed =
            cfg_cleanup::reuse_selected_read_bounds(&controls, &mut context, module.isa())?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} reused selected readable bounds: {changed}");
        }
    }
    // Late forwarding may expose a complete idiom without crossing a phase.
    bit_counts += crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
    let mut composed = None;
    if let Some(contract) = no_growth
        && !literal_views.is_empty()
    {
        let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
        let word_at = |word| packet.iter().position(|&at| at == word);
        let views = proof
            .arrays
            .iter()
            .enumerate()
            .filter_map(|(array, definition)| {
                if definition.writable || definition.appends {
                    return None;
                }
                let pointer_word = word_at(proof.view(array))?;
                let length_word = word_at(proof.view(array) + 1)?;
                Some(View {
                    array,
                    pointer_word,
                    length_word,
                    width: definition.width,
                    literal: literal_views.get(&array).cloned(),
                })
            })
            .collect::<Vec<_>>();
        if let Some(countdown_word) = word_at(contract.countdown)
            && let Some(loads) = phase_storage::input_word_loads(&context.func, input_words)
        {
            let mut candidate = module.make_context();
            candidate.func = context.func.clone();
            let capture = Capture {
                packet_loads: &loads,
                views: &views,
                header: word_loops.machine_header,
                countdown_word,
            };
            if let Some(admission) = cfg_cleanup::compose_readonly_lookups(
                &controls,
                &mut candidate,
                module,
                dense_tables,
                &capture,
            )? {
                let name = format!("{symbol}_composed");
                let id = module
                    .declare_function(&name, Linkage::Local, &signature)
                    .map_err(backend)?;
                let inspect_path = std::env::var_os("MECH_INSPECT_PHASE_CLIF")
                    .map(std::path::PathBuf::from)
                    .filter(|path| path.file_stem().is_some_and(|stem| stem == name.as_str()));
                if let Some(path) = &inspect_path {
                    std::fs::write(path, candidate.func.display().to_string()).map_err(backend)?;
                    candidate.set_disasm(true);
                }
                module
                    .define_function(id, &mut candidate)
                    .map_err(backend)?;
                let machine = candidate
                    .compiled_code()
                    .ok_or_else(|| backend("composed phase lacks code"))?;
                if let Some(path) = inspect_path {
                    let vcode = machine
                        .vcode
                        .as_ref()
                        .ok_or_else(|| backend("composed VCode was not emitted"))?;
                    std::fs::write(path.with_extension("vcode"), vcode).map_err(backend)?;
                }
                composed = Some(Composed {
                    function: id,
                    admission,
                    summary: serde_json::json!({"symbol":name,"admission":admission,
                        "code_bytes":machine.code_info().total_size,
                        "stack_bytes":compiled_stack_bytes(&machine.buffer)?,
                        "scratch_bytes":if admission.immutable_code_bytes.is_some() { 0 } else { admission.cells_per_bank * 8 }}),
                });
            }
        }
    }
    if let Some(path) = std::env::var_os("MECH_INSPECT_PHASE_CLIF") {
        let path = std::path::PathBuf::from(path);
        if path.file_stem().is_some_and(|name| name == symbol.as_str()) {
            std::fs::write(&path, context.func.display().to_string()).map_err(backend)?;
            std::fs::write(path.with_extension("proof.txt"), format!("{proof:#?}"))
                .map_err(backend)?;
        }
    }
    let vcode_path = std::env::var_os("MECH_INSPECT_PHASE_VCODE")
        .map(std::path::PathBuf::from)
        .filter(|path| path.file_stem().is_some_and(|name| name == symbol.as_str()));
    if vcode_path.is_some() {
        context.set_disasm(true);
    }
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or_else(|| backend("compact numeric phase lacks code"))?;
    if let Some(path) = vcode_path {
        let text = compiled
            .vcode
            .as_ref()
            .ok_or_else(|| backend("phase VCode diagnostic was not emitted"))?;
        std::fs::write(path, text).map_err(backend)?;
    }
    let entry_bounds = report
        .phase_entry_bounds
        .iter()
        .map(|&(array, minimum)| (array, minimum, proof.arrays[array].appends))
        .collect::<Vec<_>>();
    let changed_writes = report.proven_changed_writes.clone();
    let private_writes = report.private_write_sites.clone();
    let mut summary = serde_json::json!({"read_pack_sites":pack_sites,"wider_equal_loops":equality.wider,"private_write_sites":private_writes,"proven_changed_writes":changed_writes,"phase_entry_bounds":entry_bounds,"header":header,"exit":region.tail.exit(),"symbol":symbol,
        "code_bytes":compiled.code_info().total_size,"stack_bytes":compiled_stack_bytes(&compiled.buffer)?,
        "input_words":input_words,"output_words":output_words,
        "fixed_writable_capacities":fixed_count,"fixed_length_entry_gate":fixed_count != 0,
        "actual_unit_append_growth":no_growth.is_none(),"unique_writable_owner_gate":true,"cleanup":cleanup});
    summary["bit_counts"] = serde_json::json!(bit_counts);
    if let Some(report) = vector_predicates.filter(|report| report.removed_branches != 0) {
        summary["private_vector_predicates"] = serde_json::json!(report);
    }
    if width_guards != 0 {
        summary["proved_width_guards"] = serde_json::json!(width_guards);
    }
    if window_bounds != 0 {
        summary["shared_window_bounds"] = serde_json::json!(window_bounds);
    }
    if !equality.reused.is_empty() {
        summary["reused_shifted_equality_loops"] = serde_json::json!(equality.reused);
    }
    if !equality.prefixes.is_empty() {
        summary["forwarded_equality_ends"] = serde_json::json!(equality.prefixes);
    }
    if let Some(threaded) = threaded {
        summary["threaded_after_growth_isolation"] = serde_json::json!(threaded);
    }
    if !read_guards.is_empty() {
        summary["proven_scalar_read_guards"] = serde_json::json!(read_guards);
    }
    if !write_guards.is_empty() {
        summary["proven_scalar_write_guards"] = serde_json::json!(write_guards);
    }
    if !reused_reads.is_empty() {
        summary["reused_scalar_reads"] = serde_json::json!(reused_reads);
    }
    if !reused_operands.is_empty() {
        summary["reused_scalar_operands"] = serde_json::json!(reused_operands);
    }

    Ok(Compiled {
        function,
        entry_bounds,
        summary,
        code: compiled.buffer.data().to_vec(),
        composed,
    })
}
