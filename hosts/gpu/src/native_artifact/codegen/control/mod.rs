//! One phased canonical CFG definition for JIT and Object emission.
use crate::native_artifact::abi::{NO_NODE, NativeEntryFrame as Frame, STATUS_ABI};
use crate::native_artifact::compiler::{
    admit, backend, compiled_stack_bytes, frame_load, guard, host_supported, limit, offset,
    set_origin_with_flags,
};
use crate::native_artifact::control_compiler::{emit_scalar_action, emit_scalar_terminator};
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::{NativeCompileError, NativeCompileLimits};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::{
    NativeIterationAction, NativeIterationTerminator, NativeMatchAction, NativeMatchTerminator,
    NativeMatchValue,
};
fn successors(term: NativeIterationTerminator) -> [Option<u32>; 2] {
    match term {
        NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) => [Some(next), None],
        NativeIterationTerminator::Scalar(
            NativeMatchTerminator::Branch { yes, no, .. }
            | NativeMatchTerminator::LiteralBranch { yes, no, .. },
        ) => [Some(yes), Some(no)],
        NativeIterationTerminator::Header { body, exit, .. } => [Some(body), Some(exit)],
        NativeIterationTerminator::TailHeader { body, .. } => [Some(body), None],
        _ => [None, None],
    }
}
fn load(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    value: NativeMatchValue,
) -> Result<Value, NativeCompileError> {
    crate::native_artifact::control_compiler::load(b, locals, value)
}

#[cfg(feature = "canonical-native-program")]
#[path = "report/mod.rs"]
pub(in crate::native_artifact::codegen::control) mod codegen_report;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) use crate::native_artifact::codegen::control::codegen_report::{
    BoundCallShim, CfgCleanup, CodegenReport, ColdSnapshot, CountValueLoop, InvariantLoop,
    MutationLoop, OutlinedLoop, PreservingCallRegion, RegionToken, ScalarLoop, SteadyLoop,
};

#[cfg(not(feature = "canonical-native-program"))]
pub(in crate::native_artifact) struct CodegenReport;

pub(in crate::native_artifact) fn preflight_counts(
    blocks: usize,
    inputs: usize,
    outputs: usize,
    values: usize,
    limits: NativeCompileLimits,
) -> Result<(), NativeCompileError> {
    limit("Iteration blocks", blocks as u64, limits.max_nodes)?;
    limit("Iteration inputs", inputs as u64, limits.max_inputs)?;
    limit("Iteration outputs", outputs as u64, limits.max_outputs)?;
    limit(
        "Iteration scalar values",
        values as u64,
        limits.max_nodes.saturating_mul(8),
    )?;
    Ok(())
}

pub(in crate::native_artifact) fn define_cfg<M: Module>(
    plan: crate::native_artifact::codegen::control::plan::CodePlan<'_>,
    entry_symbol: &str,
    version: u64,
    module: &mut M,
    limits: NativeCompileLimits,
    options: &crate::native_artifact::codegen::options::OptimizationOptions,
) -> Result<(FuncId, u64, u64, CodegenReport), NativeCompileError> {
    let origin_flags = if plan.is_program() {
        crate::native_artifact::codegen::common::memory_regions::origin(*options)
    } else {
        MemFlags::new()
    };
    host_supported()?;
    if module.target_config().pointer_type() != types::I64 {
        return Err(NativeCompileError::UnsupportedHost);
    }
    preflight_counts(
        plan.blocks().len(),
        plan.input_count(),
        plan.output_count(),
        plan.value_count(),
        limits,
    )?;
    #[cfg(feature = "canonical-native-program")]
    let empty_aggregates = Default::default();
    #[cfg(feature = "canonical-native-program")]
    let empty_parallel = Default::default();
    #[cfg(feature = "canonical-native-program")]
    let empty_borrowed = Default::default();
    #[cfg(feature = "canonical-native-program")]
    let empty_closed = Default::default();
    #[cfg(feature = "canonical-native-program")]
    let empty_prefix = None;
    #[cfg(feature = "canonical-native-program")]
    let empty_admission = Default::default();
    #[cfg(feature = "canonical-native-program")]
    let empty_owner = None;
    #[cfg(feature = "canonical-native-program")]
    let empty_frozen = Default::default();
    #[cfg(feature = "canonical-native-program")]
    let (aggregates, parallel, borrowed, closed, prefix, map_admission, owner, frozen) = match plan
    {
        crate::native_artifact::codegen::control::plan::CodePlan::Program(prepared) => {
            let m = prepared.metadata();
            (
                &m.scalar_aggregates,
                &m.parallel,
                &m.borrowed_strings,
                &m.closed_tails,
                &m.prefix,
                &m.map_admission,
                &m.map_owner_words,
                &m.frozen_maps,
            )
        }
        _ => (
            &empty_aggregates,
            &empty_parallel,
            &empty_borrowed,
            &empty_closed,
            &empty_prefix,
            &empty_admission,
            &empty_owner,
            &empty_frozen,
        ),
    };
    let local_count = match plan {
        crate::native_artifact::codegen::control::plan::CodePlan::Scalar(p) => p
            .values()
            .len()
            .checked_add(p.iterations().len())
            .ok_or_else(|| backend("iteration local count overflow"))?,
        #[cfg(feature = "canonical-native-program")]
        crate::native_artifact::codegen::control::plan::CodePlan::Program(p) => {
            p.metadata().local_words().map_err(backend)?
        }
    };
    offset(local_count)?;
    offset(plan.input_count())?;
    offset(plan.output_count())?;
    plan.validate()?;
    let mut definition = CfgDefinition {
        plan,
        #[cfg(feature = "canonical-native-program")]
        options,
        origin_flags,
        local_count,
        #[cfg(feature = "canonical-native-program")]
        aggregates,
        #[cfg(feature = "canonical-native-program")]
        parallel,
        #[cfg(feature = "canonical-native-program")]
        borrowed,
        #[cfg(feature = "canonical-native-program")]
        closed,
        #[cfg(feature = "canonical-native-program")]
        prefix,
        #[cfg(feature = "canonical-native-program")]
        map_admission,
        #[cfg(feature = "canonical-native-program")]
        owner,
        #[cfg(feature = "canonical-native-program")]
        frozen,
        #[cfg(feature = "canonical-native-program")]
        controls: crate::native_artifact::codegen::options::CompilationControls::new(*options),
        #[cfg(feature = "canonical-native-program")]
        report: CodegenReport::default(),
    };
    let declarations = definition.declare(module, entry_symbol, limits)?;
    let mut context = module.make_context();
    context.func.signature = declarations.entry_signature.clone();
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let prologue = definition.prologue(&mut b, module, &declarations, version)?;
        definition.actions(&mut b, module, &declarations, prologue)?;
        b.finalize();
    }
    definition.measure(module, context, declarations, limits)
}

