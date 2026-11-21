//! Compile the same checked numeric plan under an explicit entry contract.
use super::super::{
    arrays::ArrayImports,
    checked_region_discovery, checked_traversals,
    phase_bodies::{PhaseBodies, define_body, define_prepared_body, prepare_body},
    phase_layout,
};
use super::*;
use crate::native_artifact::{
    codegen::{common::unsigned_guard_facts, control::CfgCleanup, options::OptimizationPass},
    program::{
        codegen::{numeric_compact, wide_equal},
        prepare::{readonly_numeric::Proof, tail::Region},
    },
};
use cfg_cleanup::composed_reads::{Admission, Capture, DenseTables, Literal, View};
use cranelift_codegen::{
    Context,
    ir::{self, Signature},
    isa::TargetIsa,
};
use std::sync::mpsc;
#[path = "phase_histogram.rs"]
mod bounded_histogram;

pub(super) struct Composed {
    pub function: FuncId,
    pub admission: Admission,
    pub summary: serde_json::Value,
}

pub(super) struct Compiled {
    pub function: FuncId,
    pub summary: serde_json::Value,
    pub code: Vec<u8>,
    pub composed: Option<Composed>,
    pub packed_input: Option<Vec<usize>>,
    pub entry_spans: Vec<cfg_cleanup::admitted_spans::Admission>,
}
type FinishReport = Box<
    dyn FnOnce(
        &cranelift_codegen::CompiledCode,
        &[(usize, u64, bool)],
    ) -> Result<serde_json::Value, NativeCompileError>,
>;

pub(super) struct Prepared {
    context: Context,
    details: Details,
}
struct Details {
    function: FuncId,
    entry_bounds: Vec<(usize, u64, bool)>,
    entry_spans: Vec<cfg_cleanup::admitted_spans::Admission>,
    finish_report: FinishReport,
    composed: Option<Composed>,
    packed_input: Option<Vec<usize>>,
    vcode_path: Option<std::path::PathBuf>,
}
pub(super) struct Pending {
    ticket: super::super::compile_jobs::Ticket,
    details: Details,
}
pub(super) enum General {
    Ready(Prepared),
    Deferred(DeferredGeneral),
}
pub(super) struct DeferredGeneral {
    context: Context,
    controls: CompilationControls,
    report: std::sync::Arc<numeric_compact::Report>,
    private: bool,
    input: SummaryInput,
}
pub(super) struct GeneralPending {
    job: GeneralJob,
    prepared_ir: Option<mpsc::Receiver<ir::Function>>,
}
enum GeneralJob {
    Ready(Pending),
    Deferred {
        ticket: super::super::compile_jobs::Ticket<(Context, PhaseCleanup)>,
        report: std::sync::Arc<numeric_compact::Report>,
        input: SummaryInput,
    },
}
pub(super) enum VectorCompilation {
    Equivalent(Prepared),
    Pending(Pending),
}
impl General {
    pub(super) fn function(&self) -> FuncId {
        match self {
            Self::Ready(ready) => ready.details.function,
            Self::Deferred(ready) => ready.input.function,
        }
    }
    pub(super) fn entry_bounds(&self) -> &[(usize, u64, bool)] {
        match self {
            Self::Ready(ready) => ready.entry_bounds(),
            Self::Deferred(ready) => &ready.input.entry_bounds,
        }
    }
    pub(super) fn define<M: Module>(
        self,
        module: &mut M,
        bodies: Option<&mut PhaseBodies>,
    ) -> Result<Compiled, NativeCompileError> {
        match self {
            Self::Ready(ready) => ready.define(module, bodies),
            Self::Deferred(ready) => ready.optimize(module.isa())?.define(module, bodies),
        }
    }
    pub(super) fn start(
        self,
        pool: &super::super::compile_jobs::Pool,
        compare_vector_ir: bool,
    ) -> Result<GeneralPending, NativeCompileError> {
        let (sender, prepared_ir) = if compare_vector_ir {
            let (sender, receiver) = mpsc::channel();
            (Some(sender), Some(receiver))
        } else {
            (None, None)
        };
        let job = match self {
            Self::Ready(ready) => {
                if let Some(sender) = sender
                    && !ready.context.want_disasm
                {
                    let _ = sender.send(ready.context.func.clone());
                }
                GeneralJob::Ready(ready.start(pool)?)
            }
            Self::Deferred(ready) => {
                let DeferredGeneral {
                    mut context,
                    controls,
                    report,
                    private,
                    input,
                } = ready;
                let worker_report = report.clone();
                let symbol = input.symbol.clone();
                let input_words = input.input_words;
                let output_words = input.output_words;
                let ticket = pool.submit_work(move |isa| {
                    let cleanup = optimize_phase(
                        &mut context,
                        &controls,
                        isa,
                        &worker_report,
                        &symbol,
                        input_words,
                        output_words,
                        None,
                        private,
                        None,
                        true,
                    )?;
                    prepare_body(isa, &mut context)?;
                    // Send the input before lowering so different vector bodies can still
                    // start machine compilation while the general job is running.
                    if let Some(sender) = sender {
                        let _ = sender.send(context.func.clone());
                    }
                    let context = super::super::compile_jobs::compile(context, isa)?;
                    Ok((context, cleanup))
                })?;
                GeneralJob::Deferred {
                    ticket,
                    report,
                    input,
                }
            }
        };
        Ok(GeneralPending { job, prepared_ir })
    }
}
impl DeferredGeneral {
    fn optimize(mut self, isa: &dyn TargetIsa) -> Result<Prepared, NativeCompileError> {
        let cleanup = optimize_phase(
            &mut self.context,
            &self.controls,
            isa,
            &self.report,
            &self.input.symbol,
            self.input.input_words,
            self.input.output_words,
            None,
            self.private,
            None,
            true,
        )?;
        prepare_body(isa, &mut self.context)?;
        Ok(finish_preparation(
            self.context,
            &self.report,
            cleanup,
            self.input,
            Vec::new(),
            None,
            None,
            None,
            None,
        ))
    }
}
impl GeneralPending {
    pub(super) fn start_vector(
        &mut self,
        vector: Prepared,
        pool: &super::super::compile_jobs::Pool,
    ) -> Result<VectorCompilation, NativeCompileError> {
        // Compare the whole function, including its signature, effects, source locations,
        // external names, and target layout hints. Both jobs use the same ISA and control plane.
        if !vector.context.want_disasm
            && let Some(receiver) = self.prepared_ir.take()
            && receiver
                .recv()
                .is_ok_and(|general| general == vector.context.func)
        {
            return Ok(VectorCompilation::Equivalent(vector));
        }
        vector.start(pool).map(VectorCompilation::Pending)
    }

    pub(super) fn finish<M: Module>(
        self,
        module: &mut M,
    ) -> Result<(Compiled, Context), NativeCompileError> {
        match self.job {
            GeneralJob::Ready(pending) => pending.finish_with_context(module),
            GeneralJob::Deferred {
                ticket,
                report,
                input,
            } => {
                let (context, cleanup) = ticket.wait()?;
                let Prepared { context, details } = finish_preparation(
                    context,
                    &report,
                    cleanup,
                    input,
                    Vec::new(),
                    None,
                    None,
                    None,
                    None,
                );
                super::super::compile_jobs::publish(module, details.function, &context)?;
                Ok((details.finish(&context)?, context))
            }
        }
    }
}

impl VectorCompilation {
    pub(super) fn finish<M: Module>(
        self,
        module: &mut M,
        general: &Context,
    ) -> Result<Compiled, NativeCompileError> {
        match self {
            Self::Equivalent(vector) => {
                // Keep the vector's own guard and cleanup metadata. Its declaration was
                // replayed for stable IDs, and the identical body will never be selected.
                let compiled = vector.details.finish(general)?;
                eprintln!("compact vector code reused: {}", compiled.summary["symbol"]);
                Ok(compiled)
            }
            Self::Pending(pending) => pending.finish(module),
        }
    }
}

impl Prepared {
    pub(super) fn entry_bounds(&self) -> &[(usize, u64, bool)] {
        &self.details.entry_bounds
    }
    pub(super) fn has_dense_composition(&self) -> bool {
        self.details
            .composed
            .as_ref()
            .is_some_and(|body| body.admission.immutable_code_bytes.is_some())
    }

    pub(super) fn into_composition(self) -> Option<Composed> {
        self.details.composed
    }

