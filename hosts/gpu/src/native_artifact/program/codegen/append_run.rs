//! Specialize a checked, capacity-proved append recurrence. Every side exit
//! restarts the same source visit before a write, with its current loop state.
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{Block, BlockArg, Function, Inst, InstBuilder, Value, types};
use std::collections::BTreeMap;

#[path = "packet_reloads.rs"]
mod packet_reloads;
#[path = "append_run_trace.rs"]
mod trace;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Word {
    Value(Value),
    Imm(i64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Compare {
    cc: IntCC,
    left: Word,
    right: Word,
}

impl Compare {
    fn new(cc: IntCC, left: Word, right: Word) -> Self {
        Self { cc, left, right }
    }

    fn sense(self, positive: bool) -> Self {
        Self {
            cc: if positive {
                self.cc
            } else {
                self.cc.complement()
            },
            ..self
        }
    }

    fn values(self) -> impl Iterator<Item = Value> {
        [self.left, self.right]
            .into_iter()
            .filter_map(|word| match word {
                Word::Value(value) => Some(value),
                Word::Imm(_) => None,
            })
    }

    fn constant(self) -> Option<bool> {
        let (left, right) = match (self.left, self.right) {
            (Word::Imm(left), Word::Imm(right)) => (left, right),
            _ if self.left == self.right => (0, 0),
            _ => return None,
        };
        Some(match self.cc {
            IntCC::Equal => left == right,
            IntCC::NotEqual => left != right,
            IntCC::UnsignedLessThan => (left as u64) < (right as u64),
            IntCC::UnsignedGreaterThanOrEqual => (left as u64) >= (right as u64),
            _ => return None,
        })
    }

    fn emit(self, cursor: &mut FuncCursor<'_>, mapped: impl Fn(Value) -> Value) -> Value {
        let mut operand = |word| match word {
            Word::Value(value) => mapped(value),
            Word::Imm(n) => cursor.ins().iconst(types::I64, n),
        };
        let left = operand(self.left);
        let right = operand(self.right);
        cursor.ins().icmp(self.cc, left, right)
    }
}

enum Step {
    Copy { inst: Inst, args: Vec<Value> },
    Guard(Compare),
}

#[derive(Clone, Copy)]
struct Window {
    count: usize,
    cursor: usize,
    length: usize,
    limit: Word,
    cache_guard: Compare,
    previous_guard: Compare,
}

impl Window {
    fn admit(
        self,
        cursor: &mut FuncCursor<'_>,
        seed: &[Value],
        descriptors: &BTreeMap<Value, Value>,
    ) -> Value {
        let limit = match self.limit {
            Word::Value(value) => mapped(descriptors, value),
            Word::Imm(value) => cursor.ins().iconst(types::I64, value),
        };
        let remaining = seed[self.count];
        let position = seed[self.cursor];
        let length = seed[self.length];
        let enough = cursor
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, remaining, position);
        let in_view = cursor
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, position, limit);
        let nonempty = cursor.ins().icmp_imm(IntCC::NotEqual, length, 0);
        let largest = cursor.ins().bnot(remaining);
        let no_wrap = cursor
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, length, largest);
        let readable = cursor.ins().band(enough, in_view);
        let previous = cursor.ins().band(nonempty, no_wrap);
        cursor.ins().band(readable, previous)
    }
}

struct Trace {
    steps: Vec<Step>,
    selector: Value,
    stored: Value,
    previous: Value,
    append_term: Inst,
    continuation: Block,
    seed_checks: Vec<Compare>,
    next: Vec<Value>,
    window: Option<Window>,
}

fn block(function: &mut Function) -> Block {
    let block = function.dfg.make_block();
    function.layout.append_block(block);
    block
}

fn mapped(values: &BTreeMap<Value, Value>, value: Value) -> Value {
    values.get(&value).copied().unwrap_or(value)
}

