//! Native scalar transfers for the proved common-prefix enum view.
use super::*;
use crate::native_artifact::program::prepare::numeric_prefix::Prefix;

pub(super) fn moves(
    shadow: usize,
    code: CodePlan<'_>,
    operation: &Frame,
    recipes: &[FrameWords],
    prefix: &Prefix,
) -> Result<Option<FrameWordMoves>, NativeCompileError> {
    let recipe = |v: u32| {
        recipes
            .iter()
            .find(|f| f.value == v)
            .ok_or_else(|| backend(format!("prefix frame {v} lost its words at {operation:?}")))
    };
    let mut result = FrameWordMoves {
        assignments: Vec::new(),
        parallel: true,
        constants: Vec::new(),
        comparisons: Vec::new(),
    };
    let mut copy = |source: u32, output: u32, count: usize| -> Result<(), NativeCompileError> {
        let from = recipe(source)?;
        let to = recipe(output)?;
        for field in 0..count {
            match (
                from.fields.get(field).copied().flatten(),
                to.fields.get(field).copied().flatten(),
            ) {
                (Some(from), Some(to)) => result.assignments.push((shadow + to, shadow + from)),
                (None, None) => {}
                _ => return Err(backend("prefix scalar field kind differs")),
            }
        }
        Ok(())
    };
    match operation {
        Frame::EnumPack {
            output,
            payload,
            ordinal,
        } if prefix.tag(output.get()).is_some() => {
            let count = *prefix
                .counts
                .get(*ordinal as usize)
                .ok_or_else(|| backend("prefix tag exceeds variants"))?;
            let target = recipe(output.get())?;
            if let Ok(payload) = recipe(payload.get()) {
                if payload.fields.len() != count {
                    return Err(backend("prefix pack payload length differs"));
                }
                copy(payload.value, output.get(), count)?;
            } else if count != 1 || target.fields[0].is_some() || target.arrays[0].is_none() {
                return Err(backend("prefix direct pack lost its exact matrix identity"));
            }
            for field in count..target.fields.len() - 1 {
                let word = target.fields[field]
                    .ok_or_else(|| backend("prefix padding has a managed field"))?;
                result.constants.push((shadow + word, 0));
            }
            result.constants.push((
                shadow + prefix.tag(output.get()).unwrap(),
                u64::from(*ordinal),
            ));
        }
        Frame::Copy { source, output }
            if prefix.tag(source.get()).is_some() && prefix.tag(output.get()).is_some() =>
        {
            let count = recipe(source.get())?.fields.len();
            if recipe(output.get())?.fields.len() != count {
                return Err(backend("prefix enum copy length differs"));
            }
            copy(source.get(), output.get(), count)?;
        }
        Frame::Observe {
            source,
            path,
            observation,
            output,
        } if prefix.tag(source.get()).is_some() => {
            use mech_engine::native_artifact::{
                NativeStructuralObservation as Observation, NativeStructuralStep as Step,
            };
            match (path.as_ref(), observation) {
                ([], Observation::EnumOrdinal { ordinal }) => result.comparisons.push((
                    output.get() as usize,
                    shadow + prefix.tag(source.get()).unwrap(),
                    u64::from(*ordinal),
                )),
                ([Step::EnumPayload { ordinal }], Observation::CanonicalValue) => {
                    let count = *prefix
                        .counts
                        .get(*ordinal as usize)
                        .ok_or_else(|| backend("prefix projection tag exceeds variants"))?;
                    if let Ok(output_recipe) = recipe(output.get()) {
                        if output_recipe.fields.len() != count {
                            return Err(backend("prefix projection length differs"));
                        }
                        copy(source.get(), output.get(), count)?;
                    } else if count != 1 || recipe(source.get())?.arrays[0].is_none() {
                        return Err(backend("prefix direct projection lost its matrix identity"));
                    }
                }
                (
                    [
                        Step::EnumPayload { ordinal },
                        Step::TupleItem { ordinal: field },
                    ],
                    Observation::CanonicalValue | Observation::U64,
                ) => {
                    if *field as usize
                        >= *prefix
                            .counts
                            .get(*ordinal as usize)
                            .ok_or_else(|| backend("prefix field tag exceeds variants"))?
                    {
                        return Err(backend("prefix field exceeds payload"));
                    }
                    let from = recipe(source.get())?
                        .field_slots(*field as usize)
                        .collect::<Vec<_>>();
                    if code.kind(*output).is_some() {
                        if from.len() != 1 {
                            return Err(backend("prefix scalar projection span differs"));
                        }
                        result
                            .assignments
                            .push((output.get() as usize, shadow + from[0]));
                    } else if from.is_empty() {
                        if recipe(source.get())?.arrays[*field as usize].is_none() {
                            return Err(backend(
                                "prefix managed projection lost its array identity",
                            ));
                        }
                    } else {
                        let to = recipe(output.get())?.scalar_slots().collect::<Vec<_>>();
                        if from.len() != to.len() {
                            return Err(backend("prefix managed projection scalar span differs"));
                        }
                        result.assignments.extend(
                            from.into_iter()
                                .zip(to)
                                .map(|(from, to)| (shadow + to, shadow + from)),
                        );
                    }
                }
                _ => return Err(backend("prefix observation lacks exact proof")),
            }
        }
        _ => return Ok(None),
    }
    Ok(Some(result))
}

