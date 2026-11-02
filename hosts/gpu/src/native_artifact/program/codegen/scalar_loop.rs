//! Register-backed execution of a proved callback-free scalar Tail.
//! Reuses the shared bounded tag graph and aggregate/branch algebra;
//! the owning host still sees the original pause, error and exit frontiers.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::common::word_access::SsaWords;
use crate::native_artifact::codegen::common::{aggregates, terminators};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::codegen::control::{CodegenReport, ScalarLoop};
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::{backend, guard, offset, primitive, set_origin};
use crate::native_artifact::program::codegen::hooks::Hooks;
use crate::native_artifact::program::codegen::tag_threading::{Edge, Target};
use crate::native_artifact::program::codegen::{
    borrowed as borrowed_codegen, prefix as prefix_codegen, tag_threading,
};
use crate::native_artifact::program::prepare::tail::Layout;
use crate::native_artifact::program::prepare::{
    borrowed as program_borrowed_layout, prefix as program_prefix_layout,
    tail as program_tail_layout,
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeScalarAggregates,
};
use std::collections::BTreeSet;

const MAX_WORDS: usize = 192;

fn slots(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    aggregates: &NativeScalarAggregates,
    borrowed: &program_borrowed_layout::Layout,
) -> Option<BTreeSet<usize>> {
    let region = &closed.regions[id];
    if region.map.is_some() {
        return None;
    }
    let mut words = BTreeSet::from_iter(region.words..region.words + 3);
    if let Some(borrow) = region.borrowed {
        let layout = &borrowed.regions[borrow];
        words.extend(layout.ready..layout.end);
    }
    for (site, here) in closed.sites.iter().enumerate() {
        if *here != Some(id) {
            continue;
        }
        if let Some(action) = aggregates.action(site) {
            aggregates::aggregate_words(aggregates, action, &mut words)?;
        } else if let Some((borrow, _)) = borrowed.action(site) {
            if Some(borrow) != region.borrowed {
                return None;
            }
            // View lanes above are physical pointer/length homes, not the
            // original dynamic String handle. Scalar outputs keep their slot.
            if let Some((_, action)) = borrowed.action(site) {
                if code.kind(action.output()).is_some() {
                    words.insert(action.output().get() as usize);
                }
            }
            match borrowed.action(site)?.1 {
                mech_engine::native_artifact::NativeBorrowedStringAction::MatrixElement {
                    index,
                    ..
                }
                | mech_engine::native_artifact::NativeBorrowedStringAction::ScalarSelect {
                    index,
                    ..
                } => {
                    words.insert(index.get() as usize);
                }
                _ => {}
            }
        } else {
            match code.blocks()[site].action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Copy { source, output }) if code.kind(*output).is_some() => {
                    words.extend([source.get() as usize, output.get() as usize]);
                }
                Action::Scalar(Scalar::Primitive {
                    operands, output, ..
                }) => {
                    words.extend(operands.iter().map(|v| v.get() as usize));
                    words.insert(output.get() as usize);
                }
                _ => return None,
            }
        }
        match code.blocks()[site].terminator() {
            Term::TailHeader { .. } | Term::Scalar(Branch::Jump(_) | Branch::Reject) => {}
            Term::Scalar(Branch::Branch { condition, .. }) => {
                words.insert(condition.get() as usize);
            }
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                code.kind(left)?;
                words.extend([left.get() as usize, right.get() as usize]);
            }
            _ => return None,
        }
    }
    (words.len() <= MAX_WORDS).then_some(words)
}

fn scalar_action(b: &mut FunctionBuilder<'_>, words: &Words, action: &Scalar, error: Block) {
    match action {
        Scalar::None => {}
        Scalar::Copy { source, output } => {
            let value = words.read(b, source.get() as usize);
            words.write(b, output.get() as usize, value);
        }
        Scalar::Primitive {
            op,
            operands,
            output,
            ..
        } => {
            let args = operands
                .iter()
                .map(|v| words.read(b, v.get() as usize))
                .collect::<Vec<_>>();
            let value = primitive(b, *op, &args, error);
            words.write(b, output.get() as usize, value);
        }
    }
}

fn prefix_count_action(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    prefix: Option<&program_prefix_layout::Region>,
    facts: Option<prefix_codegen::EntryFacts>,
    site: usize,
    action: &Scalar,
    error: Block,
) -> Result<bool, NativeCompileError> {
    let Some((region, facts)) = prefix.zip(facts) else {
        return Ok(false);
    };
    let count = region.frontier.read().count();
    let comparison = count
        .guards()
        .iter()
        .find(|g| g.comparison() as usize == site);
    if comparison.is_none() && !count.aliases().contains(&(site as u32)) {
        return Ok(false);
    }
    let unfinished = b.create_block();
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins()
        .brif(facts.pending_count, unfinished, &[], ordinary, &[]);
    b.switch_to_block(unfinished);
    if let Some(comparison) = comparison {
        let index = words.read(b, comparison.index().get() as usize);
        let covered = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, index, facts.available);
        guard(b, covered, error, crate::native_artifact::abi::STATUS_ABI);
        let value = b.ins().iconst(types::I64, 0);
        words.write(b, comparison.predicate().get() as usize, value);
    }
    b.ins().jump(joined, &[]);
    b.switch_to_block(ordinary);
    scalar_action(b, words, action, error);
    b.ins().jump(joined, &[]);
    b.switch_to_block(joined);
    Ok(true)
}

