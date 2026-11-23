//! Dense live transport inside the existing admitted numeric loan.
//! Canonical ownership, current funding and every cold exit stay unchanged.
use super::{hooks::Hooks, readonly_numeric};
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::{backend, offset};
use crate::native_artifact::program::prepare::tail::Region;
use cranelift_codegen::ir::{
    Block, FuncRef, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value, types,
};
use cranelift_frontend::FunctionBuilder;
use std::collections::BTreeSet;

enum InputSource {
    Canonical(usize),
    Argument(usize),
    Scalar(usize),
    Zero,
}

pub(super) struct Recipe {
    sources: Vec<InputSource>,
    inputs: Vec<usize>,
    outputs: Vec<usize>,
}
impl Recipe {
    /// One existing computation class: a complete nested numeric phase whose
    /// sole writable array appends and is returned directly. Never source IDs.
    pub fn derive(code: CodePlan<'_>, region: &Region) -> Result<Option<Self>, NativeCompileError> {
        let Some(proof) = &region.numeric else {
            return Ok(None);
        };
        if !proof.exits.is_empty()
            || proof
                .phase
                .as_ref()
                .is_none_or(|p| p.loops.len() < 2 || p.checked_children)
            || proof.result.direct_array().is_none()
        {
            return Ok(None);
        }
        let writable = proof
            .arrays
            .iter()
            .filter(|a| a.writable)
            .collect::<Vec<_>>();
        if writable.len() != 1 || !writable[0].appends {
            return Ok(None);
        }
        let shadow = region
            .shadow
            .ok_or_else(|| backend("numeric packet lacks admitted shadow"))?;
        let registered = readonly_numeric::slots(code, proof, region.words, shadow)?;
        let fresh = readonly_numeric::fresh_words(code, proof, shadow, &registered)?;
        let constants = (0..code.constant_count())
            .map(|at| code.constant(at))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter_map(|(value, word)| word.map(|_| value.get() as usize))
            .collect::<BTreeSet<_>>();
        let entry = proof
            .entry_words(region.tail.carrier().get())
            .map_err(backend)?
            .into_iter()
            .map(|word| shadow + word)
            .collect::<BTreeSet<_>>();
        let inputs = registered
            .difference(&fresh)
            .filter(|slot| !constants.contains(slot))
            .filter(|slot| !(shadow..shadow + proof.words).contains(slot) || entry.contains(slot))
            .copied()
            .collect::<Vec<_>>();
        let sources = inputs
            .iter()
            .map(|&slot| {
                if !(shadow..shadow + proof.words).contains(&slot) {
                    return Ok(InputSource::Canonical(slot));
                }
                let relative = slot - shadow;
                if let Some(binding) = proof
                    .scalar_inputs
                    .iter()
                    .chain(&proof.scalar_captures)
                    .find(|b| b.word == relative)
                {
                    return binding
                        .physical
                        .checked_mul(2)
                        .map(InputSource::Scalar)
                        .ok_or_else(|| backend("numeric packet scalar offset overflows"));
                }
                let metadata = proof.arrays.iter().enumerate().any(|(array, _)| {
                    (proof.view(array)..proof.view(array) + 4).contains(&relative)
                        || proof.capacity(array) == Some(relative)
                });
                Ok(if metadata {
                    InputSource::Argument(2 + relative)
                } else {
                    InputSource::Zero
                })
            })
            .collect::<Result<Vec<_>, NativeCompileError>>()?;
        let outputs = readonly_numeric::snapshots::live_words(
            code,
            region,
            0,
            None,
            &proof.boundary_values.exit,
            &registered,
        )?
        .into_iter()
        .collect();
        Ok(Some(Self {
            sources,
            inputs,
            outputs,
        }))
    }
    pub fn input_words(&self) -> usize {
        self.inputs.len()
    }
    pub fn output_words(&self) -> usize {
        self.outputs.len()
    }
    /// Fits inside the ORIGINAL boundary price or outlining declines. The
    /// existing source/funding prices are never raised for this transport.
    pub fn additional_template_bound(&self) -> usize {
        super::numeric_outlined::ADDITIONAL_TEMPLATE_BOUND
            + 2 * (self.inputs.len() + self.outputs.len())
    }
    pub fn words(
        &self,
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        input: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
        home_flags: impl FnMut(usize) -> MemFlags,
    ) -> Result<Words, NativeCompileError> {
        Words::new_with_regions_and_initial(
            b,
            locals,
            code,
            slots,
            |b, slot| {
                self.inputs
                    .iter()
                    .position(|&word| word == slot)
                    .map(|at| {
                        Ok(b.ins()
                            .load(types::I64, MemFlags::new(), input, offset(at)?))
                    })
                    .transpose()
            },
            home_flags,
        )
    }
    pub fn finish(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        output: Value,
    ) -> Result<usize, NativeCompileError> {
        for (at, &slot) in self.outputs.iter().enumerate() {
            let value = words.read(b, slot);
            b.ins().store(MemFlags::new(), value, output, offset(at)?);
        }
        Ok(self.outputs.len())
    }
    pub fn call(
        &self,
        b: &mut FunctionBuilder<'_>,
        function: FuncRef,
        frame: Value,
        locals: Value,
        hooks: &Hooks,
        done: Block,
        ordinary: Block,
        error: Block,
        gate_work: usize,
    ) -> Result<(), NativeCompileError> {
        let before = b.func.dfg.num_insts();
        let mut packet = |count: usize| -> Result<_, NativeCompileError> {
            Ok(b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                u32::try_from(
                    count
                        .checked_mul(8)
                        .ok_or_else(|| backend("numeric packet size overflow"))?,
                )
                .map_err(backend)?,
                3,
            )))
        };
        let input = packet(self.inputs.len())?;
        let output = packet(self.outputs.len())?;
        let bank = self
            .sources
            .iter()
            .any(|source| matches!(source, InputSource::Scalar(_)))
            .then(|| {
                b.ins()
                    .load(types::I64, MemFlags::new(), hooks.arguments, 16)
            });
        for (at, source) in self.sources.iter().enumerate() {
            let value = match *source {
                InputSource::Canonical(slot) => {
                    b.ins()
                        .load(types::I64, MemFlags::new(), locals, offset(slot)?)
                }
                InputSource::Argument(slot) => {
                    b.ins()
                        .load(types::I64, MemFlags::new(), hooks.arguments, offset(slot)?)
                }
                InputSource::Scalar(slot) => {
                    b.ins()
                        .load(types::I64, MemFlags::new(), bank.unwrap(), offset(slot)?)
                }
                InputSource::Zero => b.ins().iconst(types::I64, 0),
            };
            b.ins().stack_store(value, input, offset(at)?);
        }
        let mut arguments = hooks.numeric_outline_arguments(frame, locals).to_vec();
        arguments.push(b.ins().stack_addr(types::I64, input, 0));
        arguments.push(b.ins().stack_addr(types::I64, output, 0));
        let call = b.ins().call(function, &arguments);
        let status = b.inst_results(call)[0];
        let completed = b.inst_results(call)[1];
        let good = b.create_block();
        b.ins().brif(status, error, &[status.into()], good, &[]);
        b.switch_to_block(good);
        let finish = b.create_block();
        b.ins().brif(completed, finish, &[], ordinary, &[]);
        b.switch_to_block(finish);
        // Only a successful normal exit wrote this packet. Denial, cold
        // fallback and failure already synchronized their original homes.
        for (at, &slot) in self.outputs.iter().enumerate() {
            let value = b.ins().stack_load(types::I64, output, offset(at)?);
            b.ins().store(MemFlags::new(), value, locals, offset(slot)?);
        }
        b.ins().jump(done, &[]);
        if gate_work + b.func.dfg.num_insts() - before + 5 > self.additional_template_bound() {
            return Err(backend(
                "numeric packet exceeds its original funded envelope",
            ));
        }
        b.switch_to_block(ordinary);
        Ok(())
    }
}
