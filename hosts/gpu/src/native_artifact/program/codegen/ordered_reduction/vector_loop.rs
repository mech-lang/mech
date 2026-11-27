//! Shared emission of the existing vector loop and its bounded packed-key form.
use super::{Proof, Reduction, Window, cost::Inputs};
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

pub(super) struct Kernel<'a> {
    pub reduction: &'a Reduction,
    pub proof: &'a Proof,
    pub inputs: &'a Inputs,
    pub start: Value,
    pub end: Value,
    pub stop: Value,
    pub best: Value,
    pub count_admitted: bool,
    pub packed_fit: Option<Value>,
}
pub(super) fn packed_enabled(reduction: &Reduction) -> bool {
    cfg!(all(target_arch = "aarch64", target_endian = "little"))
        && std::env::var_os("MECH_ORDERED_PACKED_COST_INDEX").is_some()
        && std::env::var_os("MECH_ORDERED_VECTOR_CARRY_POINTERS").is_none()
        && !Inputs::uses_index(&reduction.cost)
}
struct Lanes {
    costs: Value,
    indices: Option<Value>,
    pointers: Vec<(Window, Value)>,
}
pub(super) struct Selected {
    pub minimum: Value,
    pub item: Value,
    pub pointers: Vec<(Window, Value)>,
}
impl Kernel<'_> {
    fn lanes(&self, b: &mut FunctionBuilder<'_>, packed: bool) -> Lanes {
        let (start, end, best, proof, inputs) =
            (self.start, self.end, self.best, self.proof, self.inputs);
        let costs = if packed {
            super::packed_key::initial(b, best)
        } else {
            b.ins().splat(types::I32X4, best)
        };
        let indices = (!packed).then(|| super::cost::constant(b, [u32::MAX; 4]));
        let lanes = if packed {
            super::packed_key::halfwords(b, [1, 2, 3, 4, 0, 0, 0, 0])
        } else {
            super::cost::constant(b, [0, 1, 2, 3])
        };
        let index_type = if packed { types::I16X8 } else { types::I32X4 };
        let induction_step = std::env::var_os("MECH_ORDERED_VECTOR_CARRY_LANES")
            .is_some()
            .then(|| {
                if packed {
                    super::packed_key::halfwords(b, [4; 8])
                } else {
                    super::cost::constant(b, [4; 4])
                }
            });
        let test_at_latch = std::env::var_os("MECH_ORDERED_VECTOR_LATCH_TEST").is_some();
        let pointer_scope = std::env::var_os("MECH_ORDERED_VECTOR_CARRY_POINTERS");
        let carry_end_pointers = pointer_scope.as_deref() != Some(std::ffi::OsStr::new("body"));
        let pointer_windows = if pointer_scope.is_some() {
            self.reduction.reads.as_slice()
        } else {
            &[]
        };
        let header = b.create_block();
        let body = (!test_at_latch).then(|| b.create_block());
        let finish = b.create_block();
        if test_at_latch {
            b.append_block_param(finish, types::I32X4);
            if !packed {
                b.append_block_param(finish, types::I32X4);
            }
            if carry_end_pointers {
                for _ in pointer_windows {
                    b.append_block_param(finish, types::I64);
                }
            }
        }
        for ty in [types::I64, types::I32X4] {
            b.append_block_param(header, ty);
        }
        if !packed {
            b.append_block_param(header, types::I32X4);
        }
        let mut entry = vec![start.into(), costs.into()];
        if let Some(indices) = indices {
            entry.push(indices.into());
        }
        let state_words = 2 + usize::from(!packed);
        if induction_step.is_some() {
            b.append_block_param(header, index_type);
            let first = b
                .ins()
                .ireduce(if packed { types::I16 } else { types::I32 }, start);
            let first = b.ins().splat(index_type, first);
            entry.push(b.ins().iadd(first, lanes).into());
        }
        let pointer_base = state_words + usize::from(induction_step.is_some());
        for &window in pointer_windows {
            b.append_block_param(header, types::I64);
            entry.push(inputs.pointer(b, proof, window, start).into());
        }
        b.ins().jump(header, &entry);
        b.switch_to_block(header);
        let params = b.block_params(header).to_vec();
        let k = params[0];
        if let Some(body) = body {
            let more = b.ins().icmp(IntCC::UnsignedLessThan, k, end);
            b.ins().brif(more, body, &[], finish, &[]);
            b.switch_to_block(body);
        }
        let index = if induction_step.is_some() {
            params[state_words]
        } else {
            let k32 = b
                .ins()
                .ireduce(if packed { types::I16 } else { types::I32 }, k);
            let index = b.ins().splat(index_type, k32);
            b.ins().iadd(index, lanes)
        };
        let pointers = pointer_windows
            .iter()
            .enumerate()
            .map(|(i, &window)| (window, params[pointer_base + i]))
            .collect::<Vec<_>>();
        let cost = if pointers.is_empty() {
            inputs.emit(b, proof, &self.reduction.cost, k, index)
        } else {
            inputs.emit_with_pointers(b, proof, &self.reduction.cost, k, index, &pointers)
        };
        let (costs, indices) = if packed {
            let key = super::packed_key::key(b, cost, index);
            (b.ins().umin(key, params[1]), None)
        } else {
            let better = b.ins().icmp(IntCC::UnsignedLessThan, cost, params[1]);
            let costs = b.ins().umin(cost, params[1]);
            (costs, Some(b.ins().bitselect(better, index, params[2])))
        };
        let next = b.ins().iadd_imm(k, 4);
        let mut next_group = vec![next.into(), costs.into()];
        if let Some(indices) = indices {
            next_group.push(indices.into());
        }
        if let Some(step) = induction_step {
            // Each lane carries its current ordinal at the selected width.
            // Advancing by four preserves that identity and leaves the
            // admitted window and the scalar remainder bounds unchanged.
            next_group.push(b.ins().iadd(index, step).into());
        }
        let next_pointers = pointers
            .iter()
            .map(|&(window, pointer)| {
                // k advances by four. The immutable admitted window stays
                // the same; only its integer address geometry changes. The
                // terminal address is carried without dereferencing it.
                let stride = 4 * i64::from(proof.arrays[window.array].width);
                b.ins()
                    .iadd_imm(pointer, if window.reverse { -stride } else { stride })
            })
            .collect::<Vec<_>>();
        for &pointer in &next_pointers {
            next_group.push(pointer.into());
        }
        if test_at_latch {
            // This path admits at least 16 elements. Its rounded prefix
            // contains a full first group, and next never exceeds end.
            let more = b.ins().icmp(IntCC::UnsignedLessThan, next, end);
            let mut finished = vec![costs.into()];
            if let Some(indices) = indices {
                finished.push(indices.into());
            }
            if carry_end_pointers {
                for &pointer in &next_pointers {
                    finished.push(pointer.into());
                }
            }
            b.ins().brif(more, header, &next_group, finish, &finished);
        } else {
            b.ins().jump(header, &next_group);
        }
        b.switch_to_block(finish);
        let (final_costs, final_indices) = if test_at_latch {
            let values = b.block_params(finish);
            (values[0], (!packed).then(|| values[1]))
        } else {
            (params[1], (!packed).then(|| params[2]))
        };
        let end_pointers = pointer_windows
            .iter()
            .filter(|_| carry_end_pointers)
            .enumerate()
            .map(|(i, &window)| {
                let pointer = if test_at_latch {
                    b.block_params(finish)[state_words - 1 + i]
                } else {
                    params[pointer_base + i]
                };
                (window, pointer)
            })
            .collect::<Vec<_>>();
        Lanes {
            costs: final_costs,
            indices: final_indices,
            pointers: end_pointers,
        }
    }
    fn selected<F>(
        &self,
        b: &mut FunctionBuilder<'_>,
        packed: bool,
        missing: Value,
        old_item: Option<Value>,
        selection: Option<&std::ffi::OsStr>,
        map: &mut F,
    ) -> Selected
    where
        F: FnMut(&mut FunctionBuilder<'_>, Value, bool) -> Value,
    {
        let lanes = if packed
            && std::env::var_os("MECH_ORDERED_PACKED_ACCUMULATORS").as_deref()
                == Some(std::ffi::OsStr::new("2"))
        {
            let costs = super::packed_loop::emit(
                b,
                self.start,
                self.end,
                self.best,
                std::env::var_os("MECH_ORDERED_VECTOR_CARRY_LANES").is_some(),
                std::env::var_os("MECH_ORDERED_VECTOR_LATCH_TEST").is_some(),
                std::env::var_os("MECH_ORDERED_PACKED_HEAD_GROUP").is_some(),
                |b, k, index| {
                    self.inputs
                        .emit(b, self.proof, &self.reduction.cost, k, index)
                },
            );
            Lanes {
                costs,
                indices: None,
                pointers: Vec::new(),
            }
        } else {
            self.lanes(b, packed)
        };
        let (minimum, item) = if packed {
            let (minimum, winner) = if old_item.is_some()
                && std::env::var_os("MECH_ORDERED_PACKED_WIDE_ITEM").is_some()
            {
                super::packed_key::finish_wide_item(b, lanes.costs)
            } else {
                super::packed_key::finish(b, lanes.costs)
            };
            (minimum, map(b, winner, false))
        } else if selection == Some(std::ffi::OsStr::new("item"))
            && let Some(old_item) = old_item
        {
            super::winner::emit_mapped(
                b,
                lanes.costs,
                lanes.indices.unwrap(),
                self.best,
                missing,
                true,
                old_item,
                map,
            )
        } else {
            let (minimum, winner) = super::winner::emit(
                b,
                lanes.costs,
                lanes.indices.unwrap(),
                self.best,
                missing,
                selection.is_some(),
            );
            (minimum, map(b, winner, false))
        };
        Selected {
            minimum,
            item,
            pointers: lanes.pointers,
        }
    }
    pub fn emit<F>(
        &self,
        b: &mut FunctionBuilder<'_>,
        missing: Value,
        old_item: Option<Value>,
        mut map: F,
    ) -> Selected
    where
        F: FnMut(&mut FunctionBuilder<'_>, Value, bool) -> Value,
    {
        let selection = std::env::var_os("MECH_ORDERED_SKIP_UNCHANGED_VECTOR_WINNER");
        let packed = packed_enabled(self.reduction);
        if !packed {
            return self.selected(b, false, missing, old_item, selection.as_deref(), &mut map);
        }
        // Neither guard changes source state or its exact settled charges. A
        // miss retains the original native vector loop, including wide costs.
        let fits = self.packed_fit.unwrap_or_else(|| {
            super::packed_key::fits(b, self.best, self.stop, self.count_admitted)
        });
        let narrow = b.create_block();
        let wide = b.create_block();
        let done = b.create_block();
        b.append_block_param(done, types::I32);
        b.append_block_param(done, b.func.dfg.value_type(old_item.unwrap_or(missing)));
        b.ins().brif(fits, narrow, &[], wide, &[]);
        b.switch_to_block(wide);
        let regular = self.selected(b, false, missing, old_item, selection.as_deref(), &mut map);
        assert!(regular.pointers.is_empty());
        b.ins()
            .jump(done, &[regular.minimum.into(), regular.item.into()]);
        b.switch_to_block(narrow);
        let packed = self.selected(b, true, missing, old_item, selection.as_deref(), &mut map);
        assert!(packed.pointers.is_empty());
        b.ins()
            .jump(done, &[packed.minimum.into(), packed.item.into()]);
        b.switch_to_block(done);
        Selected {
            minimum: b.block_params(done)[0],
            item: b.block_params(done)[1],
            pointers: Vec::new(),
        }
    }
}
