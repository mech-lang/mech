//! Shared checked scalar growth and backing for the compact native entry.

use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    AbiParam, Block, FuncRef, Function, Inst, InstBuilder, MemFlags, StackSlot, StackSlotData,
    StackSlotKind, Value, types,
};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncOrDataId, Linkage, Module};
use std::{cell::RefCell, collections::BTreeMap};

pub(super) struct GrowthSite {
    pub(super) choose_growth: Inst,
    pub(super) enough: Value,
    pub(super) growth: Inst,
    pub(super) capacity: Value,
    pub(super) needed: Value,
    pub(super) shift: u32,
    pub(super) done: Block,
    pub(super) error: Block,
}

pub(super) struct BackingSite {
    pub(super) call: Inst,
    pub(super) fits: Option<Inst>,
    pub(super) capacity: Inst,
    pub(super) fallback: Block,
    pub(super) consume: bool,
}

struct Site {
    growth: GrowthSite,
    backing: Option<BackingSite>,
}

struct SharedBacking {
    provider: FuncRef,
    byte_borrowing: Option<FuncRef>,
    output: Option<StackSlot>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BackingResults {
    Packet,
    Preserve,
    Registers,
    ByteRegisters,
}

pub(super) struct CapacityGrowth {
    provider: FuncRef,
    output: Option<StackSlot>,
    backing: Option<SharedBacking>,
    sites: RefCell<Vec<Site>>,
}

impl CapacityGrowth {
    pub(super) fn declare<M: Module>(
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        preserve_registers: bool,
        combined_backing: Option<BackingResults>,
    ) -> Result<Self, NativeCompileError> {
        let preserve_registers =
            preserve_registers && module.isa().default_call_conv() == CallConv::AppleAarch64;
        let name = if preserve_registers {
            "mech_compact_expand_capacity_preserving"
        } else {
            "mech_compact_expand_capacity"
        };
        let params = 3 + usize::from(preserve_registers);
        let returns = usize::from(!preserve_registers);
        let provider = provider(
            module,
            b,
            name,
            params,
            returns,
            preserve_registers,
            |_, helper| {
                let args = helper.block_params(helper.current_block().unwrap());
                let [capacity, needed, shift] = [args[0], args[1], args[2]];
                let output = args.get(3).copied();
                let capacity = expanded_capacity(helper, capacity, needed, shift);
                if let Some(output) = output {
                    helper.ins().store(MemFlags::new(), capacity, output, 0);
                    helper.ins().return_(&[]);
                } else {
                    helper.ins().return_(&[capacity]);
                }
                Ok(())
            },
        )?;
        let output = preserve_registers.then(|| stack_packet(b, 8));
        let backing = combined_backing
            .map(|results| SharedBacking::declare(module, b, results))
            .transpose()?;
        Ok(Self {
            provider,
            output,
            backing,
            sites: RefCell::default(),
        })
    }

    pub(super) fn record(&self, growth: GrowthSite) {
        self.sites.borrow_mut().push(Site {
            growth,
            backing: None,
        });
    }

    pub(super) fn record_backing(
        &self,
        b: &FunctionBuilder<'_>,
        source_block: Block,
        minimum: Value,
        capacity: Value,
        stride: usize,
        error: Block,
        backing: BackingSite,
    ) {
        let Some(shared) = &self.backing else {
            return;
        };
        if shared.byte_borrowing.is_some() && stride != 1 {
            return;
        }
        if let Some(site) = self.sites.borrow_mut().last_mut()
            && site.growth.done == source_block
            && site.growth.capacity == capacity
            && site.growth.error == error
            && site.growth.shift == stride.trailing_zeros()
            && b.block_params(source_block).first() == Some(&minimum)
        {
            site.backing = Some(backing);
        }
    }

    pub(super) fn replace(self, function: &mut Function) -> (usize, usize) {
        let sites = self.sites.into_inner();
        let mut minimum_uses = BTreeMap::new();
        for site in &sites {
            if site.backing.is_some()
                && let &[minimum] = function.dfg.block_params(site.growth.done)
            {
                minimum_uses.insert(minimum, Vec::new());
            }
        }
        if !minimum_uses.is_empty() {
            for block in function.layout.blocks() {
                for inst in function.layout.block_insts(block) {
                    for value in function.dfg.inst_values(inst) {
                        if let Some(uses) =
                            minimum_uses.get_mut(&function.dfg.resolve_aliases(value))
                        {
                            uses.push(inst);
                        }
                    }
                }
            }
        }
        let mut standalone = 0;
        let mut combined = 0;
        for Site {
            growth: site,
            backing,
        } in sites
        {
            let &[minimum] = function.dfg.block_params(site.done) else {
                continue;
            };
            if function.dfg.block_params(site.error).len() != 1 {
                continue;
            }
            if let Some((shared, backing)) = self.backing.as_ref().zip(backing)
                && function.dfg.block_params(backing.fallback).is_empty()
                // Bypassing the sufficient-capacity route must skip only
                // the original branch to the in-place or writable path.
                && function.layout.first_inst(site.done) == function.layout.last_inst(site.done)
                && minimum_uses[&minimum].iter().all(|&inst| {
                    inst == backing.call || inst == backing.capacity || Some(inst) == backing.fits
                })
            {
                shared.replace(function, site, backing);
                combined += 1;
                continue;
            }
            let mut cursor = FuncCursor::new(function).at_inst(site.growth);
            let shift = cursor.ins().iconst(types::I64, i64::from(site.shift));
            let capacity = if let Some(slot) = self.output {
                let output = cursor.ins().stack_addr(types::I64, slot, 0);
                cursor
                    .ins()
                    .call(self.provider, &[site.capacity, site.needed, shift, output]);
                cursor.ins().stack_load(types::I64, slot, 0)
            } else {
                let call = cursor
                    .ins()
                    .call(self.provider, &[site.capacity, site.needed, shift]);
                cursor.func.dfg.first_result(call)
            };
            let overflow = cursor.ins().iconst(types::I64, 1);
            cursor.func.dfg.replace(site.growth).brif(
                capacity,
                site.done,
                &[capacity.into()],
                site.error,
                &[overflow.into()],
            );
            standalone += 1;
        }
        (standalone, combined)
    }
}

fn stack_packet(b: &mut FunctionBuilder<'_>, bytes: u32) -> StackSlot {
    b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, bytes, 3))
}

