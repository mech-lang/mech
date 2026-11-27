//! One source-closed generator over caller-admitted private F64 banks.
//! Source frames reuse the compact typed recipes; scalar arithmetic and array
//! addressing use the maintained shared emitters. No body callback participates.
use crate::native_artifact::codegen::common::{
    aggregates, ssa_words::Words, terminators, word_access::SsaWords,
};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::{
    compact::{
        layout::{Layout as TypedLayout, LayoutKind},
        recipes::{self, Recipe},
    },
    prepare::{
        numeric_generators::{
            Region,
            banks::{Frame, Layout as Banks, MatrixView},
        },
        readonly_numeric::{ArrayStorage, Operation},
    },
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard, primitive},
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, Switch};
use mech_engine::native_artifact::{
    NativeControlLoop as Loop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeScalarAggregateAction as Aggregate,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    mem::{offset_of, size_of},
};
#[path = "numeric_generator_arrays.rs"]
mod arrays;
use arrays::{copy, describe, get, require, view, write_view};

pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    code: CodePlan<'_>,
    selected: &super::super::prepare::numeric_generators::funding::Selected,
    locals: Value,
    hooks: &super::hooks::Hooks,
    counter_slot: usize,
    exit: Block,
    error: Block,
) -> Result<usize, NativeCompileError> {
    use crate::native_artifact::program::protocol::GlobalHelper;
    let site = selected.region.header as usize;
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(
        b,
        site,
        GlobalHelper::NumericGeneratorPrepare,
        [zero; 3],
        error,
    );
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let native = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(frame, native, &[], ordinary, &[]);
    b.switch_to_block(native);
    let complete = b.create_block();
    let miss = b.create_block();
    let malformed = b.create_block();
    let settle = b.create_block();
    b.append_block_param(miss, types::I64);
    b.append_block_param(settle, types::I64);
    let before = b.func.dfg.num_insts();
    emit(
        b,
        code,
        &selected.region,
        &selected.banks,
        frame,
        locals,
        complete,
        miss,
        malformed,
    )?;
    let body_ir = b.func.dfg.num_insts() - before;
    if (body_ir as u64)
        .checked_add(128)
        .is_none_or(|n| n > selected.budget.native_per_visit)
    {
        return Err(backend(
            "generator body exceeds its prepaid native instruction envelope",
        ));
    }
    b.switch_to_block(complete);
    b.ins().jump(settle, &[zero.into()]);
    b.switch_to_block(miss);
    let declined = b.ins().iconst(types::I64, 1);
    b.ins().jump(settle, &[declined.into()]);
    b.switch_to_block(malformed);
    let invalid = b.ins().iconst(types::I64, 2);
    b.ins().jump(settle, &[invalid.into()]);
    b.switch_to_block(settle);
    let status = b.block_params(settle)[0];
    hooks.call(
        b,
        site,
        GlobalHelper::NumericGeneratorFinish,
        [frame, status, zero],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let done = b.create_block();
    b.ins().brif(ready, done, &[], ordinary, &[]);
    b.switch_to_block(done);
    b.ins().store(
        MemFlags::new(),
        zero,
        locals,
        crate::native_artifact::compiler::offset(counter_slot)?,
    );
    b.ins().jump(exit, &[]);
    b.switch_to_block(ordinary);
    Ok(body_ir)
}

pub(in crate::native_artifact) fn emit(
    b: &mut FunctionBuilder<'_>,
    code: CodePlan<'_>,
    region: &Region,
    banks: &Banks,
    frame: Value,
    locals: Value,
    complete: Block,
    miss: Block,
    malformed: Block,
) -> Result<(), NativeCompileError> {
    let CodePlan::Program(program) = code else {
        return Err(backend("generator lacks canonical program"));
    };
    let m = program.metadata();
    let plan = &m.plan;
    let selected = region
        .local_values
        .iter()
        .chain(&region.captures)
        .copied()
        .chain(plan.constants().iter().map(|c| c.value().get()))
        .chain(banks.inputs.iter().map(|i| i.value))
        .collect::<BTreeSet<_>>();
    let layout = TypedLayout::derive_scoped(plan, |v| selected.contains(&v))?;
    let scalar_captures = region
        .captures
        .iter()
        .copied()
        .filter(|&v| plan.values()[v as usize].scalar_kind().is_some())
        .collect::<BTreeSet<_>>();
    let constants = (0..code.constant_count())
        .map(|i| code.constant(i))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter_map(|(v, w)| w.map(|w| (v.get() as usize, w)))
        .collect::<BTreeMap<_, _>>();
    let zero = b.ins().iconst(types::I64, 0);
    let words = Words::new_with(b, locals, code, (0..layout.words).collect(), |b, slot| {
        let value = if scalar_captures.contains(&(slot as u32)) {
            b.ins().load(
                types::I64,
                MemFlags::new(),
                locals,
                crate::native_artifact::compiler::offset(slot)?,
            )
        } else if let Some(&word) = constants.get(&slot) {
            b.ins().iconst(types::I64, word as i64)
        } else {
            zero
        };
        Ok(Some(value))
    })?;
    for (field, expected) in [
        (
            offset_of!(Frame, output_elements),
            region.extents[0]
                .checked_mul(region.extents[1])
                .ok_or_else(|| backend("generator output span overflow"))?,
        ),
        (offset_of!(Frame, scratch_elements), banks.elements),
        (offset_of!(Frame, source_count), region.count),
        (offset_of!(Frame, completed), 0),
    ] {
        let actual = get(b, frame, field);
        let good = b.ins().icmp_imm(IntCC::Equal, actual, expected as i64);
        require(b, good, malformed);
    }
    let epoch = get(b, frame, offset_of!(Frame, epoch));
    let issued = get(b, frame, offset_of!(Frame, issued));
    let good = b.ins().icmp(IntCC::Equal, epoch, issued);
    require(b, good, malformed);
    let good = b.ins().icmp_imm(IntCC::NotEqual, epoch, 0);
    require(b, good, malformed);
    let output = get(b, frame, offset_of!(Frame, output));
    let scratch = get(b, frame, offset_of!(Frame, scratch));
    let output_count = region.extents[0] * region.extents[1];
    let out_end = arrays::span(b, output, output_count, malformed);
    let scratch_end = arrays::span(b, scratch, banks.elements, malformed);
    arrays::disjoint(b, output, out_end, scratch, scratch_end, malformed);
    for (index, input) in banks.inputs.iter().enumerate() {
        let at = offset_of!(Frame, inputs) + index * size_of::<MatrixView>();
        let pointer = get(b, frame, at + offset_of!(MatrixView, input));
        let count = input.extents[0] * input.extents[1];
        for (field, expected) in [
            (offset_of!(MatrixView, count), count),
            (offset_of!(MatrixView, rows), input.extents[0]),
            (offset_of!(MatrixView, columns), input.extents[1]),
        ] {
            let actual = get(b, frame, at + field);
            let good = b.ins().icmp_imm(IntCC::Equal, actual, expected as i64);
            require(b, good, malformed);
        }
        let end = arrays::span(b, pointer, count, malformed);
        arrays::disjoint(b, pointer, end, output, out_end, malformed);
        arrays::disjoint(b, pointer, end, scratch, scratch_end, malformed);
        describe(b, &words, &layout, input.value, pointer, input.extents);
    }
    let private = banks
        .banks
        .iter()
        .map(|bank| b.ins().iadd_imm(scratch, (bank.offset * 8) as i64))
        .collect::<Vec<_>>();
    let mut indices = BTreeMap::new();
    let mut counts = BTreeMap::new();
    for (at, &iteration) in banks.loops.iter().enumerate() {
        let expected = get(b, frame, offset_of!(Frame, traversals) + at * 8);
        let good = b.ins().icmp_imm(IntCC::Equal, expected, 0);
        require(b, good, malformed);
        let count = b.declare_var(types::I64);
        b.def_var(count, zero);
        counts.insert(iteration, count);
        if matches!(plan.iterations()[iteration as usize], Loop::Collection(_)) {
            let index = b.declare_var(types::I64);
            b.def_var(index, zero);
            indices.insert(iteration, index);
        }
    }
    let blocks = region
        .sites
        .iter()
        .map(|&s| (s, b.create_block()))
        .collect::<BTreeMap<_, _>>();
    let target = |site| {
        if site == region.exit {
            complete
        } else {
            blocks[&site]
        }
    };
    let bad = b.create_block();
    b.ins().jump(target(region.header), &[]);
    for &site in &region.sites {
        b.switch_to_block(target(site));
        let block = &plan.blocks()[site as usize];
        if let Some(&iteration) = banks
            .loops
            .iter()
            .find(|&&id| plan.iterations()[id as usize].as_ref().header() == site)
        {
            let old = b.use_var(counts[&iteration]);
            let one = b.ins().iconst(types::I64, 1);
            let (next, overflow) = b.ins().uadd_overflow(old, one);
            let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
            require(b, good, bad);
            let maximum = if iteration == region.iteration {
                region.count + 1
            } else if let Some(map) = region
                .maps
                .iter()
                .find(|map| map.region.iteration == iteration)
            {
                region
                    .count
                    .checked_mul(map.count + 1)
                    .ok_or_else(|| backend("generator map traversals overflow"))?
            } else {
                region
                    .count
                    .checked_mul(
                        region
                            .children
                            .iter()
                            .find(|c| c.trips.iteration == iteration)
                            .unwrap()
                            .trips
                            .maximum,
                    )
                    .ok_or_else(|| backend("generator child traversals overflow"))?
            };
            let good = b
                .ins()
                .icmp_imm(IntCC::UnsignedLessThanOrEqual, next, maximum as i64);
            require(b, good, bad);
            b.def_var(counts[&iteration], next);
        }
        let child_operation = region.children.iter().find_map(|child| {
            m.closed_tails.regions[child.region]
                .numeric
                .as_ref()
                .unwrap()
                .actions
                .iter()
                .find(|(at, _)| *at == site as usize)
                .map(|(_, operation)| operation)
        });
        let mut handled = false;
        if let Some(operation) = child_operation {
            match operation {
                Operation::Read { index, output, .. }
                | Operation::CellRead {
                    row: index, output, ..
                } => {
                    let Action::LibraryCall { operands, .. } = block.action() else {
                        return Err(backend("generator read lost source"));
                    };
                    let source = operands[0];
                    let data = view(b, &words, &layout, source.get());
                    let address = match operation {
                        Operation::CellRead { row, column, .. } => {
                            arrays::cell(b, &words, &data, *row, *column, bad)?
                        }
                        _ => arrays::linear(
                            b,
                            &words,
                            &data,
                            *index,
                            plan.values()[source.get() as usize].definition().body(),
                            bad,
                        )?,
                    };
                    let value = b.ins().load(types::I64, MemFlags::new(), address, 0);
                    words.write(b, *output as usize, value);
                    handled = true;
                }
                Operation::Write {
                    index, incoming, ..
                }
                | Operation::CellWrite {
                    row: index,
                    incoming,
                    ..
                } => {
                    let Action::LibraryCall {
                        operands, output, ..
                    } = block.action()
                    else {
                        return Err(backend("generator write lost source"));
                    };
                    let source = operands[0];
                    let data = view(b, &words, &layout, source.get());
                    let address = match operation {
                        Operation::CellWrite { row, column, .. } => {
                            arrays::cell(b, &words, &data, *row, *column, bad)?
                        }
                        _ => arrays::linear(
                            b,
                            &words,
                            &data,
                            *index,
                            plan.values()[source.get() as usize].definition().body(),
                            bad,
                        )?,
                    };
                    let value = words.read(b, *incoming as usize);
                    b.ins().store(MemFlags::new(), value, address, 0);
                    write_view(b, &words, layout.slot(output.get()), &data);
                    handled = true;
                }
                Operation::Length { output, .. } => {
                    let Action::LibraryCall { operands, .. } = block.action() else {
                        return Err(backend("generator length lost source"));
                    };
                    let data = view(b, &words, &layout, operands[0].get());
                    words.write(b, *output as usize, data[1]);
                    handled = true;
                }
                Operation::Frame | Operation::Aggregate | Operation::ArrayCopy => {}
                _ => return Err(backend("generator contains unsupported child operation")),
            }
        }
        if !handled {
            match block.action() {
                Action::Scalar(Scalar::None) | Action::PrepareU64Range { .. } => {}
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                }) => {
                    let values = operands
                        .iter()
                        .map(|v| words.read(b, v.get() as usize))
                        .collect::<Vec<_>>();
                    let value = primitive(b, *op, &values, miss);
                    words.write(b, output.get() as usize, value);
                }
                Action::Initialize { iteration } | Action::Latch { iteration } => match &plan
                    .iterations()[*iteration as usize]
                {
                    Loop::Collection(_) => {
                        let next = if matches!(block.action(), Action::Initialize { .. }) {
                            zero
                        } else {
                            let old = b.use_var(indices[iteration]);
                            b.ins().iadd_imm(old, 1)
                        };
                        b.def_var(indices[iteration], next);
                    }
                    Loop::Tail(tail) => {
                        let source = if matches!(block.action(), Action::Initialize { .. }) {
                            tail.seed()
                        } else {
                            tail.next()
                        };
                        aggregates::emit(
                            b,
                            &mut SsaWords(&words),
                            &layout,
                            &Aggregate::Copy {
                                source,
                                output: tail.carrier(),
                            },
                            miss,
                            &mut aggregates::CheckedGuards,
                        )?;
                        if matches!(block.action(), Action::Initialize { .. }) {
                            for &(owner, array, bank) in &banks.children {
                                if m.closed_tails.regions[owner].iteration != *iteration {
                                    continue;
                                }
                                let array = &m.closed_tails.regions[owner]
                                    .numeric
                                    .as_ref()
                                    .unwrap()
                                    .arrays[array];
                                let field = match array.storage {
                                    ArrayStorage::Carrier { field, .. }
                                    | ArrayStorage::TaggedCarrier { field, .. } => field,
                                    _ => return Err(backend("generator child bank lost carrier")),
                                };
                                clone_field(
                                    b,
                                    &words,
                                    &layout,
                                    tail.carrier().get(),
                                    field,
                                    private[bank],
                                    banks.banks[bank].extents,
                                    miss,
                                    bad,
                                )?;
                            }
                        }
                    }
                    _ => return Err(backend("generator contains non-native loop")),
                },
                Action::CollectionBegin { .. } => {}
                Action::CollectionElement { iteration } => {
                    let Loop::Collection(generator) = &plan.iterations()[*iteration as usize]
                    else {
                        return Err(backend("generator element lost cursor"));
                    };
                    let start = if *iteration == region.iteration {
                        region.start
                    } else {
                        let map = region
                            .maps
                            .iter()
                            .find(|m| m.region.iteration == *iteration)
                            .unwrap();
                        let c = plan
                            .constants()
                            .iter()
                            .find(|c| c.value().get() == map.first)
                            .unwrap();
                        let mech_core::ValueData::U64(start) = c.constant().data() else {
                            return Err(backend("generator map lost fixed start"));
                        };
                        *start
                    };
                    let index = b.use_var(indices[iteration]);
                    let value = b.ins().iadd_imm(index, start as i64);
                    words.write(b, generator.element().unwrap().get() as usize, value);
                }
                Action::CollectionPush { collection, value } => {
                    if *collection == region.collection {
                        let data = view(b, &words, &layout, value.get());
                        let column = b.use_var(indices[&region.iteration]);
                        arrays::push_block(
                            b,
                            output,
                            region.extents,
                            region.block_extents,
                            column,
                            &data,
                            bad,
                        );
                    } else {
                        let map = region
                            .maps
                            .iter()
                            .find(|m| m.region.collection == *collection)
                            .ok_or_else(|| backend("generator push lost scalar map"))?;
                        let index = b.use_var(indices[&map.region.iteration]);
                        let bank = banks.values[&map.output];
                        let valid =
                            b.ins()
                                .icmp_imm(IntCC::UnsignedLessThan, index, map.count as i64);
                        require(b, valid, bad);
                        let address = arrays::address(b, private[bank], index);
                        let value = words.read(b, value.get() as usize);
                        b.ins().store(MemFlags::new(), value, address, 0);
                    }
                }
                Action::CollectionFinish { collection } => {
                    let map = region
                        .maps
                        .iter()
                        .find(|m| m.region.collection == *collection)
                        .ok_or_else(|| backend("generator finish lost scalar map"))?;
                    let index = b.use_var(indices[&map.region.iteration]);
                    let valid = b.ins().icmp_imm(IntCC::Equal, index, map.count as i64);
                    require(b, valid, bad);
                    let bank = banks.values[&map.output];
                    describe(
                        b,
                        &words,
                        &layout,
                        map.region.output,
                        private[bank],
                        [1, map.count],
                    );
                }
                _ => {
                    if let Some(read) = region.maps.iter().flat_map(|m| m.region.reads.iter()).find(|read| read.site == site) {
                        let Action::LibraryCall { operands, .. } = block.action() else { return Err(backend("generator map read lost selected call")) };
                        let data = view(b, &words, &layout, operands[0].get());
                        let address = if read.indices.len() == 2 { arrays::cell(b, &words, &data, read.indices[0], read.indices[1], bad)? }
                            else { arrays::linear(b, &words, &data, read.indices[0], plan.values()[operands[0].get() as usize].definition().body(), bad)? };
                        let value = b.ins().load(types::I64, MemFlags::new(), address, 0); words.write(b, read.output as usize, value);
                    } else if let Some(binary) = m.numeric_binary.at(site as usize) {
                        let bank = banks.values[&binary.output];
                        arrays::binary(b, &words, &layout, m.calls[site as usize].as_ref().unwrap(), binary, private[bank], banks.banks[bank].extents, bad)?;
                    } else if let Action::LibraryCall { template, operands, output, .. } = block.action()
                        && template.primitive() == mech_engine::native_artifact::NativeCollectionPrimitive::MatrixTranspose {
                        let source = view(b, &words, &layout, operands[0].get());
                        let extents = super::super::prepare::numeric_generators::matrix(plan.values()[output.get() as usize].definition().body()).ok_or_else(|| backend("generator transpose lost fixed geometry"))?;
                        if extents[0] == 1 || extents[1] == 1 { describe(b, &words, &layout, output.get(), source[0], extents); }
                        else { let bank = banks.values[&output.get()]; arrays::transpose(b, private[bank], extents, &source); describe(b, &words, &layout, output.get(), private[bank], extents); }
                    } else {
                        let recipe = recipes::derive(plan, &layout, site as usize)?;
                        let Recipe::Aggregate(action) = recipe else { return Err(backend(format!("generator source site {site} has no shared recipe"))) };
                        aggregates::emit(b, &mut SsaWords(&words), &layout, &action, miss, &mut aggregates::CheckedGuards)?;
                    }
                }
            }
        }
        match block.terminator() {
            Term::Header {
                iteration,
                body,
                exit,
            } => {
                let index = b.use_var(indices[&iteration]);
                let count = if iteration == region.iteration {
                    region.count
                } else {
                    region
                        .maps
                        .iter()
                        .find(|m| m.region.iteration == iteration)
                        .unwrap()
                        .count
                };
                let more = b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThan, index, count as i64);
                if exit == region.exit {
                    let finish = b.create_block();
                    b.ins().brif(more, target(body), &[], finish, &[]);
                    b.switch_to_block(finish);
                    for (at, iteration) in banks.loops.iter().enumerate() {
                        let n = b.use_var(counts[iteration]);
                        b.ins().store(
                            MemFlags::new(),
                            n,
                            frame,
                            (offset_of!(Frame, traversals) + at * 8) as i32,
                        );
                    }
                    let n = b.ins().iconst(types::I64, region.count as i64);
                    b.ins().store(
                        MemFlags::new(),
                        n,
                        frame,
                        offset_of!(Frame, completed) as i32,
                    );
                    b.ins().jump(complete, &[]);
                } else {
                    b.ins().brif(more, target(body), &[], target(exit), &[]);
                }
            }
            term => terminators::emit_source_terminator(
                b,
                &words,
                code,
                site as usize,
                Default::default(),
                term,
                miss,
                target,
            ),
        }
    }
    b.switch_to_block(bad);
    let status = b
        .ins()
        .iconst(types::I64, super::numeric_minimal::BOUNDS as i64);
    b.ins().jump(miss, &[status.into()]);
    Ok(())
}

