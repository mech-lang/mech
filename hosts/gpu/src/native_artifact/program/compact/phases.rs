//! Existing same-Plan numeric phases through a compact concrete array boundary.
//! The generic source CFG remains the fallback when an actual writable owner
//! is shared. Real unit append growth uses the existing concrete array helper.
use super::{
    arrays::Arrays, immutable_arrays::LiteralViews, layout::Layout, owner_flow::OwnerFlow,
    phase_storage,
};
use crate::native_artifact::codegen::{
    common::ssa_words::Words,
    control::plan::CodePlan,
    options::{CompilationControls, OptimizationOptions},
};
use crate::native_artifact::program::{
    codegen::{cfg_cleanup, numeric_minimal},
    prepare::{PreparedNativeProgram, readonly_numeric::Operation},
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, compiled_stack_bytes, offset},
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, FuncRef, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::{BTreeMap, BTreeSet};
#[path = "phase_function.rs"]
mod function;
#[path = "private_phase.rs"]
mod private;
pub(in crate::native_artifact) use private::{HANDOFF as PRIVATE_HANDOFF, PrivatePhase};

// Only this caller constructs the contract and emits its current-view guards.
// General matrices retain the original native body, not a weakened index rule.
pub(in crate::native_artifact) struct VectorContract {
    guards: Vec<(usize, bool)>,
    // The caller tests these exact fixed-view lengths before either body.
    admitted_lengths: Vec<(usize, u64)>,
}
impl VectorContract {
    fn additional(
        proof: &crate::native_artifact::program::prepare::readonly_numeric::Proof,
        existing: &[(usize, u64, bool)],
    ) -> Option<Self> {
        let accessed = proof
            .actions
            .iter()
            .filter_map(|(_, action)| match action {
                Operation::Read { array, .. } | Operation::Write { array, .. } => Some(*array),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let guards = accessed
            .into_iter()
            .filter(|array| !existing.iter().any(|&(old, _, _)| old == *array))
            .map(|array| (array, proof.arrays[array].appends))
            .collect::<Vec<_>>();
        (!guards.is_empty()).then(|| Self {
            guards,
            admitted_lengths: existing
                .iter()
                .filter_map(|&(array, length, grows)| (!grows).then_some((array, length)))
                .collect(),
        })
    }
    pub(in crate::native_artifact) fn arrays(&self) -> impl Iterator<Item = usize> + '_ {
        self.guards.iter().map(|&(array, _)| array)
    }
}
struct VectorAlternative {
    function: FuncId,
    contract: VectorContract,
}

// Only the compact caller can construct this contract. Its current owner
// must pass the existing uniqueness gate before the additional capacity gate.
pub(in crate::native_artifact) struct NoGrowth {
    array: usize,
    countdown: usize,
}
impl NoGrowth {
    fn derive(
        prepared: &PreparedNativeProgram,
        region: &crate::native_artifact::program::prepare::tail::Region,
    ) -> Option<Self> {
        let proof = region.numeric.as_ref()?;
        let phase = proof.phase.as_ref()?;
        if !proof.exits.is_empty() || phase.checked_children || !phase.cold.is_empty() {
            return None;
        }
        let mut appends = proof.actions.iter().filter(|(_, a)| a.grows());
        let (Some((site, Operation::Append { array, .. })), None) =
            (appends.next(), appends.next())
        else {
            return None;
        };
        let array = *array;
        if proof.arrays.iter().enumerate().any(|(index, view)| {
            view.writable != (index == array) || view.appends != (index == array)
        }) || proof
            .actions
            .iter()
            .any(|(_, a)| matches!(a, Operation::Fallback))
        {
            return None;
        }
        let field = crate::native_artifact::program::prepare::numeric_phase::unit_append_countdown(
            prepared.plan(),
            &region.tail,
            &prepared.metadata().buffer_frames,
            &prepared.metadata().scalar_aggregates,
            *site,
        )?;
        let countdown = proof
            .fields(region.tail.carrier().get())?
            .get(field)
            .copied()
            .flatten()?;
        Some(Self { array, countdown })
    }