/// Import only the current original variant. The existing argument word zero
/// carries the scalar-bank pointer, so normalized fields never use it as a
/// second payload transport. Runtime validated these exact current homes.
pub(in crate::native_artifact::program::codegen) fn entry(
    b: &mut FunctionBuilder<'_>,
    bank: Value,
    proof: &Proof,
    error: Block,
) -> Result<std::collections::BTreeMap<usize, Value>, NativeCompileError> {
    use crate::native_artifact::compiler::offset;
    use cranelift_frontend::Switch;
    let Some(variant) = proof.tagged.as_ref().filter(|v| v.prefix.is_some()) else {
        return Ok(std::collections::BTreeMap::new());
    };
    let prefix = variant.prefix.as_ref().unwrap();
    let tag_word = prefix
        .tag(variant.carrier)
        .ok_or_else(|| backend("prefix entry tag missing"))?;
    let bindings = proof
        .scalar_inputs
        .iter()
        .filter(|binding| binding.value == variant.carrier)
        .collect::<Vec<_>>();
    let tag_binding = bindings
        .iter()
        .find(|binding| binding.word == tag_word)
        .ok_or_else(|| backend("prefix entry tag bank binding missing"))?;
    let tag_at = tag_binding
        .physical
        .checked_mul(2)
        .ok_or_else(|| backend("prefix tag offset overflows"))?;
    let tag = b
        .ins()
        .load(types::I64, MemFlags::new(), bank, offset(tag_at)?);
    let join = b.create_block();
    for _ in &bindings {
        b.append_block_param(join, types::I64);
    }
    let invalid = b.create_block();
    b.set_cold_block(invalid);
    let mut switch = Switch::new();
    let mut cases = Vec::new();
    for ordinal in 0..prefix.counts.len() {
        let block = b.create_block();
        switch.set_entry(ordinal as u128, block);
        cases.push((ordinal, block));
    }
    switch.emit(b, tag, invalid);
    b.switch_to_block(invalid);
    let status = b
        .ins()
        .iconst(types::I64, crate::native_artifact::abi::STATUS_ABI as i64);
    b.ins().jump(error, &[status.into()]);
    for (ordinal, block) in cases {
        b.switch_to_block(block);
        let physical = prefix
            .physical(variant.carrier, ordinal as u32)
            .ok_or_else(|| backend("prefix entry variant missing"))?;
        let mut values = Vec::new();
        for binding in &bindings {
            let value = if binding.word == tag_word {
                tag
            } else if let Some(actual) = physical
                .scalars
                .iter()
                .find(|actual| actual.word == binding.word)
            {
                if actual.kind != binding.kind {
                    return Err(backend("prefix entry scalar kind differs"));
                }
                let at = actual
                    .physical
                    .checked_mul(2)
                    .ok_or_else(|| backend("prefix scalar offset overflows"))?;
                b.ins().load(types::I64, MemFlags::new(), bank, offset(at)?)
            } else {
                b.ins().iconst(types::I64, 0)
            };
            values.push(value.into());
        }
        b.ins().jump(join, &values);
    }
    b.switch_to_block(join);
    Ok(bindings
        .into_iter()
        .zip(b.block_params(join).iter().copied())
        .map(|(binding, value)| (binding.word, value))
        .collect())
}