pub(super) fn provider<M: Module>(
    module: &mut M,
    b: &mut FunctionBuilder<'_>,
    name: &str,
    params: usize,
    returns: usize,
    preserve: bool,
    define: impl FnOnce(&mut M, &mut FunctionBuilder<'_>) -> Result<(), NativeCompileError>,
) -> Result<FuncRef, NativeCompileError> {
    let id = if let Some(FuncOrDataId::Func(id)) = module.get_name(name) {
        id
    } else {
        let mut signature = module.make_signature();
        signature.params.resize(params, AbiParam::new(types::I64));
        signature.returns.resize(returns, AbiParam::new(types::I64));
        if preserve {
            signature.call_conv = CallConv::PreserveAll;
        }
        let id = module
            .declare_function(name, Linkage::Local, &signature)
            .map_err(backend)?;
        let mut context = module.make_context();
        context.func.signature = signature;
        let mut builder_context = FunctionBuilderContext::new();
        let mut helper = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let entry = helper.create_block();
        helper.append_block_params_for_function_params(entry);
        helper.switch_to_block(entry);
        define(module, &mut helper)?;
        helper.seal_all_blocks();
        helper.finalize();
        module.define_function(id, &mut context).map_err(backend)?;
        id
    };
    Ok(module.declare_func_in_func(id, b.func))
}

/// The caller established needed > capacity. Zero represents overflow.
fn expanded_capacity(
    b: &mut FunctionBuilder<'_>,
    capacity: Value,
    needed: Value,
    shift: Value,
) -> Value {
    let first = b.ins().iconst(types::I64, 64);
    let first = b.ins().ushr(first, shift);
    let middle_start = b.ins().iconst(types::I64, 4096);
    let middle_start = b.ins().ushr(middle_start, shift);
    let middle_end = b.ins().iconst(types::I64, 131072);
    let middle_end = b.ins().ushr(middle_end, shift);
    let below = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, capacity, middle_start);
    let above = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThan, capacity, middle_end);
    let double = b.ins().bor(below, above);
    let (twice, overflow) = b.ins().uadd_overflow(capacity, capacity);
    // The 1.5x rate is selected only for capacities at most 131072.
    let triple = b.ins().imul_imm(capacity, 3);
    let rounded = b.ins().iadd_imm(triple, 1);
    let half = b.ins().ushr_imm(rounded, 1);
    let growth = b.ins().select(double, twice, half);
    let growth = b.ins().select(capacity, growth, first);
    let result = b.ins().umax(growth, needed);
    let failed = b.ins().band(double, overflow);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().select(failed, zero, result)
}

impl SharedBacking {
    fn declare<M: Module>(
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        results: BackingResults,
    ) -> Result<Self, NativeCompileError> {
        let name = match results {
            BackingResults::Packet => "mech_compact_append_backing",
            BackingResults::Preserve => "mech_compact_append_backing_preserving",
            BackingResults::Registers => "mech_compact_append_backing_registers",
            BackingResults::ByteRegisters => "mech_compact_append_backing_byte_consume",
        };
        let packet = matches!(results, BackingResults::Packet | BackingResults::Preserve);
        let byte = results == BackingResults::ByteRegisters;
        let preserve = results == BackingResults::Preserve;
        let provider = Self::define(module, b, name, packet, preserve, byte.then_some(3))?;
        let byte_borrowing = byte
            .then(|| {
                Self::define(
                    module,
                    b,
                    "mech_compact_append_backing_byte_copy",
                    false,
                    false,
                    Some(2),
                )
            })
            .transpose()?;
        Ok(Self {
            provider,
            byte_borrowing,
            output: packet.then(|| stack_packet(b, 24)),
        })
    }