struct CfgDefinition<'plan> {
    plan: crate::native_artifact::codegen::control::plan::CodePlan<'plan>,
    #[cfg(feature = "canonical-native-program")]
    options: &'plan crate::native_artifact::codegen::options::OptimizationOptions,
    origin_flags: MemFlags,
    local_count: usize,
    #[cfg(feature = "canonical-native-program")]
    controls: crate::native_artifact::codegen::options::CompilationControls,
    #[cfg(feature = "canonical-native-program")]
    report: CodegenReport,
    #[cfg(feature = "canonical-native-program")]
    aggregates: &'plan mech_engine::native_artifact::NativeScalarAggregates,
    #[cfg(feature = "canonical-native-program")]
    parallel: &'plan crate::native_artifact::program::prepare::parallel::Layout,
    #[cfg(feature = "canonical-native-program")]
    borrowed: &'plan crate::native_artifact::program::prepare::borrowed::Layout,
    #[cfg(feature = "canonical-native-program")]
    closed: &'plan crate::native_artifact::program::prepare::tail::Layout,
    #[cfg(feature = "canonical-native-program")]
    prefix: &'plan Option<crate::native_artifact::program::prepare::prefix::Region>,
    #[cfg(feature = "canonical-native-program")]
    map_admission: &'plan crate::native_artifact::program::prepare::map_admission::Layout,
    #[cfg(feature = "canonical-native-program")]
    owner: &'plan Option<mech_engine::native_artifact::NativeMapOwnerWords>,
    #[cfg(feature = "canonical-native-program")]
    frozen: &'plan crate::native_artifact::program::prepare::frozen_map::Layout,
}
struct CfgDeclarations<'plan> {
    function: FuncId,
    entry_signature: cranelift_codegen::ir::Signature,
    callback_signature: cranelift_codegen::ir::Signature,
    // Preserve a plan lifetime in the scalar-only feature profile as well.
    _plan: core::marker::PhantomData<&'plan ()>,
    #[cfg(feature = "canonical-native-program")]
    window_functions: Vec<(
        &'plan mech_engine::native_artifact::NativeWindowRegion,
        FuncId,
        u64,
        u64,
    )>,
    #[cfg(feature = "canonical-native-program")]
    map_functions: Vec<(
        &'plan mech_engine::native_artifact::NativeStringMapRegion,
        FuncId,
        u64,
        u64,
    )>,
    #[cfg(feature = "canonical-native-program")]
    joined_functions: Vec<(u32, FuncId, u64, u64)>,
    #[cfg(feature = "canonical-native-program")]
    joined_maps: Vec<(u32, FuncId)>,
    #[cfg(feature = "canonical-native-program")]
    packed_functions: Vec<(usize, FuncId, u64, u64)>,
    #[cfg(feature = "canonical-native-program")]
    bound_call_shim: Option<crate::native_artifact::program::codegen::bound_shim::Definition>,
}
struct CfgPrologue {
    frame: Value,
    error: cranelift_codegen::ir::Block,
    locals: Value,
    locals_home: Option<cranelift_codegen::ir::StackSlot>,
    outputs: Value,
    user_context: Value,
    callback: Value,
    callback_signature: cranelift_codegen::ir::SigRef,
    hooks: Option<crate::native_artifact::program::codegen::hooks::Hooks>,
    program_phase: Option<Value>,
    resume_block: Option<cranelift_codegen::ir::Block>,
    #[cfg(feature = "canonical-native-program")]
    parallel_entry: Option<Value>,
    #[cfg(feature = "canonical-native-program")]
    prefix_facts: Option<crate::native_artifact::program::codegen::prefix::EntryFacts>,
}
impl<'plan> CfgDefinition<'plan> {
    fn declare<M: Module>(
        &mut self,
        module: &mut M,
        entry_symbol: &str,
        limits: NativeCompileLimits,
    ) -> Result<CfgDeclarations<'plan>, NativeCompileError> {
        #[cfg(feature = "canonical-native-program")]
        let plan = self.plan;
        #[cfg(not(feature = "canonical-native-program"))]
        let _ = limits;
        #[cfg(feature = "canonical-native-program")]
        let controls = &self.controls;
        #[cfg(feature = "canonical-native-program")]
        let closed = self.closed;
        #[cfg(feature = "canonical-native-program")]
        let owner = self.owner;
        #[cfg(feature = "canonical-native-program")]
        let aggregates = self.aggregates;
        #[cfg(feature = "canonical-native-program")]
        let report = &mut self.report;
        #[cfg(feature = "canonical-native-program")]
        let window_functions =
            if let crate::native_artifact::codegen::control::plan::CodePlan::Program(program) = plan
            {
                program
                    .metadata()
                    .window_regions
                    .iter()
                    .map(|region| {
                        let (function, _, stack, code) =
                            crate::native_artifact::program::codegen::window::define(
                                module, &region,
                            )?;
                        Ok((region, function, stack, code))
                    })
                    .collect::<Result<Vec<_>, NativeCompileError>>()?
            } else {
                Vec::new()
            };
        #[cfg(feature = "canonical-native-program")]
        let map_functions =
            if let crate::native_artifact::codegen::control::plan::CodePlan::Program(program) = plan
            {
                program
                    .metadata()
                    .string_map_regions
                    .iter()
                    .map(|region| {
                        let (function, stack, code) =
                            crate::native_artifact::program::codegen::string_map::define(
                                module, &region,
                            )?;
                        Ok((region, function, stack, code))
                    })
                    .collect::<Result<Vec<_>, NativeCompileError>>()?
            } else {
                Vec::new()
            };
        #[cfg(feature = "canonical-native-program")]
        let joined_functions =
            if let crate::native_artifact::codegen::control::plan::CodePlan::Program(program) = plan
            {
                let layout = &program.metadata().selected_joins;
                map_functions
                .iter()
                .filter(|(map, ..)| {
                    layout
                        .maps
                        .iter()
                        .flatten()
                        .any(|proof| proof.collection == map.collection())
                })
                .map(|(region, ..)| {
                    let (function, stack, code) =
                        crate::native_artifact::program::codegen::join_map::define(module, region)?;
                    report.join_map_copies.push(codegen_report::JoinMapCopy {
                        collection: region.collection(),
                        symbol: crate::native_artifact::program::codegen::join_map::symbol(
                            region.collection(),
                        ),
                        stack_bytes: stack,
                        code_bytes: code,
                        geometry_words: 6,
                        initialized_extent_word: crate::native_artifact::program::protocol::abi::JOIN_MAP_INITIALIZED_WORD,
                    });
                    Ok((region.collection(), function, stack, code))
                })
                .collect::<Result<Vec<_>, NativeCompileError>>()?
            } else {
                Vec::new()
            };
        #[cfg(feature = "canonical-native-program")]
        let joined_maps = if let crate::native_artifact::codegen::control::plan::CodePlan::Program(
            program,
        ) = plan
        {
            let layout = &program.metadata().selected_joins;
            layout
                .regions
                .iter()
                .zip(layout.maps.iter())
                .filter_map(|(join, proof)| {
                    proof.as_ref().map(|proof| {
                        let function = joined_functions
                            .iter()
                            .find(|(collection, ..)| *collection == proof.collection)
                            .map(|(_, function, ..)| *function)
                            .ok_or_else(|| backend("joined map has no emitted String mapper"))?;
                        Ok((join.consumer(), function))
                    })
                })
                .collect::<Result<Vec<_>, NativeCompileError>>()?
        } else {
            Vec::new()
        };
        #[cfg(feature = "canonical-native-program")]
        let (packed_functions, bound_call_shim) =
            crate::native_artifact::program::codegen::packed_loop::define_functions(
                &controls,
                module,
                entry_symbol,
                plan,
                &closed,
                owner.as_ref(),
                &aggregates,
                limits,
                report,
            )?;
        let mut signature_entry = module.make_signature();
        signature_entry.params.push(AbiParam::new(types::I64));
        signature_entry.returns.push(AbiParam::new(types::I64));
        let function = module
            .declare_function(entry_symbol, Linkage::Export, &signature_entry)
            .map_err(backend)?;
        let mut callback_signature = module.make_signature();
        callback_signature.params = vec![AbiParam::new(types::I64); 2];
        callback_signature.returns.push(AbiParam::new(types::I64));
        Ok(CfgDeclarations {
            function,
            entry_signature: signature_entry,
            callback_signature,
            _plan: core::marker::PhantomData,
            #[cfg(feature = "canonical-native-program")]
            window_functions,
            #[cfg(feature = "canonical-native-program")]
            map_functions,
            #[cfg(feature = "canonical-native-program")]
            joined_functions,
            #[cfg(feature = "canonical-native-program")]
            joined_maps,
            #[cfg(feature = "canonical-native-program")]
            packed_functions,
            #[cfg(feature = "canonical-native-program")]
            bound_call_shim,
        })
    }
    fn prologue<M: Module>(
        &self,
        b: &mut FunctionBuilder<'_>,
        module: &mut M,
        declarations: &CfgDeclarations<'plan>,
        version: u64,
    ) -> Result<CfgPrologue, NativeCompileError> {
        let plan = self.plan;
        let origin_flags = self.origin_flags;
        let local_count = self.local_count;
        #[cfg(feature = "canonical-native-program")]
        let parallel = self.parallel;
        #[cfg(feature = "canonical-native-program")]
        let prefix = self.prefix;
        #[cfg(feature = "canonical-native-program")]
        let aggregates = self.aggregates;
        #[cfg(feature = "canonical-native-program")]
        let function = declarations.function;
        let callback_signature = declarations.callback_signature.clone();
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let frame = b.block_params(entry)[0];
        let error = b.create_block();
        b.append_block_param(error, types::I64);
        b.set_cold_block(error);
        let nonnull = b.ins().icmp_imm(IntCC::NotEqual, frame, 0);
        guard(b, nonnull, error, STATUS_ABI);
        set_origin_with_flags(b, frame, NO_NODE, origin_flags);
        for (field, value) in [
            (core::mem::offset_of!(Frame, version), version),
            (
                core::mem::offset_of!(Frame, input_count),
                plan.input_count() as u64,
            ),
            (
                core::mem::offset_of!(Frame, local_count),
                local_count as u64,
            ),
            (
                core::mem::offset_of!(Frame, output_count),
                plan.output_count() as u64,
            ),
        ] {
            let actual = frame_load(b, frame, field);
            let valid = b.ins().icmp_imm(IntCC::Equal, actual, value as i64);
            guard(b, valid, error, STATUS_ABI);
        }
        let inputs = frame_load(b, frame, core::mem::offset_of!(Frame, inputs));
        let locals = frame_load(b, frame, core::mem::offset_of!(Frame, locals));
        let outputs = frame_load(b, frame, core::mem::offset_of!(Frame, outputs));
        let user_context = frame_load(b, frame, core::mem::offset_of!(Frame, context));
        let callback = frame_load(b, frame, core::mem::offset_of!(Frame, admit_next));
        for (pointer, count) in [
            (inputs, plan.input_count()),
            (locals, local_count),
            (outputs, plan.output_count()),
            (user_context, 1),
            (callback, 1),
        ] {
            if count > 0 {
                let valid = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
                guard(b, valid, error, STATUS_ABI);
            }
        }
        // Keep the checked entry pointer in this invocation's own stack frame.
        // Reload it at source-block boundaries instead of asking register
        // allocation to retain one SSA value throughout the entire program.
        // This is a snapshot, not a fresh read of mutable callback-visible ABI
        // memory; neither callbacks nor nested entries can replace its value.
        let locals_home = if plan.is_program() {
            use cranelift_codegen::ir::{StackSlotData, StackSlotKind};
            let slot = b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                3,
            ));
            b.ins().stack_store(locals, slot, 0);
            Some(slot)
        } else {
            None
        };
        let hooks = crate::native_artifact::program::codegen::hooks::Hooks::load(
            plan.is_program(),
            b,
            module,
            frame,
            user_context,
            error,
        )?;
        #[cfg(feature = "canonical-native-program")]
        let parallel_entry = if plan.is_program() {
            let reference = module.declare_func_in_func(function, b.func);
            Some(b.ins().func_addr(types::I64, reference))
        } else {
            None
        };
        #[cfg(feature = "canonical-native-program")]
        let program_phase = if plan.is_program() {
            let phase = frame_load(
                b,
                frame,
                core::mem::offset_of!(crate::native_artifact::program::protocol::abi::Frame, phase),
            );
            let ordinary = b.ins().icmp_imm(
                IntCC::UnsignedLessThanOrEqual,
                phase,
                if prefix.is_some() {
                    crate::native_artifact::program::protocol::abi::PHASE_CAPTURE_TAIL
                } else {
                    crate::native_artifact::program::protocol::abi::PHASE_TURN
                } as i64,
            );
            let valid = if parallel.regions.is_empty() {
                ordinary
            } else {
                let worker = b.ins().icmp_imm(
                    IntCC::Equal,
                    phase,
                    crate::native_artifact::program::protocol::abi::PHASE_WORKER as i64,
                );
                b.ins().bor(ordinary, worker)
            };
            guard(b, valid, error, STATUS_ABI);
            Some(phase)
        } else {
            None
        };
        #[cfg(not(feature = "canonical-native-program"))]
        let program_phase: Option<cranelift_codegen::ir::Value> = None;
        #[cfg(feature = "canonical-native-program")]
        let prefix_facts = prefix.as_ref().map(|_| {
            crate::native_artifact::program::codegen::prefix::EntryFacts::load(
                b,
                frame,
                program_phase.expect("managed prefix phase"),
            )
        });
        let resume_block = if let Some(phase) = program_phase {
            let initialize = b.create_block();
            let resume = b.create_block();
            let is_resume = b.ins().icmp_imm(IntCC::NotEqual, phase, 0);
            b.ins().brif(is_resume, resume, &[], initialize, &[]);
            b.switch_to_block(initialize);
            Some(resume)
        } else {
            None
        };
        for index in 0..plan.input_count() {
            // Managed sessions import all fresh inputs directly into their
            // certified scalar lanes AFTER activation/capture. ABI3 retains
            // its original initialization below.
            if plan.is_program() {
                continue;
            }
            let value = plan.input(index);
            if plan.kind(value).is_none() {
                continue;
            }
            let word = b
                .ins()
                .load(types::I64, MemFlags::new(), inputs, offset(index)?);
            if let Some(maximum) = plan
                .kind(value)
                .and_then(crate::native_artifact::compiler::word_upper_bound)
            {
                let valid = b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, word, maximum as i64);
                guard(b, valid, error, STATUS_ABI);
            }
            b.ins()
                .store(MemFlags::new(), word, locals, offset(value.get() as usize)?);
        }
        for index in 0..plan.constant_count() {
            let (slot, word) = plan.constant(index)?;
            if let Some(word) = word {
                let value = b.ins().iconst(types::I64, word as i64);
                b.ins()
                    .store(MemFlags::new(), value, locals, offset(slot.get() as usize)?);
            }
        }
        let callback_signature = b.import_signature(callback_signature);
        #[cfg(feature = "canonical-native-program")]
        crate::native_artifact::codegen::common::aggregates::constants(b, locals, &aggregates)?;
        Ok(CfgPrologue {
            frame,
            error,
            locals,
            locals_home,
            outputs,
            user_context,
            callback,
            callback_signature,
            hooks,
            program_phase,
            resume_block,
            #[cfg(feature = "canonical-native-program")]
            parallel_entry,
            #[cfg(feature = "canonical-native-program")]
            prefix_facts,
        })
    }
    fn actions<M: Module>(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        module: &mut M,
        declarations: &CfgDeclarations<'plan>,
        prologue: CfgPrologue,
    ) -> Result<(), NativeCompileError> {
        let plan = self.plan;
        #[cfg(feature = "canonical-native-program")]
        let options = self.options;
        #[cfg(not(feature = "canonical-native-program"))]
        let _ = module;
        let origin_flags = self.origin_flags;
        #[cfg(feature = "canonical-native-program")]
        let controls = &self.controls;
        #[cfg(feature = "canonical-native-program")]
        let closed = self.closed;
        #[cfg(feature = "canonical-native-program")]
        let owner = self.owner;
        #[cfg(feature = "canonical-native-program")]
        let aggregates = self.aggregates;
        #[cfg(feature = "canonical-native-program")]
        let borrowed = self.borrowed;
        #[cfg(feature = "canonical-native-program")]
        let prefix = self.prefix;
        #[cfg(feature = "canonical-native-program")]
        let map_admission = self.map_admission;
        #[cfg(feature = "canonical-native-program")]
        let parallel = self.parallel;
        #[cfg(feature = "canonical-native-program")]
        let frozen = self.frozen;
        #[cfg(feature = "canonical-native-program")]
        let report = &mut self.report;
        let CfgPrologue {
            frame,
            error,
            locals,
            locals_home,
            outputs,
            user_context,
            callback,
            callback_signature,
            hooks,
            program_phase,
            resume_block,
            #[cfg(feature = "canonical-native-program")]
            parallel_entry,
            #[cfg(feature = "canonical-native-program")]
            prefix_facts,
        } = prologue;
        #[cfg(feature = "canonical-native-program")]
        let window_functions = &declarations.window_functions;
        #[cfg(feature = "canonical-native-program")]
        let map_functions = &declarations.map_functions;
        #[cfg(feature = "canonical-native-program")]
        let joined_maps = &declarations.joined_maps;
        #[cfg(feature = "canonical-native-program")]
        let packed_functions = &declarations.packed_functions;
        #[cfg(not(feature = "canonical-native-program"))]
        let _ = declarations;
        let mut reachable = vec![false; plan.blocks().len()];
        let mut pending = vec![plan.entry() as usize];
        while let Some(id) = pending.pop() {
            if reachable[id] {
                continue;
            }
            reachable[id] = true;
            pending.extend(
                successors(plan.blocks()[id].terminator())
                    .into_iter()
                    .flatten()
                    .map(|next| next as usize),
            );
        }
        let blocks = reachable
            .iter()
            .map(|&used| used.then(|| b.create_block()))
            .collect::<Vec<_>>();
        b.ins()
            .jump(blocks[plan.entry() as usize].expect("reachable entry"), &[]);
        if let Some(resume) = resume_block {
            b.switch_to_block(resume);
            #[cfg(feature = "canonical-native-program")]
            if !parallel.regions.is_empty() {
                crate::native_artifact::program::codegen::parallel::dispatch(
                    b,
                    program_phase.expect("managed worker phase"),
                    frame,
                    &parallel,
                    &blocks,
                    hooks.as_ref().expect("managed worker hooks"),
                    error,
                );
            }
            let boundary = plan
                .activation_boundary()
                .ok_or_else(|| backend("managed plan has no activation checkpoint"))?;
            #[cfg(feature = "canonical-native-program")]
            if let Some(region) = &prefix {
                let snapshot = b.create_block();
                let dispatch = b.create_block();
                let phase = program_phase.expect("managed resume phase");
                let capture_tail = b.create_block();
                let scan_or_snapshot = b.create_block();
                let is_capture_tail = b.ins().icmp_imm(
                    IntCC::Equal,
                    phase,
                    crate::native_artifact::program::protocol::abi::PHASE_CAPTURE_TAIL as i64,
                );
                b.ins()
                    .brif(is_capture_tail, capture_tail, &[], scan_or_snapshot, &[]);
                b.switch_to_block(capture_tail);
                let target = frame_load(
                    b,
                    frame,
                    core::mem::offset_of!(
                        crate::native_artifact::program::protocol::abi::Frame,
                        resume_site
                    ),
                );
                let at_exit = b
                    .ins()
                    .icmp_imm(IntCC::Equal, target, i64::from(region.tail.exit()));
                guard(b, at_exit, error, STATUS_ABI);
                let facts = prefix_facts.expect("managed prefix entry facts");
                guard(b, facts.final_input, error, STATUS_ABI);
                b.ins().jump(
                    blocks[region.tail.exit() as usize].expect("reachable scan exit"),
                    &[],
                );
                b.switch_to_block(scan_or_snapshot);
                b.ins().brif(facts.is_prefix, dispatch, &[], snapshot, &[]);
                b.switch_to_block(dispatch);
                let target = frame_load(
                    b,
                    frame,
                    core::mem::offset_of!(
                        crate::native_artifact::program::protocol::abi::Frame,
                        resume_site
                    ),
                );
                let at_boundary = b.ins().icmp_imm(IntCC::Equal, target, i64::from(boundary));
                let at_header =
                    b.ins()
                        .icmp_imm(IntCC::Equal, target, i64::from(region.tail.header()));
                let valid = b.ins().bor(at_boundary, at_header);
                guard(b, valid, error, STATUS_ABI);
                let reopen = b.create_block();
                b.ins().brif(at_header, reopen, &[], snapshot, &[]);
                b.switch_to_block(reopen);
                crate::native_artifact::program::codegen::prefix::resume(
                    *options,
                    b,
                    locals,
                    region,
                    &borrowed,
                    &closed,
                    hooks.as_ref().expect("managed hooks"),
                    error,
                )?;
                b.ins().jump(
                    blocks[region.tail.header() as usize].expect("reachable prefix header"),
                    &[],
                );
                b.switch_to_block(snapshot);
            }
            b.ins().jump(
                blocks[boundary as usize].expect("reachable checkpoint"),
                &[],
            );
        }
        let body_error = b.create_block();
        b.append_block_param(body_error, types::I64);
        b.set_cold_block(body_error);
        for (index, block) in plan.blocks().iter().enumerate() {
            let error = body_error;
            if !reachable[index] {
                continue;
            }
            b.switch_to_block(blocks[index].expect("reachable block"));
            let locals = locals_home
                .map(|slot| b.ins().stack_load(types::I64, slot, 0))
                .unwrap_or(locals);
            #[cfg(feature = "canonical-native-program")]
            if let Some(hooks) = &hooks {
                crate::native_artifact::program::codegen::scalar_loop::emit_entry(
                    &controls,
                    b,
                    frame,
                    locals,
                    plan,
                    &closed,
                    &aggregates,
                    &borrowed,
                    prefix.as_ref(),
                    prefix_facts,
                    index,
                    hooks,
                    &blocks,
                    error,
                    report,
                )?;
                crate::native_artifact::program::codegen::readonly_numeric::emit_entry(
                    &controls, b, frame, locals, plan, &closed, index, hooks, &blocks, error,
                    report,
                )?;
                let outlined = packed_functions
                    .iter()
                    .find(|(site, _, _, _)| *site == index)
                    .map(|(_, function, _, _)| module.declare_func_in_func(*function, b.func));
                crate::native_artifact::program::codegen::packed_loop::emit_entry(
                    &controls,
                    b,
                    frame,
                    locals,
                    user_context,
                    plan,
                    &closed,
                    owner.as_ref(),
                    &aggregates,
                    index,
                    hooks,
                    &blocks,
                    error,
                    outlined,
                    report,
                )?;
            }
            let before = b.func.dfg.num_insts();
            set_origin_with_flags(b, frame, index as u64, origin_flags);
            #[cfg(feature = "canonical-native-program")]
            if let crate::native_artifact::codegen::control::plan::CodePlan::Program(program) = plan
            {
                let prepared = program;
                let program = prepared.plan();
                crate::native_artifact::program::codegen::parallel::before_visit(
                    b,
                    program,
                    &parallel,
                    index,
                    program_phase.expect("managed worker phase"),
                    frame,
                    locals,
                    hooks.as_ref().expect("managed worker hooks"),
                    error,
                )?;
            }
            if plan.activation_boundary() == Some(index as u32) {
                let phase =
                    program_phase.ok_or_else(|| backend("checkpoint without managed phase"))?;
                let checkpoint = b.create_block();
                let turn = b.create_block();
                let fresh = b.ins().icmp_imm(IntCC::Equal, phase, 0);
                b.ins().brif(fresh, checkpoint, &[], turn, &[]);
                b.switch_to_block(checkpoint);
                let status = b.ins().iconst(types::I64, 5);
                b.ins().return_(&[status]);
                b.switch_to_block(turn);
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(region) = &prefix
                && region.tail.header() as usize == index
            {
                crate::native_artifact::program::codegen::prefix::gate(
                    *options,
                    b,
                    locals,
                    prefix_facts.expect("managed prefix entry facts"),
                    region,
                    &borrowed,
                    &closed,
                    hooks.as_ref().expect("managed hooks"),
                    error,
                )?;
            }
            #[cfg(feature = "canonical-native-program")]
            let admission_join = if let Some(hooks) = &hooks {
                crate::native_artifact::program::codegen::tail::admission(
                    *options, b, locals, &closed, index, hooks, error,
                )?
            } else {
                None
            };
            #[cfg(feature = "canonical-native-program")]
            let map_admission_join =
                crate::native_artifact::program::codegen::map_owner::admission(
                    b,
                    locals,
                    owner.as_ref(),
                    &map_admission,
                    index,
                    hooks.as_ref(),
                )?;
            #[cfg(feature = "canonical-native-program")]
            if let Some(region) = &prefix
                && region.tail.exit() as usize == index
            {
                crate::native_artifact::program::codegen::prefix::scanned(
                    *options,
                    b,
                    locals,
                    prefix_facts.expect("managed prefix entry facts"),
                    region,
                    &borrowed,
                    &closed,
                    hooks.as_ref().expect("managed hooks"),
                    error,
                )?;
            }
            admit(b, callback_signature, callback, user_context, index, error);
            #[cfg(feature = "canonical-native-program")]
            if let Some(joined) = map_admission_join {
                b.ins().jump(joined, &[]);
                b.switch_to_block(joined);
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(joined) = admission_join {
                b.ins().jump(joined, &[]);
                b.switch_to_block(joined);
            }
            #[cfg(feature = "canonical-native-program")]
            let segment_ir = if let Some(hooks) = &hooks {
                crate::native_artifact::program::codegen::numeric_segments::emit_entry(
                    &controls, b, frame, locals, plan, index, hooks, &blocks, error, report,
                )?
            } else {
                0
            };
            #[cfg(not(feature = "canonical-native-program"))]
            let segment_ir = 0;
            #[cfg(feature = "canonical-native-program")]
            if let crate::native_artifact::codegen::control::plan::CodePlan::Program(prepared) =
                plan
                && let NativeIterationTerminator::Header {
                    iteration, exit, ..
                } = block.terminator()
                && let Some(region) = prepared
                    .metadata()
                    .uniform_word_collections
                    .iter()
                    .find(|region| region.iteration() == iteration)
            {
                hooks
                    .as_ref()
                    .ok_or_else(|| backend("uniform collection has no owner hooks"))?
                    .uniform_words(
                        b,
                        index,
                        region,
                        locals,
                        plan.value_count() + iteration as usize,
                        blocks[exit as usize].expect("reachable collection exit"),
                        error,
                    )?;
            }
            #[cfg(feature = "canonical-native-program")]
            if let NativeIterationTerminator::Header {
                iteration, exit, ..
            } = block.terminator()
                && let Some((region, function, _, _)) = window_functions
                    .iter()
                    .find(|(region, ..)| region.iteration() == iteration)
            {
                let function = module.declare_func_in_func(*function, b.func);
                hooks
                    .as_ref()
                    .ok_or_else(|| backend("compiled window has no owner hooks"))?
                    .window_region(
                        b,
                        index,
                        region,
                        function,
                        locals,
                        plan.value_count() + iteration as usize,
                        blocks[exit as usize].expect("reachable window exit"),
                        error,
                    )?;
            }
            #[cfg(feature = "canonical-native-program")]
            if let NativeIterationTerminator::Header {
                iteration, exit, ..
            } = block.terminator()
                && let Some((region, function, _, _)) = map_functions
                    .iter()
                    .find(|(region, ..)| region.iteration() == iteration)
            {
                let function = module.declare_func_in_func(*function, b.func);
                hooks
                    .as_ref()
                    .ok_or_else(|| backend("compiled String map has no owner hooks"))?
                    .string_map_region(
                        b,
                        index,
                        region,
                        function,
                        locals,
                        plan.value_count() + iteration as usize,
                        blocks[exit as usize].expect("reachable map exit"),
                        error,
                    )?;
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(hooks) = &hooks {
                crate::native_artifact::program::codegen::borrowed::prepare(
                    *options, b, locals, &borrowed, index, hooks, error,
                )?;
            }
            #[cfg(feature = "canonical-native-program")]
            let borrowed_join = crate::native_artifact::program::codegen::borrowed::emit(
                *options,
                b,
                locals,
                &borrowed,
                &closed,
                index,
                hooks.as_ref(),
                error,
            )?;
            #[cfg(feature = "canonical-native-program")]
            let (owner_join, owner_handled) =
                crate::native_artifact::program::codegen::map_owner::emit(
                    b,
                    locals,
                    owner.as_ref(),
                    &closed,
                    &frozen,
                    index,
                    hooks.as_ref(),
                    plan,
                    error,
                )?;
            #[cfg(not(feature = "canonical-native-program"))]
            let owner_handled = false;
            #[cfg(feature = "canonical-native-program")]
            let aggregate = crate::native_artifact::codegen::common::aggregates::emit_memory(
                b,
                locals,
                &aggregates,
                index,
                error,
            )?;
            #[cfg(not(feature = "canonical-native-program"))]
            let aggregate = false;
            match block.action() {
                _ if owner_handled => {}
                _ if aggregate
                    && !matches!(
                        block.action(),
                        NativeIterationAction::Initialize { .. }
                            | NativeIterationAction::Latch { .. }
                    ) => {}
                NativeIterationAction::PrepareStringSpanSelector { collection } => {
                    #[cfg(feature = "canonical-native-program")]
                    {
                        let crate::native_artifact::codegen::control::plan::CodePlan::Program(
                            program,
                        ) = plan
                        else {
                            return Err(backend("String span requires managed program transport"));
                        };
                        let program = program.plan();
                        let projection = program.collections()[*collection as usize]
                            .string_span_projection()
                            .ok_or_else(|| backend("String span lacks selected proof"))?;
                        let hooks = hooks
                            .as_ref()
                            .ok_or_else(|| backend("String span lacks managed hooks"))?;
                        let first = load(b, locals, projection.operands()[0])?;
                        let second = load(b, locals, projection.operands()[1])?;
                        hooks.collection(b, index, [first, second], error)?;
                    }
                    #[cfg(not(feature = "canonical-native-program"))]
                    {
                        let _ = collection;
                        return Err(backend("String span requires managed program transport"));
                    }
                }
                NativeIterationAction::PrepareU64Range { iteration } => {
                    let mech_engine::native_artifact::NativeControlLoopRef::Collection(generator) =
                        plan.iteration(*iteration as usize)
                    else {
                        return Err(backend("range preparation outside collection loop"));
                    };
                    let mech_engine::native_artifact::NativeCollectionSource::U64Range(cursor) =
                        generator.source()
                    else {
                        return Err(backend("range preparation lacks selected cursor"));
                    };
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing range preparation hooks"))?;
                    let first = load(b, locals, cursor.operands()[0])?;
                    let second = load(b, locals, cursor.operands()[1])?;
                    hooks.collection(b, index, [first, second], error)?;
                    // No Matrix result word/handle is written or fabricated.
                }
                NativeIterationAction::Structural { output, .. } => {
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("structural observation without managed hooks"))?;
                    let zero = b.ins().iconst(types::I64, 0);
                    hooks.call(
                        b,
                        index,
                        GlobalHelper::StructuralObservation,
                        [zero; 3],
                        error,
                    );
                    if plan.kind(*output).is_some() {
                        let word = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                        b.ins().store(
                            MemFlags::new(),
                            word,
                            locals,
                            offset(output.get() as usize)?,
                        );
                    }
                }
                NativeIterationAction::LibraryCall {
                    operands, output, ..
                } => {
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing mixed call hooks"))?;
                    #[cfg(feature = "canonical-native-program")]
                    if joined_maps.iter().any(|(site, _)| *site as usize == index) {
                        // Ordinary managed calls return no scalar result. Give
                        // the optional composition handshake a fresh declined
                        // marker rather than observing a prior scalar's word.
                        let zero = b.ins().iconst(types::I64, 0);
                        b.ins().store(MemFlags::new(), zero, hooks.result, 0);
                    }
                    hooks.library(
                        b,
                        index,
                        operands,
                        locals,
                        |operand| plan.kind(operand).is_some(),
                        error,
                    )?;
                    #[cfg(feature = "canonical-native-program")]
                    if let Some((_, function)) =
                        joined_maps.iter().find(|(site, _)| *site as usize == index)
                    {
                        let function = module.declare_func_in_func(*function, b.func);
                        hooks.joined_string_map(b, index, function, error)?;
                    }
                    if plan.kind(*output).is_some() {
                        #[cfg(feature = "canonical-native-program")]
                        let count_join = prefix_facts.and_then(|facts| {
                            crate::native_artifact::program::codegen::prefix::count_store(
                                b,
                                facts,
                                prefix.as_ref(),
                                index,
                            )
                        });
                        let value = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                        b.ins().store(
                            MemFlags::new(),
                            value,
                            locals,
                            offset(output.get() as usize)?,
                        );
                        #[cfg(feature = "canonical-native-program")]
                        if let Some(joined) = count_join {
                            b.ins().jump(joined, &[]);
                            b.switch_to_block(joined);
                        }
                    }
                }
                NativeIterationAction::Scalar(action) => {
                    #[cfg(feature = "canonical-native-program")]
                    let count_action = if let Some(facts) = prefix_facts {
                        crate::native_artifact::program::codegen::prefix::count_action(
                            b,
                            locals,
                            facts,
                            prefix.as_ref(),
                            index,
                            action,
                            error,
                        )?
                    } else {
                        false
                    };
                    #[cfg(not(feature = "canonical-native-program"))]
                    let count_action = false;
                    if count_action {
                        // The selected EOF-dependent action was emitted above.
                    } else if let NativeMatchAction::Copy { output, .. } = action
                        && plan.kind(*output).is_none()
                    {
                        let hooks = hooks
                            .as_ref()
                            .ok_or_else(|| backend("missing managed move hook"))?;
                        let zero = b.ins().iconst(types::I64, 0);
                        hooks.call(b, index, GlobalHelper::Move, [zero; 3], error);
                    } else {
                        emit_scalar_action(b, locals, action, error)?;
                    }
                }
                NativeIterationAction::Initialize { iteration } => {
                    let count = match plan.iteration(*iteration as usize) {
                        mech_engine::native_artifact::NativeControlLoopRef::Iteration(
                            iteration,
                        ) => {
                            if aggregate {
                                // The fixed-width carrier words were copied above.
                            } else if plan.kind(iteration.carrier()).is_some() {
                                let value = load(b, locals, iteration.seed())?;
                                b.ins().store(
                                    MemFlags::new(),
                                    value,
                                    locals,
                                    offset(iteration.carrier().get() as usize)?,
                                );
                            } else {
                                let hooks = hooks
                                    .as_ref()
                                    .ok_or_else(|| backend("missing managed carrier hook"))?;
                                let zero = b.ins().iconst(types::I64, 0);
                                hooks.call(b, index, GlobalHelper::Move, [zero; 3], error);
                            }
                            b.ins().iconst(types::I64, iteration.count() as i64)
                        }
                        mech_engine::native_artifact::NativeControlLoopRef::Collection(_) => {
                            let hooks = hooks
                                .as_ref()
                                .ok_or_else(|| backend("missing collection initializer hook"))?;
                            let zero = b.ins().iconst(types::I64, 0);
                            hooks.collection(b, index, [zero; 2], error)?;
                            b.ins().load(types::I64, MemFlags::new(), hooks.result, 0)
                        }
                        mech_engine::native_artifact::NativeControlLoopRef::Tail(tail) => {
                            let zero = b.ins().iconst(types::I64, 0);
                            if !aggregate && plan.kind(tail.carrier()).is_some() {
                                let value = load(b, locals, tail.seed())?;
                                b.ins().store(
                                    MemFlags::new(),
                                    value,
                                    locals,
                                    offset(tail.carrier().get() as usize)?,
                                );
                            } else if !aggregate {
                                hooks
                                    .as_ref()
                                    .ok_or_else(|| backend("missing tail initializer hook"))?
                                    .call(b, index, GlobalHelper::Move, [zero; 3], error);
                            }
                            zero
                        }
                    };
                    let slot = plan.value_count() + *iteration as usize;
                    b.ins().store(MemFlags::new(), count, locals, offset(slot)?);
                }
                NativeIterationAction::Latch { iteration } => {
                    if let mech_engine::native_artifact::NativeControlLoopRef::Tail(tail) =
                        plan.iteration(*iteration as usize)
                    {
                        if !aggregate && plan.kind(tail.carrier()).is_some() {
                            let value = load(b, locals, tail.next())?;
                            b.ins().store(
                                MemFlags::new(),
                                value,
                                locals,
                                offset(tail.carrier().get() as usize)?,
                            );
                        } else if !aggregate {
                            let hooks = hooks
                                .as_ref()
                                .ok_or_else(|| backend("missing tail latch hook"))?;
                            let zero = b.ins().iconst(types::I64, 0);
                            hooks.call(b, index, GlobalHelper::Move, [zero; 3], error);
                        }
                    } else {
                        let id = *iteration as usize;
                        let counter = b.ins().load(
                            types::I64,
                            MemFlags::new(),
                            locals,
                            offset(plan.value_count() + id)?,
                        );
                        let nonzero = b.ins().icmp_imm(IntCC::NotEqual, counter, 0);
                        guard(b, nonzero, error, STATUS_ABI);
                        if let mech_engine::native_artifact::NativeControlLoopRef::Iteration(
                            iteration,
                        ) = plan.iteration(id)
                        {
                            if aggregate {
                                // Copy was emitted above; counted-loop progress is unchanged.
                            } else if plan.kind(iteration.carrier()).is_some() {
                                let next = load(b, locals, iteration.next())?;
                                b.ins().store(
                                    MemFlags::new(),
                                    next,
                                    locals,
                                    offset(iteration.carrier().get() as usize)?,
                                );
                            } else {
                                let hooks = hooks
                                    .as_ref()
                                    .ok_or_else(|| backend("missing managed latch hook"))?;
                                let zero = b.ins().iconst(types::I64, 0);
                                hooks.call(b, index, GlobalHelper::Move, [zero; 3], error);
                            }
                        }
                        let remaining = b.ins().iadd_imm(counter, -1);
                        b.ins().store(
                            MemFlags::new(),
                            remaining,
                            locals,
                            offset(plan.value_count() + id)?,
                        );
                    }
                }
                NativeIterationAction::CollectionElement { iteration } => {
                    let mech_engine::native_artifact::NativeControlLoopRef::Collection(data) =
                        plan.iteration(*iteration as usize)
                    else {
                        return Err(backend("element hook outside collection loop"));
                    };
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing collection element hook"))?;
                    let zero = b.ins().iconst(types::I64, 0);
                    hooks.collection(b, index, [zero; 2], error)?;
                    if let Some(output) = data.element().filter(|&value| plan.kind(value).is_some())
                    {
                        let value = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                        b.ins().store(
                            MemFlags::new(),
                            value,
                            locals,
                            offset(output.get() as usize)?,
                        );
                    }
                }
                NativeIterationAction::CollectionBegin { .. }
                | NativeIterationAction::ProjectStringSpan { .. }
                | NativeIterationAction::ProjectStringMapEntry { .. }
                | NativeIterationAction::TextJoinFinish { .. }
                | NativeIterationAction::HistogramQueryStep { .. }
                | NativeIterationAction::HistogramFinish { .. }
                | NativeIterationAction::CollectionFinish { .. }
                | NativeIterationAction::CollectionPush { .. } => {
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing collection builder hook"))?;
                    let zero = b.ins().iconst(types::I64, 0);
                    let value = match block.action() {
                        NativeIterationAction::CollectionPush { value, .. }
                            if plan.kind(*value).is_some() =>
                        {
                            load(b, locals, *value)?
                        }
                        _ => zero,
                    };
                    hooks.collection(b, index, [value, zero], error)?;
                    #[cfg(feature = "canonical-native-program")]
                    if let NativeIterationAction::HistogramQueryStep { collection, step } =
                        block.action()
                    {
                        let crate::native_artifact::codegen::control::plan::CodePlan::Program(
                            program,
                        ) = plan
                        else {
                            return Err(backend(
                                "histogram query requires managed program transport",
                            ));
                        };
                        let program = program.plan();
                        let mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(sink) =
                            program.collections()[*collection as usize].result() else {
                            return Err(backend("histogram query compiler lacks selected proof"));
                        };
                        if let Some(output) = sink.steps()[*step as usize].output()
                            && program.values()[output.get() as usize]
                                .scalar_kind()
                                .is_some()
                        {
                            let word = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                            b.ins().store(
                                MemFlags::new(),
                                word,
                                locals,
                                offset(output.get() as usize)?,
                            );
                        }
                    }
                }
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(joined) = owner_join {
                b.ins().jump(joined, &[]);
                b.switch_to_block(joined);
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(joined) = borrowed_join {
                b.ins().jump(joined, &[]);
                b.switch_to_block(joined);
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(hooks) = &hooks {
                crate::native_artifact::program::codegen::tail::prepare(
                    *options, b, locals, &closed, index, hooks, error,
                )?;
            }
            #[cfg(feature = "canonical-native-program")]
            if let crate::native_artifact::codegen::control::plan::CodePlan::Program(program) = plan
            {
                let prepared = program;
                let program = prepared.plan();
                crate::native_artifact::program::codegen::parallel::after_action(
                    b,
                    program,
                    &parallel,
                    index,
                    program_phase.expect("managed worker phase"),
                    frame,
                    parallel_entry.expect("managed checked entry"),
                    locals,
                    &blocks,
                    hooks.as_ref().expect("managed worker hooks"),
                    error,
                )?;
            }
            match block.terminator() {
                NativeIterationTerminator::TailHeader { body, .. } => {
                    b.ins()
                        .jump(blocks[body as usize].expect("reachable tail body"), &[]);
                }
                NativeIterationTerminator::Scalar(terminator) => match plan {
                    crate::native_artifact::codegen::control::plan::CodePlan::Scalar(p) => {
                        emit_scalar_terminator(
                            b,
                            frame,
                            locals,
                            outputs,
                            terminator,
                            &blocks,
                            p.outputs(),
                            p.values(),
                        )?
                    }
                    #[cfg(feature = "canonical-native-program")]
                    crate::native_artifact::codegen::control::plan::CodePlan::Program(_) => {
                        if matches!(
                            terminator,
                            NativeMatchTerminator::Reject | NativeMatchTerminator::IntegrityFailure
                        ) {
                            // A checked source failure must unwind any active
                            // closed-region fuel/owner loan before returning.
                            let status = match terminator {
                                NativeMatchTerminator::Reject => {
                                    crate::native_artifact::control_abi::STATUS_REJECT
                                }
                                _ => crate::native_artifact::abi::STATUS_INTEGRITY,
                            };
                            let status = b.ins().iconst(types::I64, status as i64);
                            b.ins().jump(error, &[status.into()]);
                        } else if terminator == NativeMatchTerminator::Complete {
                            let hooks = hooks
                                .as_ref()
                                .ok_or_else(|| backend("missing typed completion hook"))?;
                            for out in 0..plan.output_count() {
                                let value = plan.output(out);
                                let first = if plan.kind(value).is_some() {
                                    load(b, locals, value)?
                                } else {
                                    b.ins().iconst(types::I64, 0)
                                };
                                hooks.output(b, index, out, first, error)?;
                            }
                            set_origin_with_flags(b, frame, NO_NODE, origin_flags);
                            let status = b
                                .ins()
                                .iconst(types::I64, crate::native_artifact::abi::STATUS_OK as i64);
                            b.ins().return_(&[status]);
                        } else {
                            crate::native_artifact::control_compiler::emit_branch_terminator(
                                b,
                                locals,
                                terminator,
                                &blocks,
                                |v| plan.kind(v),
                            )?;
                        }
                    }
                },
                NativeIterationTerminator::Header {
                    iteration,
                    body,
                    exit,
                } => {
                    let remaining = b.ins().load(
                        types::I64,
                        MemFlags::new(),
                        locals,
                        offset(plan.value_count() + iteration as usize)?,
                    );
                    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, remaining, 0);
                    b.ins().brif(
                        nonzero,
                        blocks[body as usize].expect("reachable body"),
                        &[],
                        blocks[exit as usize].expect("reachable exit"),
                        &[],
                    );
                }
            }
            // Selected acyclic parent bodies have their own independently
            // derived, runtime-funded instruction envelope. Keep their fixed
            // entry guard in this unchanged ordinary visit audit.
            // The entry-pointer reload precedes specialized region emission;
            // include it in this visit's unchanged conservative template.
            let emitted = (b.func.dfg.num_insts() - before - segment_ir
                + usize::from(locals_home.is_some())) as u64;
            let bound = emission_template_bound(plan, index)?;
            if emitted.checked_add(128).is_none_or(|total| total > bound) {
                return Err(NativeCompileError::InvalidPlan {
                    node: Some(block.origin().node()),
                    reason: "Iteration visit exceeds conservative work template",
                });
            }
        }
        b.switch_to_block(body_error);
        let status = b.block_params(body_error)[0];
        #[cfg(feature = "canonical-native-program")]
        if let Some(hooks) = &hooks {
            let site = b.ins().load(
                types::I64,
                origin_flags,
                frame,
                core::mem::offset_of!(Frame, error_node) as i32,
            );
            crate::native_artifact::program::codegen::tail::abort(
                *options, b, locals, &closed, site, hooks, error,
            )?;
        }
        b.ins().jump(error, &[status.into()]);
        b.switch_to_block(error);
        let status = b.block_params(error)[0];
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        Ok(())
    }
    fn measure<M: Module>(
        self,
        module: &mut M,
        mut context: cranelift_codegen::Context,
        declarations: CfgDeclarations<'plan>,
        limits: NativeCompileLimits,
    ) -> Result<(FuncId, u64, u64, CodegenReport), NativeCompileError> {
        #[cfg(feature = "canonical-native-program")]
        let plan = self.plan;
        #[cfg(feature = "canonical-native-program")]
        let controls = &self.controls;
        #[cfg(feature = "canonical-native-program")]
        let mut report = self.report;
        #[cfg(not(feature = "canonical-native-program"))]
        let report = CodegenReport;
        let function = declarations.function;
        #[cfg(feature = "canonical-native-program")]
        let window_functions = declarations.window_functions;
        #[cfg(feature = "canonical-native-program")]
        let map_functions = declarations.map_functions;
        #[cfg(feature = "canonical-native-program")]
        let joined_functions = declarations.joined_functions;
        #[cfg(feature = "canonical-native-program")]
        let packed_functions = declarations.packed_functions;
        #[cfg(feature = "canonical-native-program")]
        let bound_call_shim = declarations.bound_call_shim;
        #[cfg(feature = "canonical-native-program")]
        if plan.is_program() {
            report.cfg_cleanup = Some(
                crate::native_artifact::program::codegen::cfg_cleanup::optimize(
                    &controls,
                    &mut context,
                    module.isa(),
                )?,
            );
        }
        module
            .define_function(function, &mut context)
            .map_err(backend)?;
        let compiled = context
            .compiled_code()
            .ok_or(NativeCompileError::StackMetadata {
                reason: "Iteration compiled code missing",
            })?;
        let stack = compiled_stack_bytes(&compiled.buffer)?;
        let code = compiled.code_info().total_size as u64;
        #[cfg(feature = "canonical-native-program")]
        let (stack, code) = (
            stack
                .checked_add(
                    window_functions
                        .iter()
                        .map(|(_, _, stack, _)| *stack)
                        .chain(map_functions.iter().map(|(_, _, stack, _)| *stack))
                        .chain(joined_functions.iter().map(|(_, _, stack, _)| *stack))
                        .chain(packed_functions.iter().map(|(_, _, stack, _)| *stack))
                        .max()
                        .unwrap_or(0),
                )
                .ok_or_else(|| backend("compiled region stack overflow"))?,
            window_functions
                .iter()
                .map(|(_, _, _, code)| *code)
                .chain(map_functions.iter().map(|(_, _, _, code)| *code))
                .chain(joined_functions.iter().map(|(_, _, _, code)| *code))
                .chain(packed_functions.iter().map(|(_, _, _, code)| *code))
                .try_fold(
                    code + bound_call_shim.map_or(0, |shim| shim.code),
                    |total, code| total.checked_add(code),
                )
                .ok_or_else(|| backend("compiled region code overflow"))?,
        );
        limit("Iteration stack", stack, limits.max_stack_bytes)?;
        limit("Iteration code", code, limits.max_code_bytes)?;
        Ok((function, stack, code, report))
    }
}

// This is the emitted-instruction envelope, not either runtime visit view.
// Primitive precedence and included supplements intentionally differ.
fn emission_template_bound(
    plan: crate::native_artifact::codegen::control::plan::CodePlan<'_>,
    index: usize,
) -> Result<u64, NativeCompileError> {
    let block = &plan.blocks()[index];
    let bound = match block.action() {
        NativeIterationAction::Scalar(NativeMatchAction::Primitive { op, .. }) => {
            crate::native_artifact::abi::primitive_work(*op)
                .checked_add(128)
                .ok_or_else(|| backend("iteration work overflow"))?
        }
        _ => {
            if block.terminator()
                == NativeIterationTerminator::Scalar(NativeMatchTerminator::Complete)
            {
                (plan.output_count() as u64)
                    .checked_mul(if plan.is_program() { 64 } else { 8 })
                    .and_then(|v| v.checked_add((plan.iteration_count() as u64).checked_mul(8)?))
                    .and_then(|v| v.checked_add(256))
                    .ok_or_else(|| backend("iteration completion overflow"))?
            } else {
                256
            }
        }
    };
    let bound = bound
        .checked_add(
            crate::native_artifact::iteration_abi::helper_operand_work(block.action())
                .map_err(backend)?,
        )
        .ok_or_else(|| backend("iteration operand work overflow"))?;
    #[cfg(feature = "canonical-native-program")]
    let bound =
        if let crate::native_artifact::codegen::control::plan::CodePlan::Program(prepared) = plan {
            let metadata = prepared.metadata();
            bound
                .checked_add(
                    metadata
                        .borrowed_strings
                        .extra_work(index)
                        .map_err(backend)?,
                )
                .and_then(|n| n.checked_add(metadata.closed_tails.extra_work(index)))
                .and_then(|n| {
                    n.checked_add(
                        crate::native_artifact::program::prepare::prefix::extra_work(
                            metadata.prefix.as_ref(),
                            index,
                        ),
                    )
                })
                .ok_or_else(|| backend("borrowed String instruction work overflow"))?
        } else {
            bound
        };
    Ok(bound)
}

#[cfg(feature = "canonical-native")]
pub(in crate::native_artifact) mod plan;