pub(in crate::native_artifact) fn emit_entry(
    controls: &CompilationControls,
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    locals: Value,
    code: CodePlan<'_>,
    closed: &Layout,
    aggregates: &NativeScalarAggregates,
    borrowed: &program_borrowed_layout::Layout,
    prefix: Option<&program_prefix_layout::Region>,
    facts: Option<prefix_codegen::EntryFacts>,
    site: usize,
    hooks: &Hooks,
    ordinary_blocks: &[Option<Block>],
    error: Block,
    report: &mut CodegenReport,
) -> Result<(), NativeCompileError> {
    if !controls.enabled(OptimizationPass::ScalarLoops) {
        return Ok(());
    }
    let Some(id) = closed.sites.get(site).copied().flatten() else {
        return Ok(());
    };
    let region = &closed.regions[id];
    if region.tail.header() as usize != site
        || region.map.is_some()
        || !controls.scalar_region_available()
    {
        return Ok(());
    }
    let Some(slots) = slots(code, closed, id, aggregates, borrowed) else {
        return Ok(());
    };
    let Some(graph) = tag_threading::derive_scalar(code, closed, id, aggregates, borrowed) else {
        return Ok(());
    };
    let prefix = prefix.filter(|p| p.tail.header() as usize == site);
    if prefix.is_some() && facts.is_none() {
        return Ok(());
    }
    let setup_start = b.func.dfg.num_insts();
    let ordinary = b.create_block();
    let selected = b.create_block();
    let ready = b
        .ins()
        .load(types::I64, MemFlags::new(), locals, offset(region.words)?);
    let mut ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
    if let Some(borrow) = region.borrowed {
        let ready_view = b.ins().load(
            types::I64,
            MemFlags::new(),
            locals,
            offset(borrowed.regions[borrow].ready)?,
        );
        let ready_view = b.ins().icmp_imm(IntCC::NotEqual, ready_view, 0);
        ready = b.ins().band(ready, ready_view);
    }
    b.ins().brif(ready, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    let slot_count = slots.len();
    let words = Words::new(b, locals, code, slots)?;
    let setup_work = b.func.dfg.num_insts() - setup_start;
    let exit = b.create_block();
    let bailout = b.create_block();
    let failed = b.create_block();
    b.append_block_param(failed, types::I64);
    b.set_cold_block(failed);
    let blocks = graph
        .nodes
        .iter()
        .map(|_| b.create_block())
        .collect::<Vec<_>>();
    let target = |t| match t {
        Target::Node(i) => blocks[i],
        Target::Exit => exit,
    };
    b.ins().jump(blocks[0], &[]);
    let mut maximum_site_work = 0;
    for (ordinal, node) in graph.nodes.iter().enumerate() {
        let index = node.site;
        let fail_at = b.create_block();
        b.append_block_param(fail_at, types::I64);
        b.set_cold_block(fail_at);
        b.switch_to_block(blocks[ordinal]);
        let before = b.func.dfg.num_insts();
        if index == site {
            if let Some(tag) = node.header_tag {
                let tag = b.ins().iconst(types::I64, i64::from(tag));
                words.write(b, graph.carrier_word, tag);
            }
            if let Some((prefix, facts)) = prefix.zip(facts) {
                let inspect = b.create_block();
                let proceed = b.create_block();
                let not_final = b.ins().bxor_imm(facts.final_input, 1);
                let pending = b.ins().band(facts.is_prefix, not_final);
                b.ins().brif(pending, inspect, &[], proceed, &[]);
                b.switch_to_block(inspect);
                let tag = words.read(b, graph.carrier_word);
                let pending = prefix_codegen::pending_at_header(
                    b,
                    facts,
                    prefix,
                    tag,
                    |b, slot| Ok(words.read(b, slot)),
                    fail_at,
                )?;
                b.ins().brif(pending, bailout, &[], proceed, &[]);
                b.switch_to_block(proceed);
            }
            let compute = words.read(b, region.words + 1);
            let comparison = words.read(b, region.words + 2);
            let compute_cost = b.ins().iconst(types::I64, region.compute as i64);
            let comparison_cost = b.ins().iconst(types::I64, region.comparison as i64);
            let a = b
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, compute, compute_cost);
            let c = b.ins().icmp(
                IntCC::UnsignedGreaterThanOrEqual,
                comparison,
                comparison_cost,
            );
            let enough = b.ins().band(a, c);
            let funded = b.create_block();
            b.ins().brif(enough, funded, &[], bailout, &[]);
            b.switch_to_block(funded);
            let compute = b.ins().isub(compute, compute_cost);
            let comparison = b.ins().isub(comparison, comparison_cost);
            words.write(b, region.words + 1, compute);
            words.write(b, region.words + 2, comparison);
        }
        if let Some(action) = aggregates.action(index) {
            aggregates::emit(
                b,
                &mut SsaWords(&words),
                aggregates,
                action,
                fail_at,
                &mut aggregates::CheckedGuards,
            )?;
        } else if let Some((borrow, action)) = borrowed.action(index) {
            // Bounds failures synchronize before the original failure/refund
            // callbacks. Their error edge must not re-spill stale ready words.
            borrowed_codegen::emit_native(
                controls.options(),
                b,
                locals,
                Some(&words),
                Some(frame),
                &borrowed.regions[borrow],
                closed,
                index,
                action,
                hooks,
                error,
            )?;
        } else if let Action::Scalar(action) = code.blocks()[index].action() {
            if !prefix_count_action(b, &words, prefix, facts, index, action, fail_at)? {
                scalar_action(b, &words, action, fail_at);
            }
        } else {
            unreachable!("proved callback-free scalar action");
        }
        match &node.edge {
            Edge::Jump(next) => {
                b.ins().jump(target(*next), &[]);
            }
            Edge::Dispatch { cases, other } => {
                let value = words.read(b, graph.carrier_word);
                for &(tag, next) in cases {
                    let otherwise = b.create_block();
                    let same = b.ins().icmp_imm(IntCC::Equal, value, i64::from(tag));
                    b.ins().brif(same, target(next), &[], otherwise, &[]);
                    b.switch_to_block(otherwise);
                }
                b.ins().jump(target(*other), &[]);
            }
            Edge::Branch { yes, no } => {
                terminators::emit_source_terminator(
                    b,
                    &words,
                    code,
                    code.blocks()[index].terminator(),
                    fail_at,
                    |next| {
                        let source_yes = match code.blocks()[index].terminator() {
                            Term::Scalar(
                                Branch::Branch { yes, .. } | Branch::LiteralBranch { yes, .. },
                            ) => yes,
                            _ => unreachable!(),
                        };
                        target(if next == source_yes { *yes } else { *no })
                    },
                );
            }
            Edge::Reject => terminators::emit_source_terminator(
                b,
                &words,
                code,
                code.blocks()[index].terminator(),
                fail_at,
                |_| unreachable!(),
            ),
        }
        b.switch_to_block(fail_at);
        let status = b.block_params(fail_at)[0];
        set_origin(b, frame, index as u64);
        b.ins().jump(failed, &[status.into()]);
        let work = b.func.dfg.num_insts() - before;
        maximum_site_work = maximum_site_work.max(work);
        if work > program_tail_layout::OVERHEAD as usize {
            return Err(backend(
                "scalar SSA site exceeds original closed-body envelope",
            ));
        }
    }
    let boundary_start = b.func.dfg.num_insts();
    b.switch_to_block(bailout);
    words.spill_all(b, locals)?;
    b.ins().jump(ordinary, &[]);
    b.switch_to_block(exit);
    words.spill_all(b, locals)?;
    b.ins()
        .jump(ordinary_blocks[region.tail.exit() as usize].unwrap(), &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    words.spill_all(b, locals)?;
    b.ins().jump(error, &[status.into()]);
    let fixed_work = setup_work + b.func.dfg.num_insts() - boundary_start;
    if fixed_work > program_tail_layout::FIXED_OVERHEAD as usize {
        return Err(backend(
            "scalar SSA boundaries exceed original fixed envelope",
        ));
    }
    controls.emitted_scalar_region();
    report.scalar_loops.push(ScalarLoop {
        header: site,
        carrier_word: graph.carrier_word,
        register_words: slot_count,
        source_sites: closed
            .sites
            .iter()
            .filter(|&&here| here == Some(id))
            .count(),
        product_nodes: graph.nodes.len(),
        variants: graph.variants.clone(),
        folded_branches: graph
            .nodes
            .iter()
            .filter(|n| n.folded_branch)
            .map(|n| n.site)
            .collect(),
        known_latches: graph
            .nodes
            .iter()
            .filter(|n| n.known_latch)
            .map(|n| n.site)
            .collect(),
        maximum_source_instructions: maximum_site_work,
        fixed_template_instructions: fixed_work,
    });
    b.switch_to_block(ordinary);
    Ok(())
}