    fn define<M: Module>(
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        name: &str,
        packet: bool,
        preserve: bool,
        byte_layout: Option<i64>,
    ) -> Result<FuncRef, NativeCompileError> {
        let ordinary_params = 7 - usize::from(byte_layout.is_some());
        let params = ordinary_params + usize::from(packet);
        let returns = if packet { 0 } else { 3 };
        provider(
            module,
            b,
            name,
            params,
            returns,
            preserve,
            |module, helper| {
                let args = helper.block_params(helper.current_block().unwrap());
                let &[heap, data, length, capacity, owner, needed] = &args[..6] else {
                    unreachable!("six private append backing view arguments")
                };
                let output = args.get(ordinary_params).copied();
                let layout = if let Some(layout) = byte_layout {
                    helper.ins().iconst(types::I64, layout)
                } else {
                    args[6]
                };
                let Some(FuncOrDataId::Func(writable)) =
                    module.get_name("mech_compact_writable_scalar_backing")
                else {
                    return Err(backend(
                        "scalar append requires registered writable backing",
                    ));
                };
                let writable = module.declare_func_in_func(writable, helper.func);
                let allocate = helper.create_block();
                let expand = helper.create_block();
                let overflow = helper.create_block();
                helper.append_block_param(allocate, types::I64);
                let enough = helper
                    .ins()
                    .icmp(IntCC::UnsignedLessThanOrEqual, needed, capacity);
                helper
                    .ins()
                    .brif(enough, allocate, &[capacity.into()], expand, &[]);
                helper.switch_to_block(expand);
                let width = helper.ins().ushr_imm(layout, 1);
                let shift = helper.ins().ctz(width);
                let grown = expanded_capacity(helper, capacity, needed, shift);
                helper
                    .ins()
                    .brif(grown, allocate, &[grown.into()], overflow, &[]);
                helper.switch_to_block(overflow);
                let zero = helper.ins().iconst(types::I64, 0);
                let status = helper.ins().iconst(types::I64, 1);
                if let Some(output) = output {
                    helper.ins().store(MemFlags::new(), zero, output, 0);
                    helper.ins().store(MemFlags::new(), status, output, 8);
                    helper.ins().return_(&[]);
                } else {
                    helper.ins().return_(&[zero, status, zero]);
                }
                helper.switch_to_block(allocate);
                let grown = helper.block_params(allocate)[0];
                let call = helper.ins().call(
                    writable,
                    &[heap, data, length, capacity, owner, grown, layout],
                );
                let &[data, owner_or_status] = helper.inst_results(call) else {
                    unreachable!("two backing words")
                };
                if let Some(output) = output {
                    helper.ins().store(MemFlags::new(), data, output, 0);
                    helper
                        .ins()
                        .store(MemFlags::new(), owner_or_status, output, 8);
                    helper.ins().store(MemFlags::new(), grown, output, 16);
                    helper.ins().return_(&[]);
                } else {
                    helper.ins().return_(&[data, owner_or_status, grown]);
                }
                Ok(())
            },
        )
    }

    fn replace(&self, f: &mut Function, growth: GrowthSite, backing: BackingSite) {
        // Growth never reuses the original view. It enters the shared slow
        // path before inspecting the owner, including on arithmetic failure.
        f.dfg.replace(growth.choose_growth).brif(
            growth.enough,
            growth.done,
            &[growth.capacity.into()],
            backing.fallback,
            &[],
        );
        let mut args = f.dfg.inst_args(backing.call).to_vec();
        args[5] = growth.needed;
        let provider = if let Some(copy) = self.byte_borrowing {
            args.pop();
            if backing.consume { self.provider } else { copy }
        } else {
            self.provider
        };
        let mut cursor = FuncCursor::new(f).at_inst(backing.call);
        let (data, owner_or_status, capacity) = if let Some(slot) = self.output {
            let output = cursor.ins().stack_addr(types::I64, slot, 0);
            args.push(output);
            cursor.ins().call(provider, &args);
            let data = cursor.ins().stack_load(types::I64, slot, 0);
            let owner_or_status = cursor.ins().stack_load(types::I64, slot, 8);
            // Read the capacity only after the original success branch.
            cursor
                .func
                .dfg
                .replace(backing.capacity)
                .stack_load(types::I64, slot, 16);
            (data, owner_or_status, None)
        } else {
            let call = cursor.ins().call(provider, &args);
            let &[data, owner_or_status, capacity] = cursor.func.dfg.inst_results(call) else {
                unreachable!("three private return words")
            };
            (data, owner_or_status, Some(capacity))
        };
        let old = cursor.func.dfg.inst_results(backing.call).to_vec();
        cursor.func.dfg.detach_inst_results(backing.call);
        for (old, new) in old.into_iter().zip([data, owner_or_status]) {
            cursor.func.dfg.change_to_alias(old, new);
        }
        cursor.func.layout.remove_inst(backing.call);
        if let Some(capacity) = capacity {
            let old = cursor.func.dfg.first_result(backing.capacity);
            cursor.func.dfg.detach_inst_results(backing.capacity);
            cursor.func.dfg.change_to_alias(old, capacity);
            cursor.func.layout.remove_inst(backing.capacity);
        }
    }
}