    pub(super) fn define<M: Module>(
        mut self,
        module: &mut M,
        bodies: Option<&mut PhaseBodies>,
    ) -> Result<Compiled, NativeCompileError> {
        let symbol = module
            .declarations()
            .get_function_decl(self.details.function)
            .name
            .clone()
            .ok_or_else(|| backend("compact numeric phase lacks a symbol"))?;
        let (function, target) = define_prepared_body(
            module,
            self.details.function,
            &symbol,
            &mut self.context,
            bodies,
        )?;
        self.details.function = function;
        let mut compiled = self.details.finish(&self.context)?;
        compiled.summary["symbol"] = serde_json::json!(target);
        Ok(compiled)
    }
    pub(super) fn start(
        self,
        pool: &super::super::compile_jobs::Pool,
    ) -> Result<Pending, NativeCompileError> {
        Ok(Pending {
            ticket: pool.submit(self.context)?,
            details: self.details,
        })
    }
}
impl Pending {
    pub(super) fn finish<M: Module>(self, module: &mut M) -> Result<Compiled, NativeCompileError> {
        self.finish_with_context(module)
            .map(|(compiled, _)| compiled)
    }

    fn finish_with_context<M: Module>(
        self,
        module: &mut M,
    ) -> Result<(Compiled, Context), NativeCompileError> {
        let context = self.ticket.wait()?;
        super::super::compile_jobs::publish(module, self.details.function, &context)?;
        Ok((self.details.finish(&context)?, context))
    }
}
impl Details {
    fn finish(self, context: &Context) -> Result<Compiled, NativeCompileError> {
        let compiled = context
            .compiled_code()
            .ok_or_else(|| backend("compact numeric phase lacks code"))?;
        if let Some(path) = self.vcode_path {
            let text = compiled
                .vcode
                .as_ref()
                .ok_or_else(|| backend("phase VCode diagnostic was not emitted"))?;
            std::fs::write(path, text).map_err(backend)?;
        }
        let mut summary = (self.finish_report)(compiled, &self.entry_bounds)?;
        if let Some(words) = &self.packed_input {
            summary["input_words"] = serde_json::json!(words.len());
            summary["packed_input_words"] = serde_json::json!(words);
        }
        Ok(Compiled {
            function: self.function,
            summary,
            code: compiled.buffer.data().to_vec(),
            composed: self.composed,
            packed_input: self.packed_input,
            entry_spans: self.entry_spans,
        })
    }
}

pub(super) fn define<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    id: usize,
    observe: bool,
    input_words: usize,
    output_words: usize,
    fixed_count: usize,
    selected_output: Option<&BTreeSet<usize>>,
    pack_input: bool,
    vectors: Option<&VectorContract>,
    private: Option<&PrivatePhase>,
    no_growth: Option<&NoGrowth>,
    literal_views: &BTreeMap<usize, Literal>,
    dense_tables: &mut DenseTables,
    mut bodies: Option<&mut PhaseBodies>,
    suffix: &str,
) -> Result<Compiled, NativeCompileError> {
    prepare(
        module,
        prepared,
        id,
        observe,
        input_words,
        output_words,
        fixed_count,
        selected_output,
        pack_input,
        vectors,
        private,
        no_growth,
        literal_views,
        dense_tables,
        bodies.as_deref_mut(),
        suffix,
    )?
    .define(module, bodies)
}

pub(super) fn prepare<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    id: usize,
    observe: bool,
    input_words: usize,
    output_words: usize,
    fixed_count: usize,
    selected_output: Option<&BTreeSet<usize>>,
    pack_input: bool,
    vectors: Option<&VectorContract>,
    private: Option<&PrivatePhase>,
    no_growth: Option<&NoGrowth>,
    literal_views: &BTreeMap<usize, Literal>,
    dense_tables: &mut DenseTables,
    bodies: Option<&mut PhaseBodies>,
    suffix: &str,
) -> Result<Prepared, NativeCompileError> {
    match prepare_impl(
        module,
        prepared,
        id,
        observe,
        input_words,
        output_words,
        fixed_count,
        selected_output,
        pack_input,
        vectors,
        private,
        no_growth,
        literal_views,
        dense_tables,
        bodies,
        suffix,
        false,
    )? {
        General::Ready(ready) => Ok(ready),
        General::Deferred(_) => unreachable!("serial preparation cannot defer cleanup"),
    }
}

pub(super) fn prepare_general<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    id: usize,
    observe: bool,
    input_words: usize,
    output_words: usize,
    fixed_count: usize,
    selected_output: Option<&BTreeSet<usize>>,
    pack_input: bool,
    vectors: Option<&VectorContract>,
    private: Option<&PrivatePhase>,
    no_growth: Option<&NoGrowth>,
    literal_views: &BTreeMap<usize, Literal>,
    dense_tables: &mut DenseTables,
    bodies: Option<&mut PhaseBodies>,
    suffix: &str,
    defer: bool,
) -> Result<General, NativeCompileError> {
    prepare_impl(
        module,
        prepared,
        id,
        observe,
        input_words,
        output_words,
        fixed_count,
        selected_output,
        pack_input,
        vectors,
        private,
        no_growth,
        literal_views,
        dense_tables,
        bodies,
        suffix,
        defer,
    )
}

fn general_ir_observed() -> bool {
    [
        "MECH_COMPACT_PHASE_CONTROLLER_WINDOWS",
        "MECH_INSPECT_PHASE_CLIF",
        "MECH_INSPECT_PHASE_VCODE",
        "MECH_COMPACT_FAILURE_DIR",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
}

pub(super) fn refused_general_ssa_unobserved() -> bool {
    !general_ir_observed()
        && [
            "MECH_COMPACT_DUMP_LLVM_DIRECTORY",
            "MECH_TRAVERSAL_FLOOR_DIAGNOSTIC",
            "MECH_COMPACT_SHARE_NUMERIC_ERROR_TAILS",
        ]
        .iter()
        .all(|name| std::env::var_os(name).is_none())
}

fn phase_signature<M: Module>(module: &M) -> Signature {
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 3];
    signature.returns = vec![AbiParam::new(types::I64); 2];
    signature
}

fn declare_phase<M: Module>(
    module: &mut M,
    symbol: &str,
    signature: &Signature,
) -> Result<FuncId, NativeCompileError> {
    let function = module
        .declare_function(symbol, Linkage::Local, signature)
        .map_err(backend)?;
    eprintln!("compact-stage: numeric phase {symbol}");
    Ok(function)
}

pub(super) fn declare_refused_general<M: Module>(
    module: &mut M,
    header: usize,
) -> Result<FuncId, NativeCompileError> {
    let signature = phase_signature(module);
    // The retained vector and the root still use the original module IDs.
    // Reserve the same imports and local declaration as phase_ir.
    ArrayImports::declare(module, false)?;
    let symbol = format!("mech_compact_numeric_{header}");
    let function = declare_phase(module, &symbol, &signature)?;
    eprintln!("compact-stage: skipped general numeric SSA {symbol}");
    Ok(function)
}

fn entry_bounds(proof: &Proof, bounds: &[(usize, u64)]) -> Vec<(usize, u64, bool)> {
    bounds
        .iter()
        .map(|&(array, minimum)| (array, minimum, proof.arrays[array].appends))
        .collect()
}

pub(super) fn refused_general_bounds(
    code: CodePlan<'_>,
    region: &Region,
    private: &PrivatePhase,
) -> Option<Vec<(usize, u64, bool)>> {
    let proof = region.numeric.as_ref()?;
    let bounds = numeric_compact::private_checked_child_bounds(code, region, private)?;
    Some(entry_bounds(proof, &bounds))
}

