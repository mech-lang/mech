//! Existing same-Plan numeric phases through a compact concrete array boundary.
//! The generic source CFG remains the fallback when an actual writable owner
//! is shared. Real unit append growth uses the existing concrete array helper.
use super::{
    arrays::Arrays, immutable_arrays::LiteralViews, layout::Layout, owner_flow::OwnerFlow,
    phase_bodies::PhaseBodies, phase_storage, vector_guard_facts,
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
use cranelift_codegen::ir::{AbiParam, Block, FuncRef, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::NativeProgramFacts;
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
    pub(super) array: usize,
    countdown: usize,
}
impl NoGrowth {
    fn derive(
        prepared: &PreparedNativeProgram,
        facts: &NativeProgramFacts<'_>,
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
            facts,
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
    function: Option<FuncId>,
    array: usize,
    countdown_slot: usize,
    composed: Option<function::Composed>,
    refuse_growth: bool,
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
    refuse_general_vector: bool,
    no_growth: Option<NoGrowthAlternative>,
    inputs: Vec<usize>,
    arrays: Vec<(usize, bool)>,
    fixed_arrays: Vec<usize>,
    entry_bounds: Vec<(usize, u64, bool)>,
    invariant_views: BTreeSet<usize>,
    outputs: Vec<(usize, Output)>,
    output_owners: Vec<usize>,
    result_words: usize,
    packet_words: (usize, usize),
    summary: serde_json::Value,
    observe: bool,
    private: Option<PrivatePhase>,
    relative_error_origins: bool,
}
impl Phase {
    #[cfg(feature = "early-region-emission")]
    pub(super) fn word_effects(
        &self,
        owners: &OwnerFlow,
    ) -> Vec<(usize, super::word_boundary::Effects)> {
        use super::word_boundary::Effects;
        let effects =
            |site: usize, outputs: &[(usize, Output)], output_owners: &[usize], normal: bool| {
                let mut effect = Effects::default();
                for &word in &self.inputs {
                    effect.read(word);
                }
                for &(base, _) in &self.arrays {
                    for word in base..base + 6 {
                        effect.read(word);
                    }
                }
                for &word in output_owners {
                    if owners.live_at_entry(self.header, word) {
                        effect.read(word);
                    }
                }
                // The normal adapter refreshes writable current views before
                // publishing outputs. Stack results are deliberately opaque here.
                let mut updated = BTreeSet::new();
                if normal {
                    for (array, &(base, writable)) in self.arrays.iter().enumerate() {
                        if writable && !self.invariant_views.contains(&array) {
                            for word in base..base + 6 {
                                effect.write(word);
                                updated.insert(word);
                            }
                        }
                    }
                }
                let parallel = !normal || !self.invariant_views.is_empty();
                if parallel {
                    for (_, source) in outputs {
                        if let Output::Word(source) = *source {
                            effect.read(source);
                        }
                    }
                }
                for &(destination, ref source) in outputs {
                    if !parallel && let Output::Word(source) = *source {
                        effect.read(source);
                    }
                    effect.write(destination);
                    if let Output::Word(source) = *source
                        && !updated.contains(&source)
                    {
                        effect.copy_lane(destination, source);
                    }
                    // In the sequential publication form, a later Word reads the
                    // refreshed home. Do not claim an incoming-value copy for it.
                    if !parallel {
                        updated.insert(destination);
                    }
                }
                if normal && std::env::var_os("MECH_PHASE_OWNER_TRANSFER").is_some() {
                    for (_, source) in self.owner_transfers(owners) {
                        effect.clear(source);
                    }
                }
                for word in owners.retire_bypass(self.header, site, output_owners) {
                    effect.read(word);
                    effect.clear(word);
                }
                (site, effect)
            };
        std::iter::once(effects(self.exit, &self.outputs, &self.output_owners, true))
            .chain(
                self.private
                    .iter()
                    .flat_map(|p| &p.handoffs)
                    .map(|exit| effects(exit.site, &exit.outputs, &exit.owners, false)),
            )
            .collect()
    }
    pub fn checked_traversal_key(&self) -> Option<FuncId> {
        (self.relative_error_origins && !self.refuse_general_vector).then_some(self.function)
    }
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
        self.packet_words
    }

    fn guard_vector_geometry(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        vector: &VectorContract,
        declined: Block,
        vector_guards: &mut Option<&mut Vec<vector_guard_facts::Guard>>,
    ) {
        for &(array, grows) in &vector.guards {
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
            let branch = b.ins().brif(flat, next, &[], declined, &[]);
            if let Some(guards) = vector_guards {
                guards.push(vector_guard_facts::Guard {
                    header: self.header,
                    array,
                    branch,
                });
            }
            b.switch_to_block(next);
        }
    }

    fn growth_admission(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        no_growth: &NoGrowthAlternative,
    ) -> (Value, Value) {
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
        (remaining, enough)
    }

    fn composed_admission(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        composed: &function::Composed,
        remaining: Value,
        consider_record_count: bool,
    ) -> Value {
        let admission = composed.admission;
        let cursor = words.read(b, self.inputs[admission.cursor_word]);
        let cache_length = words.read(b, self.arrays[admission.cache].0 + 1);
        let cost_length = words.read(b, self.arrays[admission.costs].0 + 1);
        let nonnegative = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, cursor, remaining);
        let within = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, cursor, cache_length);
        // Dense codes are an immutable object table. The old
        // record threshold amortizes filling a private cost cache.
        // Keep the actual cursor, length and cost admission checks.
        let work = consider_record_count.then(|| {
            let records = b.ins().isub(cursor, remaining);
            b.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                records,
                admission.minimum_records as i64,
            )
        });
        let readable = b.ins().icmp_imm(
            IntCC::UnsignedGreaterThanOrEqual,
            cost_length,
            admission.minimum_costs as i64,
        );
        let geometry = b.ins().band(nonnegative, within);
        let ready = work.map_or(readable, |work| b.ins().band(work, readable));
        b.ins().band(geometry, ready)
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
        mut vector_guards: Option<&mut Vec<vector_guard_facts::Guard>>,
    ) -> Result<(), NativeCompileError> {
        let observe = observer(module, b, self.observe)?;
        let fallback = b.create_block();
        if self.refuse_general_vector
            && let Some(vector) = &self.vector
        {
            // A geometry refusal enters the same checked continuation as an
            // owner refusal. The view words are available before inspecting
            // owners or preparing the phase packets.
            self.guard_vector_geometry(b, words, &vector.contract, fallback, &mut vector_guards);
        }
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
        let dense = self.no_growth.as_ref().and_then(|growth| {
            (root_entry
                && growth.function.is_none()
                && std::env::var_os("MECH_COMPACT_GUARD_DENSE_BEFORE_PACKET").is_some())
            .then(|| {
                (
                    growth,
                    growth
                        .composed
                        .as_ref()
                        .expect("selected dense composition"),
                )
            })
        });
        if let Some((growth, composed)) = dense {
            let (remaining, enough) = self.growth_admission(b, words, growth);
            let ready = self.composed_admission(b, words, composed, remaining, false);
            let admitted = b.ins().band(enough, ready);
            let packet = b.create_block();
            b.ins().brif(admitted, packet, &[], fallback, &[]);
            b.switch_to_block(packet);
        }
        let (input_words, output_words) = self.packet_words();
        let input = packets.input_for_call(input_words, output_words)?;
        for (at, &slot) in self.inputs.iter().enumerate() {
            let value = words.read(b, slot);
            b.ins().stack_store(value, input, offset(at)?);
        }
        if !self
            .private
            .as_ref()
            .is_some_and(PrivatePhase::compact_packet)
        {
            let writable = self.arrays.iter().filter(|a| a.1).count();
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
        }
        let prior = self
            .output_owners
            .iter()
            .filter(|&&slot| owners.live_at_entry(self.header, slot))
            .map(|&slot| words.read(b, slot))
            .collect::<Vec<_>>();
        let input = b.ins().stack_addr(types::I64, input, 0);
        let out = packets.output_address(b, input_words, output_words)?;
        let phase_timer =
            if root_entry && std::env::var_os("MECH_COMPACT_PROFILE_PHASE_CALLS").is_some() {
                let mut signature = module.make_signature();
                signature.params.extend([AbiParam::new(types::I64); 2]);
                let callback = module
                    .declare_function(
                        "mech_compact_profile_phase_call",
                        Linkage::Import,
                        &signature,
                    )
                    .map_err(backend)?;
                let callback = module.declare_func_in_func(callback, b.func);
                let header = b.ins().iconst(types::I64, self.header as i64);
                let enter = b.ins().iconst(types::I64, 0);
                b.ins().call(callback, &[header, enter]);
                Some((callback, header))
            } else {
                None
            };
        let (status, at) = if let Some((_, composed)) = dense {
            let function = module.declare_func_in_func(composed.function, b.func);
            let call = b.ins().call(function, &[storage.heap, input, out]);
            let result = b.inst_results(call);
            (result[0], result[1])
        } else if let Some(no_growth) = &self.no_growth {
            let (remaining, enough) = self.growth_admission(b, words, no_growth);
            let fast = b.create_block();
            let general = b.create_block();
            let returned = b.create_block();
            b.append_block_param(returned, types::I64);
            b.append_block_param(returned, types::I64);
            b.ins().brif(enough, fast, &[], general, &[]);
            let mut routes: Vec<_> = no_growth
                .function
                .map(|function| (fast, function))
                .into_iter()
                .collect();
            if no_growth.refuse_growth {
                // Only the private call packet has been written. The
                // original typed views and their owners are still current.
                // Resume the ordinary checked phase before any visits.
                b.switch_to_block(general);
                b.ins().jump(fallback, &[]);
            } else {
                routes.push((general, self.function));
            }
            if root_entry && let Some(composed) = &no_growth.composed {
                b.switch_to_block(fast);
                let ready = self.composed_admission(
                    b,
                    words,
                    composed,
                    remaining,
                    no_growth.function.is_some(),
                );
                let prepare = b.create_block();
                let sparse = b.create_block();
                b.ins().brif(ready, prepare, &[], sparse, &[]);
                if no_growth.function.is_some() {
                    routes[0].0 = sparse;
                } else {
                    // Only the caller's private packet was initialized.
                    // Refuse before a phase visit or owner transfer.
                    b.switch_to_block(sparse);
                    b.ins().jump(fallback, &[]);
                }
                routes.push((prepare, composed.function));
            } else if no_growth.function.is_none() {
                b.switch_to_block(fast);
                b.ins().jump(fallback, &[]);
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
            // The local general body uses the populated call packet.
            if !self.refuse_general_vector {
                self.guard_vector_geometry(b, words, &vector.contract, general, &mut vector_guards);
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
        if let Some((callback, header)) = phase_timer {
            let returned = b.ins().iconst(types::I64, 1);
            b.ins().call(callback, &[header, returned]);
        }
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
                        Output::Stack(at) => packets.read_output(b, at),
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
        // Shared bodies use a phase-relative failure location. Restore the
        // caller's exact site here, after the status branch.
        let at = if self.relative_error_origins {
            b.ins().iadd_imm(at, self.header as i64)
        } else {
            at
        };
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
                let value = packets.read_output(b, 1 + self.result_words + ordinal * 6 + lane)?;
                words.write(b, base + lane, value);
            }
        }
        if self.invariant_views.is_empty() {
            for (slot, source) in &self.outputs {
                let value = match *source {
                    Output::Stack(at) => packets.read_output(b, at)?,
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
                    Output::Stack(at) => packets.read_output(b, at),
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
    share_phase_bodies: bool,
    pool: Option<&super::compile_jobs::Pool>,
) -> Result<Vec<Phase>, NativeCompileError> {
    // Body sharing chooses earlier call targets as each definition is published.
    // Keep that choice in serial order; independent bodies can use the pool.
    // The inliner captures legalized IR on the coordinator. With a named
    // inlining scope, other function families can still compile in workers.
    let inlining = std::env::var_os("MECH_COMPACT_INLINE_SINGLE_CALLS").is_some();
    let inline_symbols = std::env::var("MECH_COMPACT_INLINE_SYMBOLS").ok();
    let pool = pool.filter(|_| !share_phase_bodies && (!inlining || inline_symbols.is_some()));
    let compare_vector_ir =
        !observe && std::env::var_os("MECH_COMPACT_REUSE_EQUIVALENT_VECTOR_IR").is_some();
    let skip_refused_general_ssa = !observe
        && !compare_vector_ir
        && std::env::var_os("MECH_COMPACT_SKIP_REFUSED_GENERAL_SSA").is_some()
        && function::refused_general_ssa_unobserved();
    // Selected LLVM exports read the verified IR on the coordinator. Keep all
    // variants of those phases there while other phases can use the workers.
    let exported_phases = std::env::var_os("MECH_COMPACT_DUMP_LLVM_DIRECTORY")
        .and_then(|_| std::env::var("MECH_COMPACT_DUMP_LLVM_SYMBOLS").ok())
        .map(|symbols| {
            symbols
                .split(',')
                .filter_map(|symbol| {
                    symbol
                        .strip_prefix("mech_compact_numeric_")?
                        .split('_')
                        .next()?
                        .parse::<usize>()
                        .ok()
                })
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let facts = prepared.plan().facts();
    let closed = &prepared.metadata().closed_tails;
    let mut result = vec![];
    let mut dense_tables = cfg_cleanup::composed_reads::DenseTables::default();
    let mut shared_bodies =
        PhaseBodies::new(std::env::var_os("MECH_COMPACT_SHARE_PHASE_RELOCATIONS").is_some());
    let mut refuse_general = std::env::var("MECH_COMPACT_REFUSE_GENERAL_VECTORS")
        .ok()
        .map(|headers| {
            headers
                .split(',')
                .map(|header| header.parse::<usize>().map_err(backend))
                .collect::<Result<BTreeSet<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    if observe && !refuse_general.is_empty() {
        return Err(backend(
            "geometry refusal requires the unobserved compact entry",
        ));
    }
    let fixed_packets = std::env::var_os("MECH_COMPACT_FIXED_PHASE_PACKET");
    let selected_outputs = std::env::var_os("MECH_COMPACT_SELECT_PHASE_OUTPUT_WORDS");
    let mut candidates = Vec::new();
    for (id, region) in closed.regions.iter().enumerate() {
        let Some(proof) = region.numeric.as_ref() else {
            continue;
        };
        let private = if std::env::var_os("MECH_PRIVATE_NESTED_PHASES").is_some() {
            let compact_packet = fixed_packets.as_deref().is_some_and(|scope| {
                scope != "refused" || refuse_general.contains(&(region.tail.header() as usize))
            });
            PrivatePhase::derive(prepared, region, layout, compact_packet)?
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
        let pool = pool.filter(|_| {
            if exported_phases.contains(&header) {
                return false;
            }
            !inlining
                || !inline_symbols.as_deref().is_some_and(|symbols| {
                    let family = format!("mech_compact_numeric_{header}");
                    symbols.split(',').any(|symbol| {
                        symbol
                            .strip_prefix(&family)
                            .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('_'))
                    })
                })
        });
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
        let compact_packet = private.as_ref().is_some_and(PrivatePhase::compact_packet);
        let result_words = numeric_minimal::output_words(proof, compact_packet)?.len();
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
                    if !compact_packet {
                        at += 4 + usize::from(proof.capacity(*array).is_some());
                    }
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
        let (input_words, output_words) = if compact_packet {
            (inputs.len(), 1 + result_words)
        } else {
            (
                inputs.len() + writable + fixed_arrays.len(),
                1 + result_words + writable * 6,
            )
        };
        let fixed_count = fixed_arrays.len();
        let output_words = output_words.max(private.as_ref().map_or(0, PrivatePhase::packet_words));
        // Each synchronous alternative returns to this same reader. Retain
        // the existing packet offsets while selecting only the normal-return
        // words the reader can consume. Failure and handoff have separate layouts.
        let selected_output = selected_outputs
            .as_deref()
            .is_some_and(|scope| scope != "non-private" || private.is_none())
            .then(|| {
                let mut selected = outputs
                    .iter()
                    .filter_map(|(_, source)| match *source {
                        Output::Stack(at) => Some(at),
                        Output::Word(_) => None,
                    })
                    .collect::<BTreeSet<_>>();
                for (ordinal, (array, _)) in arrays
                    .iter()
                    .enumerate()
                    .filter(|(_, view)| view.1)
                    .enumerate()
                {
                    if !invariant_views.contains(&array) {
                        let first = 1 + result_words + ordinal * 6;
                        selected.extend(first..first + 6);
                    }
                }
                selected
            });
        let relative_error_origins = share_phase_bodies
            && !observe
            && private
                .as_ref()
                .is_none_or(|phase| phase.handoffs.is_empty());
        let mut bodies = relative_error_origins.then_some(&mut shared_bodies);
        // A refused general geometry uses the checked source continuation.
        // Once a vector contract is available this declaration has no callers.
        // Keep body-sharing definitions intact because another phase can use
        // their published identity. Diagnostics may also request the old body.
        let omit_general = refuse_general.contains(&header)
            && !share_phase_bodies
            && std::env::var_os("MECH_COMPACT_KEEP_REFUSED_PHASE_CODE").is_none();
        let vector_fast = std::env::var_os("MECH_PHASE_VECTOR_FAST").is_some();
        let literal_views = || -> BTreeMap<_, _> {
            arrays
                .iter()
                .enumerate()
                .filter_map(|(array, &(base, writable))| {
                    if writable || proof.arrays[array].appends {
                        return None;
                    }
                    literals.literal(base).map(|literal| (array, literal))
                })
                .collect()
        };
        let refuse_growth = !observe
            && !share_phase_bodies
            && std::env::var_os("MECH_COMPACT_PROFILE_PHASE_CALLS").is_none()
            && std::env::var_os("MECH_COMPACT_REFUSE_PHASE_GROWTH").is_some();
        let unobserved_growth = refuse_growth
            && prepared.metadata().parallel.regions.is_empty()
            && function::refused_general_ssa_unobserved();
        let dense_only =
            unobserved_growth && std::env::var_os("MECH_COMPACT_DENSE_NO_GROWTH_ONLY").is_some();
        let mut chosen = None;
        if unobserved_growth
            && !omit_general
            && private.is_none()
            && std::env::var_os("MECH_WHOLE_PHASE_CAPACITY").is_some()
            && std::env::var_os("MECH_COMPACT_PREPARE_DENSE_FIRST").is_some()
            && let Some(pool) = pool
            && let Some(contract) = NoGrowth::derive(prepared, &facts, region)
        {
            let literals = literal_views();
            if !literals.is_empty() {
                // Keep speculative declarations, definitions and memoized data
                // IDs together. An unadmitted body leaves the compiler state intact.
                let mut recorded = pool.record(module)?;
                let mut tables = dense_tables.clone();
                let general = function::declare_refused_general(&mut recorded, header)?;
                let fast = function::prepare(
                    &mut recorded,
                    prepared,
                    id,
                    observe,
                    input_words,
                    output_words,
                    fixed_count,
                    selected_output.as_ref(),
                    false,
                    None,
                    None,
                    Some(&contract),
                    &literals,
                    &mut tables,
                    None,
                    "_no_growth",
                );
                if let Ok(fast) = fast
                    && fast.has_dense_composition()
                    && fast
                        .entry_bounds()
                        .iter()
                        .any(|&(array, _, row)| array == contract.array && row)
                {
                    recorded.replay(module)?;
                    dense_tables = tables;
                    chosen = Some((general, contract, fast));
                }
            }
        }
        let source_admission = if skip_refused_general_ssa && omit_general && vector_fast {
            private
                .as_ref()
                .and_then(|private| {
                    function::refused_general_bounds(CodePlan::Program(prepared), region, private)
                })
                .and_then(|bounds| {
                    VectorContract::additional(proof, &bounds).map(|contract| (bounds, contract))
                })
        } else {
            None
        };
        // A refused geometry can use the checked source continuation
        // directly. Admit a conservative current-view vector contract without
        // known length floors and take those floors from the one body we emit.
        let vector_only = if source_admission.is_none()
            && skip_refused_general_ssa
            && omit_general
            && vector_fast
            && private.is_none()
            && std::env::var_os("MECH_COMPACT_PREPARE_REFUSED_VECTOR_ONLY").is_some()
        {
            VectorContract::additional(proof, &[]).map(|contract| (Vec::new(), contract))
        } else {
            None
        };
        let use_vector_bounds = vector_only.is_some();
        let (ready, general_declaration, mut entry_bounds, vector_contract) =
            if let Some((general, _, fast)) = &chosen {
                (None, *general, fast.entry_bounds().to_vec(), None)
            } else if let Some((bounds, contract)) = source_admission.or(vector_only) {
                let declaration = function::declare_refused_general(module, header)?;
                (None, declaration, bounds, Some(contract))
            } else {
                let ready = function::prepare_general(
                    module,
                    prepared,
                    id,
                    observe,
                    input_words,
                    output_words,
                    fixed_count,
                    selected_output.as_ref(),
                    false,
                    None,
                    private.as_ref(),
                    None,
                    &BTreeMap::new(),
                    &mut dense_tables,
                    bodies.as_deref_mut(),
                    "",
                    pool.is_some() || omit_general,
                )?;
                let declaration = ready.function();
                let bounds = ready.entry_bounds().to_vec();
                let contract = if vector_fast {
                    VectorContract::additional(proof, &bounds)
                } else {
                    None
                };
                (Some(ready), declaration, bounds, contract)
            };
        let pack_input = compact_packet
            && refuse_general.contains(&header)
            && std::env::var_os("MECH_COMPACT_PACK_PHASE_INPUTS").is_some();
        let (general, fast) = if chosen.is_some() {
            (None, None)
        } else if omit_general && let Some(contract) = vector_contract.as_ref() {
            let skipped_general_ssa = ready.is_none();
            // The source proof or the verified general IR supplied the same
            // bounds. No general code or cleanup result is needed here.
            drop(ready);
            let fast = function::prepare(
                module,
                prepared,
                id,
                observe,
                input_words,
                output_words,
                fixed_count,
                selected_output.as_ref(),
                pack_input,
                Some(contract),
                private.as_ref(),
                None,
                &BTreeMap::new(),
                &mut dense_tables,
                None,
                "_vector",
            )?;
            if use_vector_bounds {
                entry_bounds = fast.entry_bounds().to_vec();
            } else if skipped_general_ssa && fast.entry_bounds() != entry_bounds.as_slice() {
                return Err(backend(
                    "refused phase source bounds differ from its vector body",
                ));
            }
            let fast = if let Some(pool) = pool {
                fast.start(pool)?.finish(module)?
            } else {
                fast.define(module, None)?
            };
            (None, Some(fast))
        } else if let (Some(pool), Some(contract)) = (pool, vector_contract.as_ref()) {
            let mut general = ready
                .expect("a compared phase needs its general body")
                .start(pool, compare_vector_ir && !pack_input)?;
            let mut recorded = pool.record(module)?;
            let fast = function::prepare(
                &mut recorded,
                prepared,
                id,
                observe,
                input_words,
                output_words,
                fixed_count,
                selected_output.as_ref(),
                pack_input,
                Some(contract),
                private.as_ref(),
                None,
                &BTreeMap::new(),
                &mut dense_tables,
                bodies.as_deref_mut(),
                "_vector",
            )
            .and_then(|ready| general.start_vector(ready, pool));
            // Complete and publish the earlier function before replaying ANY
            // later declaration/data/function event, including an error.
            let (general, context) = general.finish(module)?;
            recorded.replay(module)?;
            let fast = fast?.finish(module, &context)?;
            (Some(general), Some(fast))
        } else {
            let general = ready
                .expect("a retained phase needs its general body")
                .define(module, bodies.as_deref_mut())?;
            let fast = if let Some(contract) = vector_contract.as_ref() {
                Some(function::define(
                    module,
                    prepared,
                    id,
                    observe,
                    input_words,
                    output_words,
                    fixed_count,
                    selected_output.as_ref(),
                    pack_input,
                    Some(contract),
                    private.as_ref(),
                    None,
                    &BTreeMap::new(),
                    &mut dense_tables,
                    bodies.as_deref_mut(),
                    "_vector",
                )?)
            } else {
                None
            };
            (Some(general), fast)
        };
        let function = general
            .as_ref()
            .map_or(general_declaration, |body| body.function);
        let mut summary = general.as_ref().map_or_else(
            || {
                serde_json::json!({
                    "header": header, "exit": region.tail.exit(),
                    "symbol": format!("mech_compact_numeric_{header}"),
                    "phase_entry_bounds": entry_bounds,
                    "code_bytes": 0, "stack_bytes": 0,
                    "compilation": if chosen.is_some() {
                        "omitted: admitted dense scorer uses checked continuation"
                    } else {
                        "omitted: geometry uses checked continuation"
                    },
                    "input_words": input_words, "output_words": output_words,
                    "fixed_writable_capacities": fixed_count,
                    "fixed_length_entry_gate": fixed_count != 0,
                    "unique_writable_owner_gate": true,
                })
            },
            |body| body.summary.clone(),
        );
        let mut vector = None;
        let mut packed_input = None;
        if let Some(fast) = fast {
            let contract = vector_contract.unwrap();
            let used = general.as_ref().is_none_or(|body| fast.code != body.code);
            let mut info = fast.summary;
            info["guard_arrays"] = serde_json::json!(contract.guards);
            info["selected_by_guard"] = serde_json::json!(used);
            summary["vector_variant"] = info;
            if used {
                packed_input = fast.packed_input;
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
            if compact_packet {
                summary["private_compact_packet"] = serde_json::json!(true);
            }
        }
        let refuse_general_vector = vector.is_some() && refuse_general.remove(&header);
        if refuse_general_vector {
            summary["general_geometry_uses_checked_continuation"] = serde_json::json!(true);
        }
        let planned = chosen
            .map(|(_, contract, ready)| (contract, Some(ready)))
            .or_else(|| {
                if std::env::var_os("MECH_WHOLE_PHASE_CAPACITY").is_some()
                    && vector.is_none()
                    && private.is_none()
                    && let Some(contract) = NoGrowth::derive(prepared, &facts, region)
                    && entry_bounds
                        .iter()
                        .any(|&(array, _, row)| array == contract.array && row)
                {
                    Some((contract, None))
                } else {
                    None
                }
            });
        let no_growth = if let Some((contract, ready)) = planned {
            let fast = if let Some(ready) = ready {
                ready
            } else {
                function::prepare(
                    module,
                    prepared,
                    id,
                    observe,
                    input_words,
                    output_words,
                    fixed_count,
                    selected_output.as_ref(),
                    false,
                    None,
                    None,
                    Some(&contract),
                    &literal_views(),
                    &mut dense_tables,
                    bodies.as_deref_mut(),
                    "_no_growth",
                )?
            };
            if fast.entry_bounds() != entry_bounds {
                return Err(backend("no-growth variant entry bounds differ"));
            }
            let dense_only = dense_only && fast.has_dense_composition();
            let (function, composed) = if dense_only {
                summary["no_growth_variant"] = serde_json::json!({
                    "symbol": format!("mech_compact_numeric_{header}_no_growth"),
                    "phase_entry_bounds": entry_bounds,
                    "code_bytes": 0,
                    "stack_bytes": 0,
                    "compilation": "omitted: refusals resume the checked continuation",
                });
                (None, fast.into_composition())
            } else {
                let fast = fast.define(module, bodies)?;
                summary["no_growth_variant"] = fast.summary;
                (Some(fast.function), fast.composed)
            };
            if let Some(composed) = &composed {
                summary["composed_variant"] = composed.summary.clone();
            }
            if refuse_growth {
                summary["general_growth_uses_checked_continuation"] = serde_json::json!(true);
            }
            Some(NoGrowthAlternative {
                refuse_growth,
                function,
                array: contract.array,
                composed,
                countdown_slot: *bindings
                    .get(&contract.countdown)
                    .ok_or_else(|| backend("no-growth countdown lacks current binding"))?,
            })
        } else {
            None
        };
        let (inputs, input_words) = if let Some(words) = packed_input {
            // Only the selected vector body can read this layout. Geometry
            // refusal still branches to the source continuation before the call.
            if !refuse_general_vector {
                return Err(backend("packed phase inputs require one selected body"));
            }
            let compact = words.iter().map(|&word| inputs[word]).collect::<Vec<_>>();
            summary["private_packed_input_words"] = serde_json::json!(compact.len());
            (compact, words.len())
        } else {
            (inputs, input_words)
        };
        result.push(Phase {
            header,
            exit: region.tail.exit() as usize,
            function,
            vector,
            refuse_general_vector,
            no_growth,
            inputs,
            arrays,
            fixed_arrays,
            entry_bounds,
            invariant_views,
            outputs,
            output_owners,
            result_words,
            packet_words: (input_words, output_words),
            summary,
            observe,
            private,
            relative_error_origins,
        });
        covered.extend(proof.sites.iter().copied());
    }
    if !refuse_general.is_empty() {
        return Err(backend(format!(
            "geometry refusal needs a selected vector phase for {refuse_general:?}"
        )));
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