fn emit(
    f: &mut Function,
    header: Block,
    old_params: &[Value],
    trace: Trace,
    reloads: Option<packet_reloads::Reloads>,
) -> usize {
    let hot = block(f);
    let mut state = Vec::new();
    let mut incoming = BTreeMap::new();
    let mut values = BTreeMap::new();
    // After a real append the result may supply the same word in several
    // original positions, such as row length and column count.
    for (&old, &next) in old_params.iter().zip(&trace.next) {
        let param = *incoming.entry(next).or_insert_with(|| {
            state.push(next);
            f.dfg.append_block_param(hot, types::I64)
        });
        values.insert(old, param);
    }
    let carried = f.dfg.append_block_param(hot, types::I64);
    let current = old_params
        .iter()
        .map(|v| BlockArg::Value(values[v]))
        .collect::<Vec<_>>();

    let seed = block(f);
    let mut cursor = FuncCursor::new(f).at_inst(trace.append_term);
    let empty = cursor.ins().icmp_imm(IntCC::Equal, trace.selector, 0);
    cursor
        .func
        .dfg
        .replace(trace.append_term)
        .brif(empty, seed, &[], trace.continuation, &[]);
    cursor.goto_bottom(seed);
    for check in trace.seed_checks {
        let valid = check.emit(&mut cursor, |value| value);
        let success = block(cursor.func);
        cursor
            .ins()
            .brif(valid, success, &[], trace.continuation, &[]);
        cursor.goto_bottom(success);
    }
    if let Some(reloads) = &reloads {
        values.extend(reloads.for_run(&mut cursor));
    }
    let window = trace.window;
    if let Some(window) = window {
        let valid = window.admit(&mut cursor, &trace.next, &values);
        let success = block(cursor.func);
        cursor
            .ins()
            .brif(valid, success, &[], trace.continuation, &[]);
        cursor.goto_bottom(success);
    }
    let args = state
        .iter()
        .copied()
        .chain([trace.stored])
        .map(BlockArg::Value)
        .collect::<Vec<_>>();
    cursor.ins().jump(hot, &args);

    let (fallback, fallback_args) = if let Some(reloads) = reloads {
        let restore = block(cursor.func);
        cursor.goto_bottom(restore);
        let mut args = current;
        args.extend(reloads.emit(&mut cursor).into_iter().map(BlockArg::Value));
        cursor.ins().jump(header, &args);
        (restore, Vec::new())
    } else {
        (header, current)
    };
    cursor.goto_bottom(hot);
    for step in trace.steps {
        match step {
            Step::Copy { inst, args } => {
                let result = cursor.func.dfg.inst_results(inst).first().copied();
                if result == Some(trace.previous) {
                    values.insert(trace.previous, carried);
                    continue;
                }
                let copy = cursor.func.dfg.clone_inst(inst);
                for (value, source) in cursor.func.dfg.inst_args_mut(copy).iter_mut().zip(args) {
                    *value = mapped(&values, source);
                }
                cursor.insert_inst(copy);
                if let Some(result) = result {
                    let value = cursor.func.dfg.first_result(copy);
                    values.insert(result, value);
                    if result == trace.selector {
                        let empty = cursor.ins().icmp_imm(IntCC::Equal, value, 0);
                        let success = block(cursor.func);
                        cursor
                            .ins()
                            .brif(empty, success, &[], fallback, &fallback_args);
                        cursor.goto_bottom(success);
                        let zero = cursor.ins().iconst(types::I64, 0);
                        values.insert(result, zero);
                    }
                }
            }
            Step::Guard(check) => {
                if window.is_some_and(|proof| {
                    check == proof.cache_guard || check == proof.previous_guard
                }) {
                    continue;
                }
                let condition = check.emit(&mut cursor, |v| mapped(&values, v));
                let success = block(cursor.func);
                cursor
                    .ins()
                    .brif(condition, success, &[], fallback, &fallback_args);
                cursor.goto_bottom(success);
            }
        }
    }
    let args = state
        .iter()
        .copied()
        .chain([trace.stored])
        .map(|v| BlockArg::Value(mapped(&values, v)))
        .collect::<Vec<_>>();
    cursor.ins().jump(hot, &args);
    state.len() + 1
}

pub(in crate::native_artifact) fn optimize(
    context: &mut Context,
    header: Block,
    packet: Option<(usize, bool)>,
) -> Result<(usize, usize, usize), &'static str> {
    context.compute_cfg();
    context.compute_domtree();
    let trace = trace::capture(context, header)?;
    let old_params = context.func.dfg.block_params(header).to_vec();
    let reloads = packet.and_then(|(words, split_shared)| {
        packet_reloads::split(context, header, &trace, words, split_shared)
    });
    let count = reloads.as_ref().map_or(0, packet_reloads::Reloads::count);
    let entry_proved_guards = if trace.window.is_some() { 2 } else { 0 };
    let words = emit(&mut context.func, header, &old_params, trace, reloads);
    context.compute_cfg();
    context.compute_domtree();
    Ok((words, count, entry_proved_guards))
}