fn clone_field(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    layout: &TypedLayout,
    carrier: u32,
    field: usize,
    destination: Value,
    extents: [u64; 2],
    miss: Block,
    bad: Block,
) -> Result<(), NativeCompileError> {
    let base = layout.slot(carrier);
    let ty = layout.ty(carrier);
    let fields = match &ty.kind {
        LayoutKind::Tagged(variants) => variants
            .iter()
            .enumerate()
            .map(|(tag, variant)| {
                let payload = variant
                    .ty
                    .as_ref()
                    .ok_or_else(|| backend("generator tagged carrier lacks payload"))?;
                let leaves = payload.leaves();
                let (offset, array) = leaves
                    .get(field)
                    .ok_or_else(|| backend("generator prefix array field absent"))?;
                if !matches!(array.kind, LayoutKind::Array { .. }) {
                    return Err(backend("generator prefix field is not an array"));
                }
                Ok((Some(tag), base + variant.offset + offset))
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            let leaves = ty.leaves();
            let (offset, array) = leaves
                .get(field)
                .ok_or_else(|| backend("generator carrier array field absent"))?;
            if !matches!(array.kind, LayoutKind::Array { .. }) {
                return Err(backend("generator carrier field is not an array"));
            }
            vec![(None, base + offset)]
        }
    };
    let done = b.create_block();
    let mut cases = Vec::new();
    let mut switch = Switch::new();
    for (tag, at) in fields {
        let block = b.create_block();
        if let Some(tag) = tag {
            switch.set_entry(tag as u128, block);
        }
        cases.push((at, block));
    }
    if matches!(ty.kind, LayoutKind::Tagged(_)) {
        let tag = words.read(b, base);
        switch.emit(b, tag, bad);
    } else {
        b.ins().jump(cases[0].1, &[]);
    }
    for (at, block) in cases {
        b.switch_to_block(block);
        let old = (0..6).map(|n| words.read(b, at + n)).collect::<Vec<_>>();
        for (actual, expected) in [
            (old[1], extents[0] * extents[1]),
            (old[3], extents[0]),
            (old[4], extents[1]),
        ] {
            let valid = b.ins().icmp_imm(IntCC::Equal, actual, expected as i64);
            guard(b, valid, miss, crate::native_artifact::abi::STATUS_ABI);
        }
        copy(b, old[0], destination, extents[0] * extents[1]);
        let data = arrays::description(b, destination, extents);
        write_view(b, words, at, &data);
        b.ins().jump(done, &[]);
    }
    b.switch_to_block(done);
    Ok(())
}