fn prepare_impl<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    id: usize,
    observe: bool,
    input_words: usize,
    output_words: usize,
    fixed_count: usize,
    selected_output: Option<&BTreeSet<usize>>,
    pack_input: bool,
    vectors: Option<&VectorContract>,
    private: Option<&PrivatePhase>,
    no_growth: Option<&NoGrowth>,
    literal_views: &BTreeMap<usize, Literal>,
    dense_tables: &mut DenseTables,
    mut bodies: Option<&mut PhaseBodies>,
    suffix: &str,
    defer_general: bool,
) -> Result<General, NativeCompileError> {
    let code = CodePlan::Program(prepared);
    let closed = &prepared.metadata().closed_tails;
    let region = &closed.regions[id];
    let header = region.tail.header() as usize;
    let proof = region
        .numeric
        .as_ref()
        .ok_or_else(|| backend("phase lacks checked numeric proof"))?;
    let controls = CompilationControls::new(OptimizationOptions::default());
    let symbol = format!("mech_compact_numeric_{header}{suffix}");
    let signature = phase_signature(module);
    let error_origin_base = bodies.is_some().then_some(header);
    let (mut context, mut report) = phase_ir(
        module,
        code,
        region,
        &signature,
        &controls,
        observe,
        vectors,
        private,
        no_growth,
        selected_output,
        false,
        error_origin_base,
    )?;
    let function = declare_phase(module, &symbol, &signature)?;
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
    let possible_histogram = std::env::var_os("MECH_GROUP_PACKED_HISTOGRAM").is_some()
        && !observe
        && proof.phase.is_none()
        && proof.exits.is_empty()
        && proof.prepaid_sites.is_empty()
        && proof.arrays.iter().all(|array| !array.appends)
        && module.isa().endianness() == cranelift_codegen::ir::Endianness::Little;
    if defer_general
        && !pack_input
        && bodies.is_none()
        && !general_ir_observed()
        && vectors.is_none()
        && no_growth.is_none()
        && literal_views.is_empty()
        && !possible_histogram
    {
        let input = SummaryInput {
            function,
            entry_bounds: entry_bounds(proof, &report.phase_entry_bounds),
            header,
            exit: region.tail.exit(),
            symbol,
            input_words,
            output_words,
            fixed_count,
            actual_unit_append_growth: true,
            checked_regions: None,
            selected_output_words: selected_output.map(BTreeSet::len),
            packed_input: None,
            unsigned_guards: 0,
            packed_halfword_stores: 0,
            admitted_index_guards: 0,
            entry_spans: Vec::new(),
        };
        return Ok(General::Deferred(DeferredGeneral {
            context,
            controls,
            report: std::sync::Arc::new(report),
            private: private.is_some(),
            input,
        }));
    }
    let phase_cleanup = optimize_phase(
        &mut context,
        &controls,
        module.isa(),
        &report,
        &symbol,
        input_words,
        output_words,
        vectors,
        private.is_some(),
        no_growth,
        true,
    )?;
    let cleanup = &phase_cleanup.cleanup;
    let pack_halfwords = cfg!(target_arch = "aarch64")
        && module.isa().endianness() == cranelift_codegen::ir::Endianness::Little
        && std::env::var_os("MECH_PACK_PRIVATE_HALFWORD_READS").is_some();
    let reuse_lengths = std::env::var_os("MECH_REUSE_ADMITTED_VIEW_LENGTHS").is_some();
    let bitwise_indices = std::env::var_os("MECH_REUSE_ADMITTED_BITWISE_INDEX_RANGES").is_some();
    let pack_halfword_stores = std::env::var_os("MECH_PACK_PRIVATE_HALFWORD_STORES");
    let mut paired_halfword_stores = None;
    let mut admitted_index_guards = 0;
    let mut entry_spans = Vec::new();
    if !observe
        && cleanup.enabled
        && no_growth.is_none()
        && vectors.is_some_and(|contract| contract.extend_bounds)
        && private.is_some_and(|contract| contract.permits(proof))
        && let Some(loads) = phase_storage::input_word_loads(&context.func, input_words)
    {
        let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
        let mut lengths = BTreeMap::new();
        for (array, view) in proof.arrays.iter().enumerate() {
            if !view.writable || view.appends || view.width != 4 {
                continue;
            }
            let Ok(at) = packet.binary_search(&(proof.view(array) + 1)) else {
                continue;
            };
            let mut values = loads.iter().filter(|(_, pair)| pair.1 == at);
            if let Some((&value, _)) = values.next()
                && values.next().is_none()
            {
                lengths.insert(value, array);
            }
        }
        let required = cfg_cleanup::admitted_lengths::required_index_floors(
            &context.func,
            &lengths.keys().copied().collect(),
            bitwise_indices,
        );
        for (length, minimum) in required {
            let array = lengths[&length];
            if let Some((_, floor)) = report
                .phase_entry_bounds
                .iter_mut()
                .find(|(old, _)| *old == array)
            {
                *floor = (*floor).max(minimum);
            } else {
                report.phase_entry_bounds.push((array, minimum));
            }
        }
        report.phase_entry_bounds.sort_unstable();
    }
    if (pack_halfwords || reuse_lengths)
        && !observe
        && cleanup.enabled
        && let Some(vectors) = vectors
        && (!vectors.admitted_lengths.is_empty() || vectors.extend_bounds)
        && private.is_some_and(|contract| contract.permits(proof))
        && let Some(loads) = phase_storage::input_word_loads(&context.func, input_words)
    {
        let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
        let input_word = |word| {
            let at = packet.binary_search(&word).ok()?;
            let mut values = loads.iter().filter(|(_, pair)| pair.1 == at);
            let (&value, _) = values.next()?;
            values.next().is_none().then_some(value)
        };
        if pack_halfwords {
            let mut floors = BTreeMap::new();
            for &(array, minimum) in &vectors.admitted_lengths {
                let view = &proof.arrays[array];
                if view.width != 2 || !view.writable {
                    continue;
                }
                if let (Some(pointer), Some(bytes)) =
                    (input_word(proof.view(array)), minimum.checked_mul(2))
                {
                    floors.insert(pointer, bytes);
                }
            }
            if !floors.is_empty() {
                context.flowgraph();
                let (packed, stores) = cfg_cleanup::packed_halfword_reads::fold(
                    &mut context.func,
                    &context.cfg,
                    &context.domtree,
                    &floors,
                );
                paired_halfword_stores = Some(stores);
                if packed != 0 {
                    context.verify(module.isa()).map_err(backend)?;
                    if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                        eprintln!("{symbol} packed private halfword loads: {packed}");
                    }
                }
            }
        }
        if reuse_lengths {
            let admitted_lengths = if vectors.extend_bounds {
                &report.phase_entry_bounds
            } else {
                &vectors.admitted_lengths
            };
            let floors: BTreeMap<_, _> = admitted_lengths
                .iter()
                .filter_map(|&(array, minimum)| {
                    input_word(proof.view(array) + 1).map(|length| (length, minimum))
                })
                .collect();
            if !floors.is_empty() {
                // Keep the later dense scorer composition's default-read structure.
                let (folded, ranged) = if no_growth.is_none()
                    && std::env::var_os("MECH_REUSE_ADMITTED_INDEX_RANGES").is_some()
                {
                    cfg_cleanup::admitted_lengths::fold_index_ranges(
                        &mut context.func,
                        &floors,
                        bitwise_indices,
                    )
                } else {
                    (
                        cfg_cleanup::admitted_lengths::fold(&mut context.func, &floors),
                        0,
                    )
                };
                admitted_index_guards += ranged;
                if folded + ranged != 0 {
                    context.flowgraph();
                    if ranged != 0 {
                        // Removing a read/default edge can orphan a block that
                        // still refers to words defined on the old entry path.
                        context
                            .eliminate_unreachable_code(module.isa())
                            .map_err(backend)?;
                        context.flowgraph();
                    }
                    context.verify(module.isa()).map_err(backend)?;
                    if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                        eprintln!(
                            "{symbol} reused admitted view lengths: {folded} literal, {ranged} indexed"
                        );
                    }
                }
            }
        }
    }
    if !observe
        && cleanup.enabled
        && vectors.is_some()
        && std::env::var_os("MECH_SHARE_CLAMP_DECISIONS").is_some()
    {
        context.flowgraph();
        let reused =
            cfg_cleanup::shared_clamp::fold(&mut context.func, &context.cfg, &context.domtree);
        if reused != 0 {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
            if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                eprintln!("{symbol} shared clamp decisions: {reused}");
            }
        }
    }
    if cfg!(target_arch = "aarch64")
        && !observe
        && cleanup.enabled
        && vectors.is_some()
        && std::env::var_os("MECH_SHORT_CIRCUIT_COMPARISON_BRANCHES").is_some()
    {
        context.flowgraph();
        let changed = cfg_cleanup::selected_continuations::comparison_chain::fold(
            &mut context.func,
            &context.cfg,
            &context.domtree,
        );
        if changed != 0 {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
            if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                eprintln!("{symbol} short-circuited comparison branches: {changed}");
            }
        }
    }
    let mut future_write_hints = Vec::new();
    if !observe
        && cleanup.enabled
        && module.isa().name() == "aarch64"
        && !report.fixed_view_writes.is_empty()
        && let Some(vectors) = vectors
    {
        context.flowgraph();
        if std::env::var_os("MECH_PREFETCH_PRIVATE_WRITES").is_some() {
            future_write_hints = cfg_cleanup::future_store::insert(
                &mut context.func,
                &context.cfg,
                &context.domtree,
                &report.fixed_view_writes,
                &vectors.admitted_lengths,
                std::env::var_os("MECH_PREFETCH_SECOND_PRIVATE_WRITE").is_some(),
            );
        }
        if !future_write_hints.is_empty() {
            context.verify(module.isa()).map_err(backend)?;
        }
    }
    if !observe
        && cleanup.enabled
        && pack_halfwords
        && std::env::var_os("MECH_SCHEDULE_INDEPENDENT_PRIVATE_READS").is_some()
        && let Some(vectors) = vectors
        && private.is_some_and(|contract| contract.permits(proof))
        && let Some(loads) = phase_storage::input_word_loads(&context.func, input_words)
    {
        let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
        let mut views = Vec::new();
        let mut bases = BTreeMap::new();
        for (array, definition) in proof.arrays.iter().enumerate() {
            if !definition.writable || definition.appends {
                continue;
            }
            let Some(at) = packet.binary_search(&proof.view(array)).ok() else {
                continue;
            };
            let mut matches = loads.iter().filter(|(_, pair)| pair.1 == at);
            let Some((&base, _)) = matches.next() else {
                continue;
            };
            if matches.next().is_some() {
                continue;
            }
            let width = definition.width;
            let minimum_bytes = vectors
                .admitted_lengths
                .iter()
                .find_map(|&(at, length)| {
                    (at == array)
                        .then(|| length.checked_mul(u64::from(width)))
                        .flatten()
                })
                .unwrap_or(0);
            bases.insert(array, base);
            views.push(cfg_cleanup::independent_private_read::View {
                base,
                element_bytes: width,
                minimum_bytes,
            });
        }
        let writes = report
            .fixed_view_writes
            .iter()
            .filter_map(|write| bases.get(&write.array).map(|&base| (write.store, base)))
            .collect();
        context.flowgraph();
        if cfg_cleanup::independent_private_read::schedule(
            &mut context.func,
            &context.cfg,
            &context.domtree,
            &views,
            &writes,
        ) {
            context.verify(module.isa()).map_err(backend)?;
            if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                eprintln!("{symbol} scheduled independent private read");
            }
        }
    }
    let bounded_histogram = if std::env::var_os("MECH_BOUNDED_BYTE_HISTOGRAM").is_some()
        && !observe
        && vectors.is_some()
        && no_growth.is_none()
        && proof.phase.is_none()
        && proof.exits.is_empty()
        && proof.prepaid_sites.is_empty()
        && proof.arrays.iter().all(|array| !array.appends)
        && module.isa().endianness() == cranelift_codegen::ir::Endianness::Little
    {
        bounded_histogram::prepare(
            module,
            &mut context,
            proof,
            region,
            report.machine_header,
            input_words,
            &symbol,
            &signature,
            bodies.as_deref_mut(),
        )?
    } else {
        None
    };
    let word_loops = &report;
    let mut histogram = None;
    if std::env::var_os("MECH_GROUP_PACKED_HISTOGRAM").is_some()
        && !observe
        && no_growth.is_none()
        && proof.phase.is_none()
        && proof.exits.is_empty()
        && proof.prepaid_sites.is_empty()
        && proof.arrays.iter().all(|array| !array.appends)
        && module.isa().endianness() == cranelift_codegen::ir::Endianness::Little
        && let Some(loads) = phase_storage::input_word_loads(&context.func, input_words)
    {
        let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
        let word_at = |word| packet.iter().position(|&at| at == word);
        let mut nodes = Vec::new();
        let mut bins = Vec::new();
        for (array, definition) in proof.arrays.iter().enumerate() {
            let destination = match (definition.width, definition.writable) {
                (8, false) => &mut nodes,
                (4, true) => &mut bins,
                _ => continue,
            };
            if let (Some(pointer_word), Some(length_word)) =
                (word_at(proof.view(array)), word_at(proof.view(array) + 1))
            {
                destination.push(View {
                    array,
                    pointer_word,
                    length_word,
                    width: definition.width,
                    literal: None,
                });
            }
        }
        if !nodes.is_empty() && !bins.is_empty() {
            let mut grouped = module.make_context();
            grouped.func = context.func.clone();
            if let Some(admission) = cfg_cleanup::grouped_histogram::optimize(
                &mut grouped,
                word_loops.machine_header,
                &loads,
                &nodes,
                &bins,
            ) {
                let name = format!("{symbol}_grouped");
                let grouped_id = module
                    .declare_function(&name, Linkage::Local, &signature)
                    .map_err(backend)?;
                grouped.verify(module.isa()).map_err(backend)?;
                let inspection = std::env::var_os("MECH_INSPECT_PHASE_CLIF")
                    .map(std::path::PathBuf::from)
                    .filter(|path| path.file_stem().is_some_and(|stem| stem == name.as_str()));
                if let Some(path) = &inspection {
                    std::fs::write(path, grouped.func.display().to_string()).map_err(backend)?;
                    grouped.set_disasm(true);
                }
                let (grouped_id, target) = define_body(
                    module,
                    grouped_id,
                    &name,
                    &mut grouped,
                    bodies.as_deref_mut(),
                )?;
                let machine = grouped
                    .compiled_code()
                    .ok_or_else(|| backend("grouped phase lacks code"))?;
                if let Some(path) = inspection {
                    let vcode = machine
                        .vcode
                        .as_ref()
                        .ok_or_else(|| backend("grouped phase VCode was not emitted"))?;
                    std::fs::write(path.with_extension("vcode"), vcode).map_err(backend)?;
                }
                histogram = Some(serde_json::json!({
                    "symbol": target,
                    "admission": admission,
                    "code_bytes": machine.code_info().total_size,
                    "stack_bytes": compiled_stack_bytes(&machine.buffer)?,
                }));
                let fast = module.declare_func_in_func(grouped_id, &mut context.func);
                cfg_cleanup::grouped_histogram::guard_entry(&mut context, admission, fast);
                context.verify(module.isa()).map_err(backend)?;
            }
        }
    }
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
            let mut composed_previous = phase_cleanup.append_previous.clone();
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
                let mut composed_header = capture.header;
                // Select the countdown only for this independently admitted
                // immutable lookup variant. Re-emit from the same numeric
                // proof and verify its own composition before substitution.
                if admission.immutable_code_bytes.is_some()
                    && !word_loops.reduction_completions.is_empty()
                    && std::env::var_os("MECH_REDUCTION_COUNTDOWN").is_some()
                {
                    let (mut fast, fast_report) = phase_ir(
                        module,
                        code,
                        region,
                        &signature,
                        &controls,
                        observe,
                        vectors,
                        private,
                        no_growth,
                        selected_output,
                        true,
                        error_origin_base,
                    )?;
                    let fast_cleanup = optimize_phase(
                        &mut fast,
                        &controls,
                        module.isa(),
                        &fast_report,
                        &symbol,
                        input_words,
                        output_words,
                        vectors,
                        private.is_some(),
                        no_growth,
                        false,
                    )?;
                    if let Some(loads) = phase_storage::input_word_loads(&fast.func, input_words) {
                        let fast_capture = Capture {
                            packet_loads: &loads,
                            header: fast_report.machine_header,
                            ..capture
                        };
                        if let Some(fast_admission) = cfg_cleanup::compose_readonly_lookups(
                            &controls,
                            &mut fast,
                            module,
                            dense_tables,
                            &fast_capture,
                        )? && fast_admission == admission
                        {
                            composed_header = fast_capture.header;
                            candidate = fast;
                            composed_previous = fast_cleanup.append_previous;
                        }
                    }
                }
                let name = format!("{symbol}_composed");
                if admission.immutable_code_bytes.is_some()
                    && std::env::var_os("MECH_FORWARD_ZERO_TRIP_REDUCTIONS").is_some()
                {
                    candidate.flowgraph();
                    if cfg_cleanup::identity_routes::zero_trip::fold(
                        &mut candidate.func,
                        &candidate.cfg,
                        &candidate.domtree,
                    ) {
                        candidate.flowgraph();
                        candidate.verify(module.isa()).map_err(backend)?;
                        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                            eprintln!("{name} forwarded zero-trip reduction");
                        }
                    }
                }
                if admission.immutable_code_bytes.is_some()
                    && module.isa().name() == "aarch64"
                    && module.isa().endianness() == cranelift_codegen::ir::Endianness::Little
                    && std::env::var_os("MECH_NORMALIZE_DEFAULTED_READ_SETUP").is_some()
                    && proof.arrays[contract.array].width == 8
                    && let Some(output_base) = word_at(proof.view(contract.array))
                    && let Some(loads) =
                        phase_storage::input_word_loads(&candidate.func, input_words)
                {
                    candidate.flowgraph();
                    let capture = Capture {
                        packet_loads: &loads,
                        header: composed_header,
                        ..capture
                    };
                    let changes = cfg_cleanup::defaulted_read_setup::optimize(
                        &mut candidate.func,
                        &candidate.cfg,
                        &candidate.domtree,
                        &capture,
                        admission.cursor_word,
                        output_base,
                    );
                    candidate.flowgraph();
                    candidate.verify(module.isa()).map_err(backend)?;
                    if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                        eprintln!("{name} defaulted read setup: {changes:?}");
                    }
                }
                if admission.immutable_code_bytes.is_some()
                    && std::env::var_os("MECH_CARRY_REDUCTION_BOUND").is_some()
                {
                    candidate.flowgraph();
                    if cfg_cleanup::reduction_bound::carry(
                        &mut candidate.func,
                        &candidate.cfg,
                        &candidate.domtree,
                    ) {
                        candidate.verify(module.isa()).map_err(backend)?;
                        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                            eprintln!("{name} carries an adjusted reduction bound");
                        }
                    }
                }
                if admission.immutable_code_bytes.is_some()
                    && std::env::var_os("MECH_CARRY_REDUCTION_BOUND").is_some()
                {
                    candidate.flowgraph();
                    if cfg_cleanup::reduction_rotation::rotate(
                        &mut candidate.func,
                        &candidate.cfg,
                        &candidate.domtree,
                    ) {
                        candidate.flowgraph();
                        candidate.verify(module.isa()).map_err(backend)?;
                        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                            eprintln!("{name} rotated nested reduction");
                        }
                    }
                }
                if admission.immutable_code_bytes.is_some()
                    && module.isa().name() == "aarch64"
                    && std::env::var_os("MECH_COMPOSE_SPLIT_COST_LENGTH").is_some()
                    && let Some(loads) =
                        phase_storage::input_word_loads(&candidate.func, input_words)
                {
                    candidate.flowgraph();
                    let capture = Capture {
                        packet_loads: &loads,
                        header: composed_header,
                        ..capture
                    };
                    let split = cfg_cleanup::defaulted_read_setup::split_admitted_cost_checks(
                        &mut candidate.func,
                        &candidate.cfg,
                        &capture,
                        admission,
                    );
                    if split != 0 {
                        candidate.flowgraph();
                        candidate.verify(module.isa()).map_err(backend)?;
                        eprintln!("{name} split admitted cost checks: {split}");
                    }
                }
                if module.isa().endianness() == cranelift_codegen::ir::Endianness::Little
                    && let Some(previous) = &composed_previous
                    && let Some(output_word) = word_at(proof.view(contract.array))
                    && let Some(length_word) = word_at(proof.view(contract.array) + 1)
                    && let Some(loads) =
                        phase_storage::input_word_loads(&candidate.func, input_words)
                {
                    candidate.flowgraph();
                    let forwarded =
                        previous.apply(&mut candidate, &loads, output_word, length_word);
                    if forwarded.is_ok()
                        && std::env::var_os("MECH_FORWARD_PREVIOUS_CLEANUP").is_some()
                    {
                        cfg_cleanup::coalesce_equal_join_arguments(
                            &controls,
                            &mut candidate,
                            module.isa(),
                        )?;
                        cfg_cleanup::coalesce_parallel_join_arguments(
                            &controls,
                            &mut candidate,
                            module.isa(),
                        )?;
                        cfg_cleanup::optimize_final_ordered(
                            &controls,
                            &mut candidate,
                            module.isa(),
                        )?;
                    }
                    candidate.verify(module.isa()).map_err(backend)?;
                    if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some()
                        || std::env::var_os("MECH_APPEND_FORWARD_REPORT").is_some()
                    {
                        eprintln!("{name} late previous append forwarding: {forwarded:?}");
                    }
                }
                if admission.immutable_code_bytes.is_some()
                    && std::env::var_os("MECH_FOLD_NARROW_PACKED_WORDS").is_some()
                {
                    // A low-word cost must not retain an independently packed
                    // high field. Full item/store uses keep the original word.
                    cfg_cleanup::narrow_bits::fold(&mut candidate.func);
                    if std::env::var("MECH_FOLD_NARROW_PACKED_WORDS").as_deref() == Ok("phis") {
                        cfg_cleanup::narrow_bits::fold_parallel(&mut candidate.func);
                    }
                    candidate.verify(module.isa()).map_err(backend)?;
                }
                let id = module
                    .declare_function(&name, Linkage::Local, &signature)
                    .map_err(backend)?;
                if !observe && module.isa().name() == "aarch64" {
                    candidate.func.aarch64_branch_on_zero =
                        admission.immutable_code_bytes.is_some()
                            && std::env::var_os("MECH_COMPACT_DENSE_BRANCH_ON_ZERO").is_some();
                    // Numeric phases start with their own Function. The
                    // enclosing checked entry's register preference does not
                    // reach this independently admitted immutable-cost body.
                    if admission.immutable_code_bytes.is_some()
                        && std::env::var_os("MECH_ORDERED_PREFER_PRESERVED_REGS").is_some()
                    {
                        candidate.func.aarch64_prefer_preserved_regs = true;
                    }
                    phase_layout::apply(&mut candidate, &name).map_err(backend)?;
                }
                let inspect_path = std::env::var_os("MECH_INSPECT_PHASE_CLIF")
                    .map(std::path::PathBuf::from)
                    .filter(|path| path.file_stem().is_some_and(|stem| stem == name.as_str()));
                if let Some(path) = &inspect_path {
                    std::fs::write(path, candidate.func.display().to_string()).map_err(backend)?;
                    candidate.set_disasm(true);
                }
                let (id, target) =
                    define_body(module, id, &name, &mut candidate, bodies.as_deref_mut())?;
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
                    summary: serde_json::json!({"symbol":target,"admission":admission,
                        "code_bytes":machine.code_info().total_size,
                        "stack_bytes":compiled_stack_bytes(&machine.buffer)?,
                        "scratch_bytes":if admission.immutable_code_bytes.is_some() { 0 } else { admission.cells_per_bank * 8 }}),
                });
            }
        }
    }
    let packed_halfword_stores = if pack_halfword_stores.is_some()
        && let Some(stores) = paired_halfword_stores
    {
        context.compute_cfg();
        let packed = stores.fold(&mut context.func, &context.cfg);
        if packed != 0 {
            context.verify(module.isa()).map_err(backend)?;
        }
        packed
    } else {
        0
    };
    let extrema = std::env::var_os("MECH_PRIVATE_VECTOR_PROVE_UNSIGNED_EXTREMA").is_some();
    let unsigned_guards = if !observe
        && cleanup.enabled
        && private.is_some()
        && vectors.is_some()
        && controls.enabled(OptimizationPass::UnsignedGuardFacts)
        && (extrema || std::env::var_os("MECH_PRIVATE_VECTOR_UNSIGNED_GUARDS").is_some())
    {
        let removed = unsigned_guard_facts::optimize_with_ranges(
            &mut context,
            &[],
            controls.enabled(OptimizationPass::ModularUnsignedRanges),
            extrema,
        );
        if removed != 0 {
            let isa = module.isa();
            context
                .prune_unreachable_and_remove_constant_phis(isa)
                .map_err(backend)?;
            context.verify(isa).map_err(backend)?;
        }
        removed
    } else {
        0
    };
    if !observe
        && vectors.is_some()
        && prepared.metadata().parallel.regions.is_empty()
        && std::env::var_os("MECH_REUSE_COMPARED_BYTES").is_some()
        && let Some(private) = private.filter(|contract| contract.permits(proof))
        && let Some(loads) = phase_storage::input_word_loads(&context.func, input_words)
    {
        // Only current fixed immutable byte views from this sealed caller
        // qualify. Later defaults remain unless a current source read has
        // already proved their exact logical bounds.
        let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
        let views = private
            .immutable_fixed_arrays(proof)
            .into_iter()
            .filter(|&array| proof.arrays[array].width == 1)
            .filter_map(|array| {
                Some((
                    array,
                    (
                        packet.iter().position(|&word| word == proof.view(array))?,
                        packet
                            .iter()
                            .position(|&word| word == proof.view(array) + 1)?,
                    ),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        let pointers = loads
            .iter()
            .filter_map(|(&value, &(_, word))| {
                views
                    .values()
                    .any(|&(pointer, _)| pointer == word)
                    .then_some(value)
            })
            .collect();
        let current_views = views
            .into_iter()
            .filter_map(|(array, (pointer, length))| {
                let word = |at| {
                    loads
                        .iter()
                        .find_map(|(&value, &(_, word))| (word == at).then_some(value))
                };
                Some((array, (word(pointer)?, word(length)?)))
            })
            .collect();
        let reused = cfg_cleanup::compared_bytes::fold(
            &mut context,
            &pointers,
            &report.bounded_byte_reads,
            &current_views,
        );
        if reused != 0 {
            context.verify(module.isa()).map_err(backend)?;
            eprintln!("{symbol} reused compared byte pairs: {reused}");
        }
    }
    if !observe
        && let Some(private) = private.filter(|contract| contract.permits(proof))
        && vectors.is_some()
        && std::env::var_os("MECH_COMPACT_IMMUTABLE_DEFAULTED_READS")
            .is_some_and(|target| target == "all" || target == symbol.as_str())
        && let Some(loads) = phase_storage::input_word_loads(&context.func, input_words)
    {
        let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
        let views = private
            .immutable_fixed_arrays(proof)
            .into_iter()
            .filter_map(|array| {
                let pointer_word = packet.iter().position(|&word| word == proof.view(array))?;
                let length_word = packet
                    .iter()
                    .position(|&word| word == proof.view(array) + 1)?;
                Some(View {
                    array,
                    pointer_word,
                    length_word,
                    width: proof.arrays[array].width,
                    literal: None,
                })
            })
            .collect::<Vec<_>>();
        let rematerialize =
            std::env::var_os("MECH_COMPACT_REMATERIALIZE_IMMUTABLE_READS").is_some();
        let (folded, immutable) = cfg_cleanup::defaulted_loads::fold_immutable(
            module,
            &mut context,
            &loads,
            &views,
            rematerialize,
        )?;
        eprintln!("{symbol} defaulted loads folded: {folded}, immutable: {immutable}");
    } else if !observe
        && private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_COMPACT_SELECT_PHASE_DEFAULTED_LOADS")
            .is_some_and(|target| target == "all" || target == symbol.as_str())
    {
        let folded = cfg_cleanup::defaulted_loads::fold(module, &mut context)?;
        eprintln!("{symbol} defaulted loads folded: {folded}");
    }
    if !observe
        && private.is_some()
        && vectors.is_some()
        && controls.enabled(OptimizationPass::ScalarSelects)
        && std::env::var_os("MECH_COMPACT_PHASE_SELECTS")
            .is_some_and(|target| target == "all" || target == symbol.as_str())
    {
        let selected =
            crate::native_artifact::codegen::common::scalar_selects::optimize(&mut context.func);
        if selected != 0 {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
        }
        eprintln!("{symbol} scalar choices folded: {selected}");
    }
    // Preserve default-read recognition and its readonly load flags before
    // folding the now-admitted address predicates.
    if !observe
        && cleanup.enabled
        && no_growth.is_none()
        && vectors.is_some_and(|contract| contract.extend_spans)
        && private.is_some_and(|contract| contract.permits(proof))
        && let Some(loads) = phase_storage::input_word_loads(&context.func, input_words)
    {
        let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
        let lengths = proof
            .arrays
            .iter()
            .enumerate()
            .filter_map(|(array, view)| {
                (!view.writable && !view.appends && view.width == 1)
                    .then(|| packet.binary_search(&(proof.view(array) + 1)).ok())
                    .flatten()
            })
            .collect();
        entry_spans = cfg_cleanup::admitted_spans::fold(
            &mut context,
            &loads,
            &lengths,
            std::env::var_os("MECH_REUSE_ADMITTED_INPUT_WINDOWS").is_some(),
        );
        if std::env::var_os("MECH_REUSE_ADMITTED_INPUT_WORD_LENGTHS").is_some() {
            cfg_cleanup::admitted_spans::admit_word_length(
                &mut context.func,
                &loads,
                &mut entry_spans,
            );
        }
        if !entry_spans.is_empty() {
            if entry_spans
                .iter()
                .any(|span| span.window_checks_removed != 0)
            {
                context
                    .compute_flowgraph_and_cleanup(module.isa())
                    .map_err(backend)?;
            } else {
                context.flowgraph();
            }
            context.verify(module.isa()).map_err(backend)?;
        }
    }
    if !observe
        && private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_COMPACT_SKIP_EMPTY_JUMPS")
            .is_some_and(|scope| scope == "vector" || scope == "all")
    {
        let count = cfg_cleanup::empty_jumps::forward(&mut context.func);
        if count != 0 {
            let isa = module.isa();
            context.flowgraph();
            context.eliminate_unreachable_code(isa).map_err(backend)?;
            context.verify(isa).map_err(backend)?;
        }
        eprintln!("{symbol} forwarded empty jumps: {count}");
    }
    if !observe
        && private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_COMPACT_FORWARD_ARGUMENT_JUMPS")
            .is_some_and(|scope| scope == "vector" || scope == "all")
    {
        let count = cfg_cleanup::argument_jumps::forward(&mut context.func);
        if count != 0 {
            let isa = module.isa();
            context.flowgraph();
            context.eliminate_unreachable_code(isa).map_err(backend)?;
            context.verify(isa).map_err(backend)?;
        }
        eprintln!("{symbol} forwarded argument jumps: {count}");
    }
    let packed_input = if pack_input {
        phase_storage::pack_used_input_words(&mut context.func, input_words)
    } else {
        None
    };
    if let Some(path) = std::env::var_os("MECH_INSPECT_PHASE_CLIF") {
        let path = std::path::PathBuf::from(path);
        if path.file_stem().is_some_and(|name| name == symbol.as_str()) {
            std::fs::write(&path, context.func.display().to_string()).map_err(backend)?;
            std::fs::write(path.with_extension("proof.txt"), format!("{proof:#?}"))
                .map_err(backend)?;
        }
    }
    let vcode_path = std::env::var_os("MECH_INSPECT_PHASE_VCODE")
        // Keep register-profile capture on the normal refused-general and
        // preparation route. The older VCode probe observes the general IR.
        .or_else(|| std::env::var_os("MECH_INSPECT_PHASE_REGISTER_PROFILE"))
        .map(std::path::PathBuf::from)
        .filter(|path| path.file_stem().is_some_and(|name| name == symbol.as_str()));
    if vcode_path.is_some() {
        context.set_disasm(true);
    }
    let checked_regions = if !observe
        && let Ok(manifest) = std::env::var("MECH_COMPACT_PHASE_CONTROLLER_WINDOWS")
        && let Some(windows) = manifest
            .strip_prefix(&symbol)
            .and_then(|prefix| prefix.strip_prefix('@'))
    {
        let (base, windows) = windows
            .split_once('@')
            .ok_or_else(|| backend("expected phase region identifier base"))?;
        let families = checked_region_discovery::phase_candidates(
            &context.func,
            windows,
            base.parse().map_err(backend)?,
        )?;
        let checked = checked_traversals::share_phase_regions(
            module,
            &mut context.func,
            families,
            checked_traversals::CaptureBindings::from_environment(),
        )?;
        if checked.is_some() {
            context.flowgraph();
            context.verify(module.isa()).map_err(backend)?;
        }
        checked
    } else {
        None
    };
    context.func.forward_spill_reloads = !observe
        && private.is_some()
        && vectors.is_some()
        && std::env::var_os("MECH_COMPACT_FORWARD_PHASE_SPILLS")
            .is_some_and(|phase| phase == symbol.as_str());
    if !observe && vectors.is_some() && module.isa().name() == "aarch64" {
        phase_layout::apply(&mut context, &symbol).map_err(backend)?;
    }
    prepare_body(module.isa(), &mut context)?;
    if !observe && vectors.is_some() {
        phase_layout::apply_register_weights(&mut context, &symbol).map_err(backend)?;
    }
    // Inspect the actual prepared body without requesting the general observed
    // pipeline or changing disassembly, compilation sharing, or admission.
    if let Some(path) = std::env::var_os("MECH_INSPECT_OPTIMIZED_PHASE_CLIF")
        .map(std::path::PathBuf::from)
        .filter(|path| path.file_stem().is_some_and(|name| name == symbol.as_str()))
    {
        std::fs::write(path, context.func.display().to_string()).map_err(backend)?;
    }
    let entry_bounds = entry_bounds(proof, &report.phase_entry_bounds);
    Ok(General::Ready(finish_preparation(
        context,
        &report,
        phase_cleanup,
        SummaryInput {
            function,
            entry_bounds,
            header,
            exit: region.tail.exit(),
            symbol,
            input_words,
            output_words,
            fixed_count,
            actual_unit_append_growth: no_growth.is_none(),
            checked_regions,
            selected_output_words: selected_output.map(BTreeSet::len),
            packed_input,
            unsigned_guards,
            packed_halfword_stores,
            admitted_index_guards,
            entry_spans,
        },
        future_write_hints,
        histogram,
        bounded_histogram,
        composed,
        vcode_path,
    )))
}

pub(super) struct SummaryInput {
    function: FuncId,
    entry_bounds: Vec<(usize, u64, bool)>,
    header: usize,
    exit: u32,
    symbol: String,
    input_words: usize,
    output_words: usize,
    fixed_count: usize,
    actual_unit_append_growth: bool,
    checked_regions: Option<serde_json::Value>,
    selected_output_words: Option<usize>,
    packed_input: Option<Vec<usize>>,
    unsigned_guards: usize,
    packed_halfword_stores: usize,
    admitted_index_guards: usize,
    entry_spans: Vec<cfg_cleanup::admitted_spans::Admission>,
}

fn finish_preparation(
    context: Context,
    report: &numeric_compact::Report,
    phase_cleanup: PhaseCleanup,
    input: SummaryInput,
    future_write_hints: Vec<cfg_cleanup::future_store::Hint>,
    histogram: Option<serde_json::Value>,
    bounded_histogram: Option<serde_json::Value>,
    composed: Option<Composed>,
    vcode_path: Option<std::path::PathBuf>,
) -> Prepared {
    let SummaryInput {
        function,
        entry_bounds,
        header,
        exit,
        symbol,
        input_words,
        output_words,
        fixed_count,
        actual_unit_append_growth,
        checked_regions,
        selected_output_words,
        packed_input,
        unsigned_guards,
        packed_halfword_stores,
        admitted_index_guards,
        entry_spans,
    } = input;
    let PhaseCleanup {
        bit_counts,
        cleanup,
        equality,
        threaded,
        vector_predicates,
        width_guards,
        window_bounds,
        append_previous: _,
    } = phase_cleanup;
    let pack_sites = report.read_pack_sites.clone();
    let read_guards = report.proven_scalar_read_guards.clone();
    let write_guards = report.proven_scalar_write_guards.clone();
    let reused_reads = report.reused_scalar_reads.clone();
    let reused_operands = report.reused_scalar_operands.clone();
    let changed_writes = report.proven_changed_writes.clone();
    let private_writes = report.private_write_sites.clone();
    let span_summary = (!entry_spans.is_empty()).then(|| serde_json::json!(entry_spans));
    let finish_report: FinishReport = Box::new(move |compiled, entry_bounds| {
        let mut summary = serde_json::json!({"read_pack_sites":pack_sites,"wider_equal_loops":equality.wider,"private_write_sites":private_writes,"proven_changed_writes":changed_writes,"phase_entry_bounds":entry_bounds,"header":header,"exit":exit,"symbol":symbol,
        "code_bytes":compiled.code_info().total_size,"stack_bytes":compiled_stack_bytes(&compiled.buffer)?,
        "input_words":input_words,"output_words":output_words,
        "fixed_writable_capacities":fixed_count,"fixed_length_entry_gate":fixed_count != 0,
        "actual_unit_append_growth":actual_unit_append_growth,"unique_writable_owner_gate":true,"cleanup":cleanup});
        if admitted_index_guards != 0 {
            summary["admitted_masked_index_guards"] = serde_json::json!(admitted_index_guards);
        }
        if let Some(checked_regions) = checked_regions {
            summary["checked_regions"] = checked_regions;
        }
        if let Some(words) = selected_output_words {
            summary["selected_success_words"] = serde_json::json!(words);
        }
        if unsigned_guards != 0 {
            summary["unsigned_guards"] = serde_json::json!(unsigned_guards);
        }
        if packed_halfword_stores != 0 {
            summary["packed_halfword_stores"] = serde_json::json!(packed_halfword_stores);
        }
        summary["bit_counts"] = serde_json::json!(bit_counts);
        if let Some(hint) = future_write_hints.first() {
            summary["future_write_hint"] = serde_json::json!(hint);
        }
        if let Some(hint) = future_write_hints.get(1) {
            summary["second_future_write_hint"] = serde_json::json!(hint);
        }
        if let Some(report) = vector_predicates {
            summary["private_vector_predicates"] = serde_json::json!(report);
        }
        if width_guards != 0 {
            summary["proved_width_guards"] = serde_json::json!(width_guards);
        }
        if window_bounds != 0 {
            summary["shared_window_bounds"] = serde_json::json!(window_bounds);
        }
        if !equality.mismatch_forwarded.is_empty() {
            summary["forwarded_vector_mismatches"] = serde_json::json!(equality.mismatch_forwarded);
        }
        if !equality.word_probed.is_empty() {
            summary["first_word_equality_probes"] = serde_json::json!(equality.word_probed);
        }
        if !equality.word_reused.is_empty() {
            summary["word_reuse_loops"] = serde_json::json!(equality.word_reused);
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
        if let Some(histogram) = histogram {
            summary["grouped_histogram"] = histogram;
        }
        if let Some(histogram) = bounded_histogram {
            summary["bounded_byte_histogram"] = histogram;
        }
        if let Some(spans) = span_summary {
            summary["admitted_input_spans"] = spans;
        }

        Ok(summary)
    });
    Prepared {
        context,
        details: Details {
            function,
            entry_bounds,
            entry_spans,
            finish_report,
            composed,
            packed_input,
            vcode_path,
        },
    }
}

#[cfg(all(test, target_arch = "aarch64"))]
#[path = "phase_layout_tests.rs"]
mod phase_layout_tests;

fn phase_ir<M: Module>(
    module: &mut M,
    code: CodePlan<'_>,
    region: &crate::native_artifact::program::prepare::tail::Region,
    signature: &Signature,
    controls: &CompilationControls,
    observe: bool,
    vectors: Option<&VectorContract>,
    private: Option<&PrivatePhase>,
    no_growth: Option<&NoGrowth>,
    selected_output: Option<&BTreeSet<usize>>,
    countdown: bool,
    error_origin_base: Option<usize>,
) -> Result<(Context, numeric_compact::Report), NativeCompileError> {
    let report;
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
        )?
        .with_selected_output(selected_output);
        let native = if let Some(base) = error_origin_base {
            native.with_error_origin_base(base)
        } else {
            native
        };
        report = numeric_compact::emit(
            controls, &mut b, code, region, &native, vectors, private, no_growth, done, error,
            countdown,
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
    Ok((context, report))
}

pub(super) struct PhaseCleanup {
    bit_counts: usize,
    cleanup: CfgCleanup,
    equality: wide_equal::Outcome,
    threaded: Option<serde_json::Value>,
    vector_predicates: Option<serde_json::Value>,
    width_guards: usize,
    window_bounds: usize,
    append_previous: Option<crate::native_artifact::program::codegen::append_run::Previous>,
}

fn optimize_phase(
    context: &mut Context,
    controls: &CompilationControls,
    isa: &dyn TargetIsa,
    report: &numeric_compact::Report,
    symbol: &str,
    input_words: usize,
    output_words: usize,
    vectors: Option<&VectorContract>,
    private: bool,
    no_growth: Option<&NoGrowth>,
    report_diagnostics: bool,
) -> Result<PhaseCleanup, NativeCompileError> {
    if std::env::var_os("MECH_QUALIFY_PHASE_INPUT_PACKET").is_some_and(|selection| {
        // An exact phase list permits held-feature experiments without changing
        // the existing all-phase option or the input packet's proof contract.
        selection
            .to_str()
            .and_then(|value| value.strip_prefix("only:"))
            .is_none_or(|symbols| symbols.split(',').any(|selected| selected == symbol))
    }) {
        super::phase_storage::qualify_input_packet_loads(&mut context.func, input_words);
    }
    // Numeric phases have their own sealed SSA; never match through packets.
    let mut bit_counts =
        crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
    let cleanup = cfg_cleanup::optimize(controls, context, isa)?;
    let word_loops = report;
    // Resolve literal admission flags before allocating the already selected
    // capacity-proved reduction. Keep other native routines on the usual pass.
    if no_growth.is_some()
        && !word_loops.reduction_completions.is_empty()
        && cfg_cleanup::fold_bitwise_constants(controls, &mut context.func) != 0
    {
        cfg_cleanup::optimize(controls, context, isa)?;
    }
    let compact_equality = private
        && vectors.is_some()
        && std::env::var_os("MECH_COMPACT_SHORT_EQUALITY_PHASE")
            .is_some_and(|target| target == symbol);
    let equality = wide_equal::optimize(
        context,
        &word_loops.compact_word_loops,
        private && vectors.is_some() && std::env::var_os("MECH_REUSE_EQUALITY_PREFIX").is_some(),
        !compact_equality,
    );
    // Named experimental condition: growth state is absent from this callee.
    let threaded = if no_growth.is_some()
        && std::env::var_os("MECH_THREAD_AFTER_GROWTH_ISOLATION").is_some()
    {
        Some(cfg_cleanup::thread_loop_edges(controls, context, isa)?)
    } else {
        None
    };
    let mut append_previous = None;
    if no_growth.is_some() && std::env::var_os("MECH_CHECKED_APPEND_RUN").is_some() {
        let packet = std::env::var_os("MECH_SPLIT_APPEND_DESCRIPTORS")
            .map(|value| (input_words, value == "all"));
        let paired = report_diagnostics
            && isa.name() == "aarch64"
            && isa.endianness() == cranelift_codegen::ir::Endianness::Little
            && std::env::var_os("MECH_PAIR_CHECKED_APPENDS").is_some();
        let outcome = if std::env::var_os("MECH_FORWARD_PREVIOUS_APPEND").is_some()
            && let Some(contract) = no_growth
        {
            crate::native_artifact::program::codegen::append_run::optimize_with_previous(
                context,
                word_loops.machine_header,
                packet,
                paired,
                // The private constructor proved this exact append-only phase.
                // The numeric proof below additionally excludes indexed writes.
                // Unsupported recurrences retain the checked implementation.
                contract,
                &mut append_previous,
            )
        } else {
            crate::native_artifact::program::codegen::append_run::optimize(
                context,
                word_loops.machine_header,
                packet,
                paired,
            )
        };
        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} checked append run: {outcome:?}");
        }
    }
    let narrow_fixed = (private && vectors.is_some())
        .then(|| std::env::var_os("MECH_NARROW_PRIVATE_ARITHMETIC"))
        .flatten();
    if narrow_fixed.is_some()
        || (no_growth.is_some() && std::env::var_os("MECH_NARROW_MASKED_ARITHMETIC").is_some())
    {
        let narrowed =
            crate::native_artifact::program::codegen::low_word::narrow(&mut context.func);
        let shifts = if narrow_fixed.is_some_and(|mode| mode == "shifts") {
            crate::native_artifact::program::codegen::low_word::narrow_masked_shifts(
                &mut context.func,
            )
        } else {
            0
        };
        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} low-word operations: {narrowed} arithmetic, {shifts} shifts");
        }
    }
    // A completed ordered reduction names its own selected continuation.
    // Only those edges enter the existing acyclic forwarding proof.
    if no_growth.is_some() && !word_loops.reduction_completions.is_empty() {
        let report = cfg_cleanup::forward_identity_routes_from(
            controls,
            context,
            isa,
            Some(&word_loops.reduction_completions),
        )?;
        if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!(
                "{symbol} completed reduction routes: {}",
                serde_json::to_string(&report).unwrap()
            );
        }
    }
    let vector_predicates = if private
        && vectors.is_some()
        && std::env::var_os("MECH_PRIVATE_VECTOR_PREDICATES").is_some()
    {
        let report = cfg_cleanup::refine_current_predicates(controls, context, isa)?;
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
    let width_guards =
        if private && vectors.is_some() && std::env::var_os("MECH_PRIVATE_VECTOR_WIDTHS").is_some()
        {
            let removed = cfg_cleanup::fold_proved_widths(controls, context, isa)?;
            if removed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                eprintln!("{symbol} proved-width guards: {removed}");
            }
            removed
        } else {
            0
        };
    let window_bounds = if private && vectors.is_some() {
        cfg_cleanup::combine_window_bounds(controls, context, isa)?
    } else {
        0
    };
    let paired_spans = if private
        && vectors.is_some()
        && !equality.reused.is_empty()
        && std::env::var_os("MECH_ORDERED_CAPTURE_SPANS").is_some()
    {
        cfg_cleanup::fold_ordered_capture_spans(controls, context, isa)?
    } else {
        0
    };
    if paired_spans != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
        eprintln!("{symbol} ordered capture spans: {paired_spans}");
    }
    if private && vectors.is_some() && std::env::var_os("MECH_ADJACENT_ADDRESSES").is_some() {
        let changed = cfg_cleanup::reuse_adjacent_addresses(controls, context, isa)?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} adjacent typed accesses: {changed}");
        }
    }
    if std::env::var_os("MECH_SHARE_REPEATED_BOUNDS").is_some() {
        let changed = cfg_cleanup::share_repeated_bounds(controls, context, isa)?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} repeated bounds checks: {changed}");
        }
    }
    if private
        && vectors.is_some()
        && let Some(mode) = std::env::var_os("MECH_DEFER_EXIT_PACKET_WORDS")
    {
        let changed = super::phase_storage::sink_exit_packet_words(
            &mut context.func,
            input_words,
            output_words,
            mode == "multiple",
        );
        if changed != 0 {
            context.verify(isa).map_err(backend)?;
            if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                eprintln!("{symbol} deferred exit packet words: {changed}");
            }
        }
    }
    if private
        && vectors.is_some()
        && std::env::var_os("MECH_REUSE_DOMINATING_WINDOW_FACTS").is_some()
    {
        let changed = cfg_cleanup::reuse_dominating_window_facts(controls, context, isa)?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} reused dominating window facts: {changed}");
        }
    }
    if private && vectors.is_some() && std::env::var_os("MECH_PROVE_SCALAR_SPANS").is_some() {
        let changed = cfg_cleanup::prove_scalar_spans(controls, context, isa)?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} proved scalar spans: {changed}");
        }
    }
    if private
        && vectors.is_some()
        && let Some(mode) = std::env::var_os("MECH_PROVE_PACKED_BASE_SPANS")
    {
        let proved = cfg_cleanup::prove_packed_bases(controls, context, isa, mode == "offsets")?;
        if proved != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} proved packed base spans: {proved}");
        }
    }
    if private && vectors.is_some() && std::env::var_os("MECH_REUSE_SELECTED_READ_BOUND").is_some()
    {
        let changed = cfg_cleanup::reuse_selected_read_bounds(controls, context, isa)?;
        if changed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} reused selected readable bounds: {changed}");
        }
    }
    if private
        && vectors.is_some()
        && std::env::var_os("MECH_COALESCE_EQUAL_JOIN_ARGUMENTS").is_some()
    {
        let removed = cfg_cleanup::coalesce_equal_join_arguments(controls, context, isa)?;
        if removed != 0 && std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
            eprintln!("{symbol} merged equal block arguments: {removed}");
        }
    }
    if private
        && vectors.is_some()
        && cleanup.enabled
        && std::env::var_os("MECH_FORWARD_SELECTED_CONTINUATIONS").is_some()
    {
        context.compute_cfg();
        context.compute_domtree();
        let forwarded = cfg_cleanup::selected_continuations::optimize(
            &mut context.func,
            &context.cfg,
            &context.domtree,
        );
        if forwarded != 0 {
            context.compute_cfg();
            context.compute_domtree();
            if cfg!(target_arch = "aarch64")
                && std::env::var_os("MECH_NORMALIZE_SELECTED_LOOP_TEST").is_some()
            {
                context.eliminate_unreachable_code(isa).map_err(backend)?;
                context.compute_cfg();
                context.compute_domtree();
                let normalized = cfg_cleanup::selected_continuations::branch_form::optimize(
                    &mut context.func,
                    &context.domtree,
                );
                if normalized != 0 {
                    context.compute_cfg();
                    context.compute_domtree();
                    if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                        eprintln!("{symbol} normalized selected loop tests: {normalized}");
                    }
                }
            }
            context.verify(isa).map_err(backend)?;
            if std::env::var_os("MECH_INSPECT_PHASE_CLIF").is_some() {
                eprintln!("{symbol} forwarded selected continuations: {forwarded}");
            }
        }
    }
    // Late forwarding may expose a complete idiom without crossing a phase.
    bit_counts += crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
    let threaded = threaded
        .filter(|_| report_diagnostics)
        .map(|report| serde_json::json!(report));
    let vector_predicates = vector_predicates
        .filter(|report| report_diagnostics && report.removed_branches != 0)
        .map(|report| serde_json::json!(report));
    Ok(PhaseCleanup {
        bit_counts,
        cleanup,
        equality,
        threaded,
        vector_predicates,
        width_guards,
        window_bounds,
        append_previous,
    })
}