    pub(in crate::native_artifact) fn proves_append(&self, array: usize, maximum: usize) -> bool {
        self.array == array && maximum == 1
    }
}
struct NoGrowthAlternative {
    function: FuncId,
    array: usize,
    countdown_slot: usize,
    composed: Option<function::Composed>,
}

enum Output {
    Stack(usize),
    Word(usize),
}

pub(super) struct Phase {
    pub header: usize,
    pub exit: usize,
    function: FuncId,
    vector: Option<VectorAlternative>,
    no_growth: Option<NoGrowthAlternative>,
    inputs: Vec<usize>,
    arrays: Vec<(usize, bool)>,
    fixed_arrays: Vec<usize>,
    entry_bounds: Vec<(usize, u64, bool)>,
    invariant_views: BTreeSet<usize>,
    outputs: Vec<(usize, Output)>,
    output_owners: Vec<usize>,
    result_words: usize,
    summary: serde_json::Value,
    observe: bool,
    private: Option<PrivatePhase>,
}
impl Phase {
    pub fn summary(&self, owners: &OwnerFlow) -> serde_json::Value {
        let mut summary = self.summary.clone();
        if std::env::var_os("MECH_PHASE_OWNER_TRANSFER").is_some()
            || std::env::var_os("MECH_PHASE_OWNER_MOVE_CENSUS").is_some()
        {
            summary["exit_owner_transfers"] = serde_json::json!(self.owner_transfers(owners));
        }
        if !self.invariant_views.is_empty() {
            summary["invariant_input_views"] = serde_json::json!(self.invariant_views);
        }
        summary
    }
    fn owner_transfers(&self, owners: &OwnerFlow) -> Vec<(usize, usize)> {
        select_exit_owner_transfers(
            &self.outputs,
            &self.output_owners,
            &self.arrays,
            self.result_words,
            &owners.retire_bypass(self.header, self.exit, &self.output_owners),
        )
    }
    pub fn destinations(&self) -> Vec<usize> {
        std::iter::once(self.exit)
            .chain(
                self.private
                    .iter()
                    .flat_map(|p| p.handoffs.iter().map(|e| e.site)),
            )
            .collect()
    }
    pub fn packet_words(&self) -> (usize, usize) {
        let writable = self.arrays.iter().filter(|a| a.1).count();
        (
            self.inputs.len() + writable + self.fixed_arrays.len(),
            (1 + self.result_words + writable * 6)
                .max(self.private.as_ref().map_or(0, PrivatePhase::packet_words)),
        )
    }
    pub fn emit<M: Module>(
        &self,
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        storage: &Arrays,
        packets: &phase_storage::Slots,
        owners: &OwnerFlow,
        layout: &Layout,
        destination: Block,
        source_blocks: &[Option<Block>],
        error: Block,
        root_entry: bool,
    ) -> Result<(), NativeCompileError> {
        let observe = observer(module, b, self.observe)?;
        let fallback = b.create_block();
        for &(base, writable) in &self.arrays {
            if !writable {
                continue;
            }
            let owner = words.read(b, base + 5);
            let inspect = b.create_block();
            let next = b.create_block();
            b.ins().brif(owner, inspect, &[], fallback, &[]);
            b.switch_to_block(inspect);
            let count = b.ins().load(types::I64, MemFlags::new(), owner, 0);
            let unique = b.ins().icmp_imm(IntCC::Equal, count, 1);
            b.ins().brif(unique, next, &[], fallback, &[]);
            b.switch_to_block(next);
        }
        for &array in &self.fixed_arrays {
            let base = self.arrays[array].0;
            let length = words.read(b, base + 1);
            let capacity = words.read(b, base + 2);
            let fits = b
                .ins()
                .icmp(IntCC::UnsignedLessThanOrEqual, length, capacity);
            let next = b.create_block();
            b.ins().brif(fits, next, &[], fallback, &[]);
            b.switch_to_block(next);
        }
        // Refusal resumes the original root CFG before any phase action.
        // Length floors survive successful appends; pointers are never cached.
        for &(array, minimum, grows) in &self.entry_bounds {
            let base = self.arrays[array].0;
            let length = words.read(b, base + 1);
            let rows = words.read(b, base + 3);
            let columns = words.read(b, base + 4);
            let enough =
                b.ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, minimum as i64);
            let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
            let flat = if grows {
                row
            } else {
                let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
                b.ins().bor(row, column)
            };
            let valid = b.ins().band(enough, flat);
            let next = b.create_block();
            b.ins().brif(valid, next, &[], fallback, &[]);
            b.switch_to_block(next);
        }
        if let Some(observe) = observe.filter(|_| self.vector.is_none()) {
            let header = b.ins().iconst(types::I64, self.header as i64);
            let yes = b.ins().iconst(types::I64, 1);
            b.ins().call(observe, &[storage.heap, header, yes]);
        }
        let writable = self.arrays.iter().filter(|a| a.1).count();
        let (input_words, output_words) = self.packet_words();
        let (input, output) = packets.for_call(input_words, output_words)?;
        for (at, &slot) in self.inputs.iter().enumerate() {
            let value = words.read(b, slot);
            b.ins().stack_store(value, input, offset(at)?);
        }
        for (at, &(base, _)) in self.arrays.iter().filter(|a| a.1).enumerate() {
            let owner = words.read(b, base + 5);
            b.ins()
                .stack_store(owner, input, offset(self.inputs.len() + at)?);
        }
        for (at, &array) in self.fixed_arrays.iter().enumerate() {
            let capacity = words.read(b, self.arrays[array].0 + 2);
            b.ins()
                .stack_store(capacity, input, offset(self.inputs.len() + writable + at)?);
        }
        let prior = self
            .output_owners
            .iter()
            .filter(|&&slot| owners.live_at_entry(self.header, slot))
            .map(|&slot| words.read(b, slot))
            .collect::<Vec<_>>();
        let input = b.ins().stack_addr(types::I64, input, 0);
        let out = b.ins().stack_addr(types::I64, output, 0);
        let (status, at) = if let Some(no_growth) = &self.no_growth {
            let base = self.arrays[no_growth.array].0;
            let length = words.read(b, base + 1);
            let capacity = words.read(b, base + 2);
            let remaining = words.read(b, no_growth.countdown_slot);
            let valid = b
                .ins()
                .icmp(IntCC::UnsignedLessThanOrEqual, length, capacity);
            let available = b.ins().isub(capacity, length);
            let fits = b
                .ins()
                .icmp(IntCC::UnsignedLessThanOrEqual, remaining, available);
            let enough = b.ins().band(valid, fits);
            let fast = b.create_block();
            let general = b.create_block();
            let returned = b.create_block();
            b.append_block_param(returned, types::I64);
            b.append_block_param(returned, types::I64);
            b.ins().brif(enough, fast, &[], general, &[]);
            let mut routes = vec![(fast, no_growth.function), (general, self.function)];
            if root_entry && let Some(composed) = &no_growth.composed {
                let admission = composed.admission;
                b.switch_to_block(fast);
                let cursor = words.read(b, self.inputs[admission.cursor_word]);
                let cache_length = words.read(b, self.arrays[admission.cache].0 + 1);
                let cost_length = words.read(b, self.arrays[admission.costs].0 + 1);
                let nonnegative =
                    b.ins()
                        .icmp(IntCC::UnsignedGreaterThanOrEqual, cursor, remaining);
                let within = b
                    .ins()
                    .icmp(IntCC::UnsignedLessThanOrEqual, cursor, cache_length);
                let records = b.ins().isub(cursor, remaining);
                let work = b.ins().icmp_imm(
                    IntCC::UnsignedGreaterThanOrEqual,
                    records,
                    admission.minimum_records as i64,
                );
                let readable = b.ins().icmp_imm(
                    IntCC::UnsignedGreaterThanOrEqual,
                    cost_length,
                    admission.minimum_costs as i64,
                );
                let geometry = b.ins().band(nonnegative, within);
                let ready = b.ins().band(work, readable);
                let ready = b.ins().band(geometry, ready);
                let prepare = b.create_block();
                let sparse = b.create_block();
                b.ins().brif(ready, prepare, &[], sparse, &[]);
                routes[0].0 = sparse;
                routes.push((prepare, composed.function));
            }
            for (block, function) in routes {
                b.switch_to_block(block);
                let function = module.declare_func_in_func(function, b.func);
                let call = b.ins().call(function, &[storage.heap, input, out]);
                let result = b.inst_results(call);
                let args = [result[0].into(), result[1].into()];
                b.ins().jump(returned, &args);
            }
            b.switch_to_block(returned);
            let result = b.block_params(returned);
            (result[0], result[1])
        } else if let Some(vector) = &self.vector {
            let general = b.create_block();
            let returned = b.create_block();
            b.append_block_param(returned, types::I64);
            b.append_block_param(returned, types::I64);
            // Failed geometry selects the unchanged general native body. All
            // original owner/capacity/window gates above remain in force.
            for &(array, grows) in &vector.contract.guards {
                let base = self.arrays[array].0;
                let rows = words.read(b, base + 3);
                let columns = words.read(b, base + 4);
                let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
                let flat = if grows {
                    row
                } else {
                    let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
                    b.ins().bor(row, column)
                };
                let next = b.create_block();
                b.ins().brif(flat, next, &[], general, &[]);
                b.switch_to_block(next);
            }
            if let Some(observe) = observe {
                let header = b.ins().iconst(types::I64, self.header as i64);
                let yes = b.ins().iconst(types::I64, 2);
                b.ins().call(observe, &[storage.heap, header, yes]);
            }
            let function = module.declare_func_in_func(vector.function, b.func);
            let call = b.ins().call(function, &[storage.heap, input, out]);
            let result = b.inst_results(call).to_vec();
            b.ins()
                .jump(returned, &[result[0].into(), result[1].into()]);
            b.switch_to_block(general);
            if let Some(observe) = observe {
                let header = b.ins().iconst(types::I64, self.header as i64);
                let yes = b.ins().iconst(types::I64, 1);
                b.ins().call(observe, &[storage.heap, header, yes]);
            }
            let function = module.declare_func_in_func(self.function, b.func);
            let call = b.ins().call(function, &[storage.heap, input, out]);
            let result = b.inst_results(call).to_vec();
            b.ins()
                .jump(returned, &[result[0].into(), result[1].into()]);
            b.switch_to_block(returned);
            let result = b.block_params(returned);
            (result[0], result[1])
        } else {
            let function: FuncRef = module.declare_func_in_func(self.function, b.func);
            let call = b.ins().call(function, &[storage.heap, input, out]);
            let result = b.inst_results(call);
            (result[0], result[1])
        };
        if let Some(private) = &self.private
            && !private.handoffs.is_empty()
        {
            let dispatch = b.create_block();
            let normal = b.create_block();
            let cold = b
                .ins()
                .icmp_imm(IntCC::Equal, status, PRIVATE_HANDOFF as i64);
            b.ins().brif(cold, dispatch, &[], normal, &[]);
            b.switch_to_block(dispatch);
            for exit in &private.handoffs {
                let selected = b.create_block();
                let next = b.create_block();
                let matches = b.ins().icmp_imm(IntCC::Equal, at, exit.site as i64);
                b.ins().brif(matches, selected, &[], next, &[]);
                b.switch_to_block(selected);
                if let Some(observer) = observe {
                    let site = b.ins().iconst(types::I64, exit.site as i64);
                    let selected = b.ins().iconst(types::I64, 1);
                    b.ins().call(observer, &[storage.heap, site, selected]);
                }
                // Every source is captured before any overlapping root home is
                // replaced. The same fixed backing remains owned by the caller.
                let values = exit
                    .outputs
                    .iter()
                    .map(|(_, from)| match *from {
                        Output::Stack(at) => {
                            Ok(b.ins().stack_load(types::I64, output, offset(at)?))
                        }
                        Output::Word(at) => Ok(words.read(b, at)),
                    })
                    .collect::<Result<Vec<_>, NativeCompileError>>()?;
                let old = exit
                    .owners
                    .iter()
                    .filter(|&&slot| owners.live_at_entry(self.header, slot))
                    .map(|&slot| words.read(b, slot))
                    .collect::<Vec<_>>();
                for ((to, _), value) in exit.outputs.iter().zip(values) {
                    words.write(b, *to, value);
                }
                for &slot in &exit.owners {
                    let owner = words.read(b, slot);
                    storage.retain(b, owner);
                }
                for owner in old {
                    storage.release(b, owner, error);
                }
                storage.retire(
                    b,
                    words,
                    &owners.retire_bypass(self.header, exit.site, &exit.owners),
                    error,
                );
                b.ins().jump(
                    source_blocks[exit.site]
                        .ok_or_else(|| backend("private cold continuation is absent"))?,
                    &[],
                );
                b.switch_to_block(next);
            }
            // A callee can return only an exhaustively listed checked export.
            // Preserve a distinct error if the private ABI is ever corrupted.
            b.ins().return_(&[status, at]);
            b.switch_to_block(normal);
        }
        let failed = b.create_block();
        let success = b.create_block();
        b.set_cold_block(failed);
        b.ins().brif(status, failed, &[], success, &[]);
        b.switch_to_block(failed);
        // The two diagnostic layers use distinct internal bounds statuses.
        // Crossing this adapter preserves the raw source entry's vocabulary.
        let bounds = b
            .ins()
            .icmp_imm(IntCC::Equal, status, numeric_minimal::BOUNDS as i64);
        let raw_bounds = b.ins().iconst(types::I64, super::arrays::BOUNDS as i64);
        let status = b.ins().select(bounds, raw_bounds, status);
        // The callee reports its exact reached source location. No hot origin
        // home or header substitution participates in successful execution.
        b.ins().return_(&[status, at]);
        b.switch_to_block(success);
        // The synchronous callee transfers each current owner back to this
        // same carrier slot. It already settles replacements on real growth.
        for (ordinal, (array, &(base, _))) in self
            .arrays
            .iter()
            .enumerate()
            .filter(|(_, a)| a.1)
            .enumerate()
        {
            // The checked numeric proof excludes rebinding these arrays and
            // records every growing operation. Without append, the callee's
            // only mutations are element stores: its complete current view
            // remains the input view. Contents are never assumed unchanged.
            if self.invariant_views.contains(&array) {
                continue;
            }
            for lane in 0..6 {
                let value = b.ins().stack_load(
                    types::I64,
                    output,
                    offset(1 + self.result_words + ordinal * 6 + lane)?,
                );
                words.write(b, base + lane, value);
            }
        }
        if self.invariant_views.is_empty() {
            for (slot, source) in &self.outputs {
                let value = match *source {
                    Output::Stack(at) => b.ins().stack_load(types::I64, output, offset(at)?),
                    Output::Word(at) => words.read(b, at),
                };
                words.write(b, *slot, value);
            }
        } else {
            // Result publication is parallel transport. Capture all sources
            // before any target can overwrite an invariant source home.
            let values = self
                .outputs
                .iter()
                .map(|(_, source)| match *source {
                    Output::Stack(at) => Ok(b.ins().stack_load(types::I64, output, offset(at)?)),
                    Output::Word(at) => Ok(words.read(b, at)),
                })
                .collect::<Result<Vec<_>, NativeCompileError>>()?;
            for ((slot, _), value) in self.outputs.iter().zip(values) {
                words.write(b, *slot, value);
            }
        }
        // Count only the exact one-result/one-prior settlement class. No
        // ownership operation changes in this diagnostic. The disabled emitter
        // must reproduce its parent Object exactly before measurements.
        if std::env::var_os("MECH_PHASE_OWNER_CENSUS").is_some()
            && let ([slot], [old]) = (self.output_owners.as_slice(), prior.as_slice())
        {
            let mut signature = module.make_signature();
            signature.params = vec![AbiParam::new(types::I64); 3];
            signature.returns = vec![AbiParam::new(types::I64)];
            let id = module
                .declare_function(
                    "mech_compact_same_owner_observe",
                    Linkage::Import,
                    &signature,
                )
                .map_err(backend)?;
            let observer = module.declare_func_in_func(id, b.func);
            let owner = words.read(b, *slot);
            let equal = b.ins().icmp(IntCC::Equal, owner, *old);
            let present = b.ins().icmp_imm(IntCC::NotEqual, owner, 0);
            let equal = b.ins().uextend(types::I64, equal);
            let present = b.ins().uextend(types::I64, present);
            let present = b.ins().ishl_imm(present, 1);
            let category = b.ins().bor(equal, present);
            let header = b.ins().iconst(types::I64, self.header as i64);
            b.ins().call(observer, &[storage.heap, header, category]);
        }
        let exit_transfers = self.owner_transfers(owners);
        if std::env::var_os("MECH_PHASE_OWNER_MOVE_CENSUS").is_some() {
            let mut signature = module.make_signature();
            signature.params = vec![AbiParam::new(types::I64); 3];
            signature.returns = vec![AbiParam::new(types::I64)];
            let id = module
                .declare_function(
                    "mech_compact_owner_move_observe",
                    Linkage::Import,
                    &signature,
                )
                .map_err(backend)?;
            let observer = module.declare_func_in_func(id, b.func);
            for &(destination, source) in &exit_transfers {
                let result = words.read(b, destination);
                let held = words.read(b, source);
                let equal = b.ins().icmp(IntCC::Equal, result, held);
                let present = b.ins().icmp_imm(IntCC::NotEqual, result, 0);
                let equal = b.ins().uextend(types::I64, equal);
                let present = b.ins().uextend(types::I64, present);
                let present = b.ins().ishl_imm(present, 1);
                let category = b.ins().bor(equal, present);
                let header = b.ins().iconst(types::I64, self.header as i64);
                b.ins().call(observer, &[storage.heap, header, category]);
            }
        }
        let transfer = std::env::var_os("MECH_PHASE_OWNER_TRANSFER").is_some();
        for &slot in &self.output_owners {
            if transfer
                && let Some(&(_, source)) = exit_transfers
                    .iter()
                    .find(|&&(destination, _)| destination == slot)
            {
                // Every fallible source action and every result read succeeded.
                // The returned lane is exactly this current loan owner, and
                // the existing exit liveness retires its old home. Move one
                // token; shared backing still owns every other alias token.
                let zero = b.ins().iconst(types::I64, 0);
                words.write(b, source, zero);
            } else {
                let owner = words.read(b, slot);
                storage.retain(b, owner);
            }
        }
        for owner in prior {
            storage.release(b, owner, error);
        }
        storage.retire(
            b,
            words,
            &owners.retire_bypass(self.header, self.exit, &self.output_owners),
            error,
        );
        b.ins().jump(destination, &[]);
        b.switch_to_block(fallback);
        if let Some(observe) = observe {
            let header = b.ins().iconst(types::I64, self.header as i64);
            let no = b.ins().iconst(types::I64, 0);
            b.ins().call(observe, &[storage.heap, header, no]);
        }
        let _ = layout;
        Ok(())
    }
}
pub(super) fn define<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    layout: &Layout,
    literals: &LiteralViews,
    observe: bool,
) -> Result<Vec<Phase>, NativeCompileError> {
    let closed = &prepared.metadata().closed_tails;
    let mut result = vec![];
    let mut dense_tables = cfg_cleanup::composed_reads::DenseTables::default();
    let mut candidates = Vec::new();
    for (id, region) in closed.regions.iter().enumerate() {
        let Some(proof) = region.numeric.as_ref() else {
            continue;
        };
        let private = if std::env::var_os("MECH_PRIVATE_NESTED_PHASES").is_some() {
            PrivatePhase::derive(prepared, region, layout)?
        } else {
            None
        };
        let complete_readonly_frame = !proof.writes()
            && proof.exits.is_empty()
            && proof.result.frame().is_some()
            && !proof.arrays.iter().any(|a| a.appends);
        let computation = complete_readonly_frame
            || proof.phase.as_ref().map_or_else(
                || {
                    !proof.arrays.iter().any(|a| a.appends)
                        && proof
                            .actions
                            .iter()
                            .any(|(_, a)| matches!(a, Operation::Write { .. }))
                },
                |phase| phase.loops.len() > 1 && !phase.checked_children,
            );
        if ((proof.exits.is_empty() && computation) || private.is_some())
            && (proof.result.direct_array().is_some() || proof.result.frame().is_some())
            && !proof
                .actions
                .iter()
                .any(|(_, a)| matches!(a, Operation::AppendLeBytes { .. }))
        {
            candidates.push((id, private));
        }
    }
    candidates.sort_by_key(|(id, _)| {
        (
            std::cmp::Reverse(closed.regions[*id].numeric.as_ref().unwrap().sites.len()),
            closed.regions[*id].tail.header(),
        )
    });
    let mut covered = BTreeSet::new();
    for (id, private) in candidates {
        let region = &closed.regions[id];
        let header = region.tail.header() as usize;
        let proof = region.numeric.as_ref().unwrap();
        if proof.sites.iter().any(|site| covered.contains(site))
            || (region.cold_resume().is_some() && private.is_none())
        {
            continue;
        }
        let carrier = region.tail.carrier().get();
        let carrier_words = proof
            .frames
            .iter()
            .find(|f| f.value == carrier)
            .ok_or_else(|| backend("compact numeric phase lost its current carrier"))?;
        let mut bindings = BTreeMap::new();
        for (path, word) in &carrier_words.scalar_paths {
            let (offset, ty) = layout.ty(carrier).at_path(path)?;
            if ty.words != 1 {
                return Err(backend("compact numeric scalar carrier width differs"));
            }
            bindings.insert(*word, layout.slot(carrier) + offset);
        }
        let mut arrays = vec![];
        for (array, definition) in proof.arrays.iter().enumerate() {
            let (base, ty) = layout.array_field(definition, carrier)?;
            if ty.words != 6 || ty.element()?.packed_bytes != Some(definition.width.into()) {
                return Err(backend("compact numeric array physical element differs"));
            }
            for (lane, from) in [0, 1, 3, 4].into_iter().enumerate() {
                bindings.insert(proof.view(array) + lane, base + from);
            }
            if let Some(capacity) = proof.capacity(array) {
                bindings.insert(capacity, base + 2);
            }
            arrays.push((base, definition.writable));
        }
        let entry = numeric_minimal::input_words(proof, carrier)?;
        let fixed_arrays = proof
            .arrays
            .iter()
            .enumerate()
            .filter_map(|(a, d)| (d.writable && proof.capacity(a).is_none()).then_some(a))
            .collect::<Vec<_>>();
        let inputs = entry
            .iter()
            .map(|word| {
                bindings
                    .get(word)
                    .copied()
                    .ok_or_else(|| backend("compact numeric input lacks a current typed binding"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let result_words = numeric_minimal::output_words(proof)?.len();
        // Only the admitted same-Plan proof constructs this contract. Unknown
        // operations/rebinding already decline that proof. All append/bulk
        // append arrays keep the original refresh, even on zero-trip visits.
        let invariant_views = if std::env::var_os("MECH_PHASE_INVARIANT_VIEWS").is_some() {
            proof
                .arrays
                .iter()
                .enumerate()
                .filter_map(|(array, definition)| {
                    (definition.writable && !definition.appends).then_some(array)
                })
                .collect::<BTreeSet<_>>()
        } else {
            BTreeSet::new()
        };
        let target = region.tail.returned().get();
        let mut outputs = vec![];
        if let Some(array) = proof.result.direct_array() {
            if layout.ty(target).words != 6 {
                return Err(backend("compact numeric direct result width differs"));
            }
            array_outputs(
                &mut outputs,
                layout.slot(target),
                array,
                &arrays,
                result_words,
                &invariant_views,
            );
        } else {
            let frame = proof
                .frames
                .iter()
                .find(|f| f.value == target)
                .ok_or_else(|| backend("compact numeric result frame is absent"))?;
            let target_fields = layout.ty(target).leaves();
            if target_fields.len() != frame.arrays.len() {
                return Err(backend("compact numeric result leaves differ"));
            }
            let mut scalars = frame.scalar_paths.iter();
            let mut at = 1;
            for ((offset, ty), array) in target_fields.iter().zip(&frame.arrays) {
                if let Some(array) = array {
                    if ty.words != 6 {
                        return Err(backend("native result array width differs"));
                    }
                    array_outputs(
                        &mut outputs,
                        layout.slot(target) + offset,
                        *array,
                        &arrays,
                        result_words,
                        &invariant_views,
                    );
                    at += 4 + usize::from(proof.capacity(*array).is_some());
                } else {
                    let (path, _) = scalars
                        .next()
                        .ok_or_else(|| backend("native result lacks scalar path"))?;
                    let (scalar_offset, scalar_ty) = layout.ty(target).at_path(path)?;
                    if scalar_offset != *offset || scalar_ty.words != 1 {
                        return Err(backend("native scalar result path differs"));
                    }
                    outputs.push((layout.slot(target) + offset, Output::Stack(at)));
                    at += 1;
                }
            }
            if scalars.next().is_some() {
                return Err(backend("native result has extra scalar paths"));
            }
        }

        let output_owners = layout
            .ty(target)
            .owners
            .iter()
            .map(|&lane| layout.slot(target) + lane)
            .collect();
        let writable = arrays.iter().filter(|a| a.1).count();
        let general = function::define(
            module,
            prepared,
            id,
            observe,
            inputs.len() + writable + fixed_arrays.len(),
            (1 + result_words + writable * 6)
                .max(private.as_ref().map_or(0, PrivatePhase::packet_words)),
            fixed_arrays.len(),
            None,
            private.as_ref(),
            None,
            &BTreeMap::new(),
            &mut dense_tables,
            "",
        )?;
        let entry_bounds = general.entry_bounds.clone();
        let mut summary = general.summary;
        let mut vector = None;
        if std::env::var_os("MECH_PHASE_VECTOR_FAST").is_some()
            && let Some(contract) = VectorContract::additional(proof, &entry_bounds)
        {
            let fast = function::define(
                module,
                prepared,
                id,
                observe,
                inputs.len() + writable + fixed_arrays.len(),
                (1 + result_words + writable * 6)
                    .max(private.as_ref().map_or(0, PrivatePhase::packet_words)),
                fixed_arrays.len(),
                Some(&contract),
                private.as_ref(),
                None,
                &BTreeMap::new(),
                &mut dense_tables,
                "_vector",
            )?;
            let used = fast.code != general.code;
            let mut info = fast.summary;
            info["guard_arrays"] = serde_json::json!(contract.guards);
            info["selected_by_guard"] = serde_json::json!(used);
            summary["vector_variant"] = info;
            if used {
                vector = Some(VectorAlternative {
                    function: fast.function,
                    contract,
                });
            }
        }
        if let Some(private) = &private {
            summary["private_fixed_nested"] = serde_json::json!({
                "ordinary_funding_guard_unchanged":true,
                "unique_fixed_writable_gate":true,
                "loops":proof.phase.as_ref().unwrap().loops.iter().map(|l|l.header).collect::<Vec<_>>(),
                "cold_exports":private.handoffs.iter().map(|e|serde_json::json!({"site":e.site,"scalar_words":e.exports.len(),"owner_outputs":e.owners.len()})).collect::<Vec<_>>()
            });
        }
        let no_growth = if std::env::var_os("MECH_WHOLE_PHASE_CAPACITY").is_some()
            && vector.is_none()
            && private.is_none()
            && let Some(contract) = NoGrowth::derive(prepared, region)
            && entry_bounds
                .iter()
                .any(|&(array, _, row)| array == contract.array && row)
        {
            let literal_views = arrays
                .iter()
                .enumerate()
                .filter_map(|(array, &(base, writable))| {
                    if writable || proof.arrays[array].appends {
                        return None;
                    }
                    literals.literal(base).map(|literal| (array, literal))
                })
                .collect();
            let fast = function::define(
                module,
                prepared,
                id,
                observe,
                inputs.len() + writable + fixed_arrays.len(),
                1 + result_words + writable * 6,
                fixed_arrays.len(),
                None,
                None,
                Some(&contract),
                &literal_views,
                &mut dense_tables,
                "_no_growth",
            )?;
            if fast.entry_bounds != entry_bounds {
                return Err(backend("no-growth variant entry bounds differ"));
            }
            summary["no_growth_variant"] = fast.summary;
            if let Some(composed) = &fast.composed {
                summary["composed_variant"] = composed.summary.clone();
            }
            Some(NoGrowthAlternative {
                function: fast.function,
                array: contract.array,
                composed: fast.composed,
                countdown_slot: *bindings
                    .get(&contract.countdown)
                    .ok_or_else(|| backend("no-growth countdown lacks current binding"))?,
            })
        } else {
            None
        };
        result.push(Phase {
            header,
            exit: region.tail.exit() as usize,
            function: general.function,
            vector,
            no_growth,
            inputs,
            arrays,
            fixed_arrays,
            entry_bounds,
            invariant_views,
            outputs,
            output_owners,
            result_words,
            summary,
            observe,
            private,
        });
        covered.extend(proof.sites.iter().copied());
    }
    Ok(result)
}

// Only an exact current six-word view/packet identity grants a move. Existing
// retirement proves the source owner home is dead on this successful exit.
// Each source token is used once, even when multiple results alias its buffer.
fn select_exit_owner_transfers(
    outputs: &[(usize, Output)],
    output_owners: &[usize],
    arrays: &[(usize, bool)],
    result_words: usize,
    retiring: &[usize],
) -> Vec<(usize, usize)> {
    let mut selected = Vec::new();
    let mut used = BTreeSet::new();
    for &destination in output_owners {
        let Some((_, from)) = outputs.iter().find(|&&(to, _)| to == destination) else {
            continue;
        };
        let source = match *from {
            Output::Word(at) => arrays.iter().any(|&(base, _)| at == base + 5).then_some(at),
            Output::Stack(at) => {
                arrays
                    .iter()
                    .filter(|a| a.1)
                    .enumerate()
                    .find_map(|(ordinal, &(base, _))| {
                        (at == 1 + result_words + ordinal * 6 + 5).then_some(base + 5)
                    })
            }
        };
        if let Some(source) = source
            && source != destination
            && retiring.contains(&source)
            && !outputs.iter().any(|&(to, _)| to == source)
            && used.insert(source)
        {
            selected.push((destination, source));
        }
    }
    selected
}
#[cfg(test)]
mod exit_owner_tests {
    use super::*;
    #[test]
    fn private_exit_moves_current_packet_owner_after_growth() {
        assert_eq!(
            select_exit_owner_transfers(
                &[(25, Output::Stack(14))],
                &[25],
                &[(0, false), (10, true)],
                8,
                &[15]
            ),
            [(25, 15)]
        );
    }
    #[test]
    fn shared_readonly_exit_moves_only_one_dead_source_token() {
        assert_eq!(
            select_exit_owner_transfers(
                &[(25, Output::Word(5)), (35, Output::Word(5))],
                &[25, 35],
                &[(0, false)],
                0,
                &[5]
            ),
            [(25, 5)]
        );
    }
    #[test]
    fn live_overwritten_or_nonowner_sources_cannot_move() {
        assert!(
            select_exit_owner_transfers(&[(25, Output::Word(5))], &[25], &[(0, false)], 0, &[])
                .is_empty()
        );
        assert!(
            select_exit_owner_transfers(
                &[(25, Output::Word(5)), (5, Output::Word(5))],
                &[25],
                &[(0, false)],
                0,
                &[5]
            )
            .is_empty()
        );
        assert!(
            select_exit_owner_transfers(&[(25, Output::Word(4))], &[25], &[(0, false)], 0, &[4])
                .is_empty()
        );
        assert!(
            select_exit_owner_transfers(&[(25, Output::Stack(2))], &[25], &[(0, true)], 0, &[5])
                .is_empty()
        );
    }
}

fn array_outputs(
    outputs: &mut Vec<(usize, Output)>,
    destination: usize,
    array: usize,
    arrays: &[(usize, bool)],
    result_words: usize,
    invariant_views: &BTreeSet<usize>,
) {
    let (base, writable) = arrays[array];
    let ordinal = arrays[..array].iter().filter(|a| a.1).count();
    for lane in 0..6 {
        let source = if writable && !invariant_views.contains(&array) {
            Output::Stack(1 + result_words + ordinal * 6 + lane)
        } else {
            Output::Word(base + lane)
        };
        outputs.push((destination + lane, source));
    }
}
pub(super) fn observer<M: Module>(
    module: &mut M,
    b: &mut FunctionBuilder<'_>,
    enabled: bool,
) -> Result<Option<FuncRef>, NativeCompileError> {
    if !enabled {
        return Ok(None);
    }
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 3];
    signature.returns = vec![AbiParam::new(types::I64)];
    let id = module
        .declare_function("mech_compact_phase_observe", Linkage::Import, &signature)
        .map_err(backend)?;
    Ok(Some(module.declare_func_in_func(id, b.func)))
}
