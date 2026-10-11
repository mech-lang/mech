//! Inferred computation facts for the experimental native control IR.
//!
//! Effects describe observable host behavior. Coeffects describe requirements
//! on the calling context. Allocation is recorded separately: implementing a
//! value with physical storage does not by itself make that value impure.
//! This finite, monomorphic analysis is a foundation for region inference, not
//! a lifetime proof or the proposed language-wide effect/coeffect type system.
use super::{Expr, Function, Kind, Op};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Effect {
    EntryInput,
    Output,
    Scheduling,
    Mutation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArenaAccess {
    Read,
    Allocate,
    RetireFrame,
}

/// An active arena capability, with a source layout where one is required.
/// Layouts use the admitted IR's structural kinds, never pointer identity or
/// equal lane counts. Region variables and typed provenance are future work.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArenaRequirement {
    pub layout: Option<Kind>,
    pub access: ArenaAccess,
}

/// A supplied arena shadows the caller's arena. An incompatible requirement
/// inside it cannot be repaired by a matching arena further up the call stack.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArenaConflict {
    pub supplied: Kind,
    pub required: ArenaRequirement,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Coeffects {
    pub arenas: BTreeSet<ArenaRequirement>,
    pub big_integers: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComputationSummary {
    pub effects: BTreeSet<Effect>,
    pub coeffects: Coeffects,
    pub arena_conflicts: BTreeSet<ArenaConflict>,
    /// Includes locally supplied arenas, even when no arena capability escapes.
    pub uses_managed_memory: bool,
    /// A may-allocate fact; this is neither a count nor a capacity bound.
    pub allocates: bool,
    /// Pure bigint operators require recyclable value frames; named mutable
    /// cells alone retain their original entry-owned execution path.
    pub big_integer_values: bool,
}

impl ComputationSummary {
    fn join(&mut self, other: &Self) {
        self.effects.extend(&other.effects);
        self.coeffects.big_integers |= other.coeffects.big_integers;
        self.coeffects
            .arenas
            .extend(other.coeffects.arenas.iter().cloned());
        self.arena_conflicts
            .extend(other.arena_conflicts.iter().cloned());
        self.uses_managed_memory |= other.uses_managed_memory;
        self.allocates |= other.allocates;
        self.big_integer_values |= other.big_integer_values;
    }

    fn require(&mut self, layout: Option<Kind>, access: ArenaAccess) {
        self.uses_managed_memory = true;
        self.coeffects
            .arenas
            .insert(ArenaRequirement { layout, access });
    }

    /// A callback that can run with copied arguments in an independent worker.
    pub fn independent_worker(&self) -> bool {
        self.effects.is_empty()
            && !self.coeffects.big_integers
            && self.coeffects.arenas.is_empty()
            && self.arena_conflicts.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputationAnalysis {
    pub functions: BTreeMap<String, ComputationSummary>,
    pub entry: ComputationSummary,
}

fn callee<'a>(
    name: &str,
    summaries: &'a BTreeMap<String, ComputationSummary>,
) -> Result<&'a ComputationSummary, String> {
    summaries
        .get(name)
        .ok_or_else(|| format!("computation analysis: unknown function {name}"))
}

fn expression(
    value: &Expr,
    summaries: &BTreeMap<String, ComputationSummary>,
) -> Result<ComputationSummary, String> {
    let mut result = ComputationSummary::default();
    for operand in operands(value) {
        result.join(&expression(operand, summaries)?);
    }
    match &value.op {
        Op::Integer(_)
        | Op::Float(_)
        | Op::Boolean(_)
        | Op::Text(_)
        | Op::Type
        | Op::ReadBytes(_)
        | Op::Floats(_)
        | Op::Parameter(_)
        | Op::Local(_)
        | Op::Tuple(_)
        | Op::Field(_, _)
        | Op::Match { .. }
        | Op::Reduce { .. }
        | Op::Sequence { .. } => {}
        Op::Primitive(name, _) => {
            if name == "io/argument-u64" {
                result.effects.insert(Effect::EntryInput);
            } else if name.starts_with("io/") {
                return Err(format!(
                    "computation analysis: undeclared IO primitive {name}"
                ));
            }
        }
        Op::Call(name, _) | Op::Invoke { name, .. } => result.join(callee(name, summaries)?),
        Op::Loop {
            predicate, step, ..
        } => {
            result.join(callee(predicate, summaries)?);
            result.join(callee(step, summaries)?);
        }
        Op::Branch { yes, no, .. } => {
            result.join(callee(yes, summaries)?);
            result.join(callee(no, summaries)?);
        }
        Op::Heap { action, layout, .. } => match action {
            // A null value requires no storage and has no allocation effect.
            0 => {}
            1 | 2 => {
                let layout = layout
                    .clone()
                    .ok_or("computation analysis: managed access needs a layout")?;
                result.require(
                    Some(layout),
                    if *action == 1 {
                        ArenaAccess::Allocate
                    } else {
                        ArenaAccess::Read
                    },
                );
                result.allocates |= *action == 1;
            }
            10..=18 => {
                result.coeffects.big_integers = true;
                result.allocates |= *action == 10;
                if !matches!(action, 15 | 18) {
                    result.effects.insert(Effect::Mutation);
                }
            }
            19..=33 => {
                result.coeffects.big_integers = true;
                result.big_integer_values = true;
                result.allocates |= matches!(action, 19..=25 | 33);
            }
            _ => return Err("computation analysis: unknown managed action".into()),
        },
        Op::Arena {
            capacity,
            layout,
            function,
            ..
        } => {
            let mut body = callee(function, summaries)?.clone();
            if capacity.is_some() {
                // Only the callback executes inside this supplied region. Its
                // capacity and arguments were evaluated in the caller above.
                for required in std::mem::take(&mut body.coeffects.arenas) {
                    if required
                        .layout
                        .as_ref()
                        .is_some_and(|required| required != layout)
                    {
                        body.arena_conflicts.insert(ArenaConflict {
                            supplied: layout.clone(),
                            required,
                        });
                    }
                }
            } else {
                // A frame borrows an existing arena; it cannot supply one.
                body.require(None, ArenaAccess::RetireFrame);
            }
            body.uses_managed_memory = true;
            result.join(&body);
        }
        Op::Print { .. } | Op::Write { .. } => {
            result.effects.insert(Effect::Output);
        }
        Op::Parallel { function, .. } => {
            result.effects.insert(Effect::Scheduling);
            result.join(callee(function, summaries)?);
        }
        Op::Scan { function, .. } => {
            result.effects.insert(Effect::Output);
            result.effects.insert(Effect::Scheduling);
            result.join(callee(function, summaries)?);
        }
    }
    Ok(result)
}

/// Evaluated operands only. Named callbacks are represented by their inferred
/// summaries; they are not expanded recursively into the expression tree.
fn operands(value: &Expr) -> Vec<&Expr> {
    match &value.op {
        Op::Tuple(a)
        | Op::Primitive(_, a)
        | Op::Call(_, a)
        | Op::Invoke { arguments: a, .. }
        | Op::Heap { arguments: a, .. }
        | Op::Print { values: a, .. } => a.iter().collect(),
        Op::Field(e, _) | Op::Loop { initial: e, .. } => vec![e],
        Op::Branch {
            condition, state, ..
        } => vec![condition, state],
        Op::Match {
            scrutinee, arms, ..
        } => std::iter::once(scrutinee.as_ref())
            .chain(arms.iter().flat_map(|(condition, body)| [condition, body]))
            .collect(),
        Op::Reduce {
            start,
            step,
            end,
            body,
            ..
        } => vec![start, step, end, body],
        Op::Arena {
            capacity,
            arguments,
            ..
        } => capacity
            .iter()
            .map(Box::as_ref)
            .chain(std::iter::once(arguments.as_ref()))
            .collect(),
        Op::Sequence { effects, output } => effects
            .iter()
            .chain(std::iter::once(output.as_ref()))
            .collect(),
        Op::Parallel {
            start,
            end,
            grain,
            arguments,
            ..
        } => vec![start, end, grain, arguments],
        Op::Scan {
            count,
            state,
            width,
            buffer,
            ..
        } => vec![count, state, width, buffer],
        Op::Integer(_)
        | Op::Float(_)
        | Op::Boolean(_)
        | Op::Text(_)
        | Op::Type
        | Op::ReadBytes(_)
        | Op::Floats(_)
        | Op::Parameter(_)
        | Op::Local(_)
        | Op::Write { .. } => vec![],
    }
}

/// Infer the least fixed point over the finite set of admitted effects and
/// source layouts. Recursive and mutually recursive calls are not presumed
/// pure merely because a traversal has already visited them.
pub fn infer(
    functions: &BTreeMap<String, Function>,
    effects: &[Expr],
    output: &Expr,
) -> Result<ComputationAnalysis, String> {
    let mut summaries: BTreeMap<_, _> = functions
        .keys()
        .map(|name| (name.clone(), ComputationSummary::default()))
        .collect();
    loop {
        let next = functions
            .iter()
            .map(|(name, function)| Ok((name.clone(), expression(&function.output, &summaries)?)))
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        if next == summaries {
            break;
        }
        summaries = next;
    }
    let mut entry = expression(output, &summaries)?;
    for effect in effects {
        entry.join(&expression(effect, &summaries)?);
    }
    Ok(ComputationAnalysis {
        functions: summaries,
        entry,
    })
}

fn validate_callbacks(value: &Expr, analysis: &ComputationAnalysis) -> Result<(), String> {
    match &value.op {
        Op::Parallel { function, .. } | Op::Scan { function, .. } => {
            let summary = callee(function, &analysis.functions)?;
            if !summary.independent_worker()
                || (matches!(&value.op, Op::Scan { .. }) && summary.uses_managed_memory)
            {
                return Err(format!(
                    "native worker callback {function} requires unavailable effects/resources: {:?}; {:?}; {:?}; scans require no managed storage",
                    summary.effects, summary.coeffects.arenas, summary.arena_conflicts,
                ));
            }
        }
        _ => {}
    }
    for operand in operands(value) {
        validate_callbacks(operand, analysis)?;
    }
    Ok(())
}

pub(super) fn validate(
    analysis: &ComputationAnalysis,
    functions: &BTreeMap<String, Function>,
    effects: &[Expr],
    output: &Expr,
    native_functions: bool,
) -> Result<(), String> {
    if !native_functions
        && analysis
            .functions
            .values()
            .any(|summary| !summary.independent_worker() || summary.uses_managed_memory)
    {
        return Err("native callback must be pure".into());
    }
    for function in functions.values() {
        validate_callbacks(&function.output, analysis)?;
    }
    for effect in effects {
        validate_callbacks(effect, analysis)?;
    }
    validate_callbacks(output, analysis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_sequence::control::{Program, compile};

    const CATALOG: &[&str] = &[
        "memory/null",
        "memory/allocate",
        "memory/read",
        "memory/with-arena",
        "memory/with-frame",
        "heap/null",
        "heap/new-refs",
        "heap/read-refs",
        "io/print",
        "io/argument-u64",
        "io/write",
        "io/write-scan",
        "parallel/reduce-ranges",
        "stats/sum",
        "control/while",
        "control/if",
        "control/select",
        "number/shift-left",
        "bigint/new",
        "bigint/multiply-u64-into",
        "bigint/floor-u64",
        "bigint/floor",
    ];
    const ROW: &str = "<row> := <{value<u64>}>\n";
    const LEAF: &str = "Leaf(n<u64>) = (r<ref>) :=\n  r := memory/allocate({value: n}).\n";

    fn program(source: &str) -> Program {
        compile(source, CATALOG).unwrap()
    }
    fn summary<'a>(program: &'a Program, name: &str) -> &'a ComputationSummary {
        &program.computation.functions[name]
    }

    #[test]
    fn arbitrary_precision_cells_require_an_entry_resource_and_ordered_mutations() {
        let p = program("Scale(x<bigint>) = (y<bigint>) := y := bigint/multiply-u64-into(x, x, 3).\nx := bigint/new(7)\ny := Scale(x)\nio/print(\"{}\\n\", bigint/floor-u64(y, x))\n");
        let s = summary(&p, "Scale");
        assert!(s.coeffects.big_integers);
        assert_eq!(s.effects, BTreeSet::from([Effect::Mutation]));
        assert!(!s.independent_worker());
        rejects(&worker("  x := bigint/new(n)\n  y := bigint/multiply-u64-into(x, x, 2)\n  r := bigint/floor-u64(y, x).", ""), "unavailable effects/resources");
        assert!(compile("x := bigint/new(1)\ny := memory/read(x, <u64>)\n", CATALOG).is_err());
    }
    #[test]
    fn pure_bigint_values_infer_storage_without_mutation_and_cannot_escape_heap() {
        let p = program("Add(x<bigint>) => <u64> := (x + 1) * x\nx := bigint/new(7)\nio/print(\"{}\\n\", Add(x))\n");
        let s = summary(&p,"Add");
        assert!(s.big_integer_values && s.allocates && s.coeffects.big_integers);
        assert!(s.effects.is_empty());
        assert!(!s.independent_worker());
        assert!(program("x := bigint/new(1)\nio/print(\"{}\\n\", bigint/floor-u64(x, x))\n").computation.entry.big_integer_values == false);
        assert!(compile("x := bigint/new(1)\nr := memory/allocate({value: x + 1})\n",CATALOG).unwrap_err().contains("scalar or reference lanes"));
    }
    #[test]
    fn bigint_root_lanes_account_for_backend_byte_packing() {
        let kind = Kind::Tuple(vec![Kind::Bytes(17),Kind::BigInteger,
            Kind::Record(vec![("flag".into(),Kind::Boolean),("value".into(),Kind::BigInteger)])]);
        assert_eq!(super::super::bigint_value_lanes(&kind,true),vec![2,4]);
        assert_eq!(super::super::bigint_value_lanes(&kind,false),vec![17,19]);
    }
    fn entry(source: &str) -> String {
        format!("{source}\nio/print(\"{{}}\\n\", 1)\n")
    }
    fn worker(body: &str, prefix: &str) -> String {
        format!(
            "{prefix}Chunk(start<u64>, end<u64>, n<u64>) = (r<u64>) :=\n{body}\nr := parallel/reduce-ranges(0, 4, 1, Chunk, 0, \"sum\")\nio/print(\"{{}}\\n\", r)\n"
        )
    }
    fn rejects(source: &str, fragment: &str) {
        let error = compile(source, CATALOG).unwrap_err();
        assert!(error.contains(fragment), "{error}");
    }

    #[test]
    fn pure_recursion_and_entry_effects_are_separate() {
        let p = program(
            "Factorial(n<u64>) => <u64>\n  | 0 => 1\n  | k => k * Factorial(k - 1).\nn := io/argument-u64(1)\nr := Factorial(n)\nio/print(\"{}\\n\", r)\n",
        );
        assert_eq!(summary(&p, "Factorial"), &ComputationSummary::default());
        assert_eq!(
            p.computation.entry.effects,
            BTreeSet::from([Effect::EntryInput, Effect::Output])
        );
    }

    #[test]
    fn mutually_recursive_effects_reach_every_caller() {
        let p = program(
            "A(n<u64>) => <u64>\n  | 0 => 1\n  | k => B(k - 1).\nB(n<u64>) = (r<u64>) :=\n  ignored := io/print(\"{}\\n\", n)\n  r := A(n).\nr := A(4)\nio/print(\"{}\\n\", r)\n",
        );
        for name in ["A", "B"] {
            assert_eq!(summary(&p, name).effects, BTreeSet::from([Effect::Output]));
        }
    }

    #[test]
    fn allocation_requires_a_typed_arena_and_is_not_host_io() {
        let p = program(&entry(&format!("{ROW}{LEAF}r := Leaf(7)")));
        let s = summary(&p, "Leaf");
        assert!(s.effects.is_empty());
        assert!(s.allocates && s.uses_managed_memory);
        assert_eq!(
            s.coeffects.arenas,
            BTreeSet::from([ArenaRequirement {
                layout: Some(Kind::Record(vec![("value".into(), Kind::Integer)])),
                access: ArenaAccess::Allocate,
            }])
        );
    }

    #[test]
    fn supplied_arena_discharges_only_its_callback() {
        let p = program(&entry(&format!(
            "{ROW}{LEAF}Test(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  row := memory/read(h, <row>)\n  r := row.value.\nr := memory/with-arena(1, <row>, Test, 7)"
        )));
        assert!(summary(&p, "Test").coeffects.arenas.len() == 2);
        assert!(p.computation.entry.coeffects.arenas.is_empty());
        assert!(p.computation.entry.allocates);
    }

    #[test]
    fn frame_requires_an_existing_arena_even_without_allocations() {
        let p = program(&entry(
            "Identity(n<u64>) = (r<u64>) :=\n  r := n.\nr := memory/with-frame(Identity, 7)",
        ));
        assert_eq!(
            p.computation.entry.coeffects.arenas,
            BTreeSet::from([ArenaRequirement {
                layout: None,
                access: ArenaAccess::RetireFrame,
            }])
        );
    }

    #[test]
    fn matching_outer_arena_supplies_frame_and_transitive_access() {
        let prefix = format!(
            "{ROW}{LEAF}Test(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  row := memory/read(h, <row>)\n  r := row.value.\n"
        );
        let p = program(&worker(
            "  r := memory/with-arena(1, <row>, Framed, n).",
            &format!("{prefix}Framed(n<u64>) = (r<u64>) :=\n  r := memory/with-frame(Test, n).\n"),
        ));
        assert!(summary(&p, "Framed").coeffects.arenas.len() == 3);
        assert!(summary(&p, "Chunk").independent_worker());
        assert!(summary(&p, "Chunk").allocates);
    }

    #[test]
    fn equal_arity_does_not_discharge_another_layout() {
        let source = worker(
            "  r := memory/with-arena(1, <other>, Test, n).",
            &format!(
                "{ROW}<other> := <{{different<u64>}}>\n{LEAF}Test(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  r := 1.\n"
            ),
        );
        rejects(&source, "ArenaRequirement");
    }

    #[test]
    fn outer_arena_cannot_repair_an_incompatible_inner_arena() {
        let prefix = format!(
            "{ROW}<other> := <{{different<u64>}}>\n{LEAF}Test(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  r := 1.\nInner(n<u64>) = (r<u64>) :=\n  r := memory/with-arena(1, <other>, Test, n).\n"
        );
        rejects(
            &worker("  r := memory/with-arena(1, <row>, Inner, n).", &prefix),
            "ArenaConflict",
        );
    }

    #[test]
    fn recursive_allocations_propagate_without_becoming_io() {
        let p = program(&entry(&format!(
            "{ROW}Make(n<u64>) => <ref>\n  | 0 => memory/allocate({{value: 0}})\n  | k => Make(k - 1).\nr := Make(4)"
        )));
        let s = summary(&p, "Make");
        assert!(s.effects.is_empty());
        assert!(s.allocates);
        assert_eq!(s.coeffects.arenas.len(), 1);
        assert!(!s.independent_worker());
    }

    #[test]
    fn private_arena_does_not_discharge_capacity_requirements() {
        let prefix = format!(
            "{ROW}{LEAF}Allocate(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  r := 1.\nIdentity(n<u64>) = (r<u64>) :=\n  r := n.\n"
        );
        rejects(
            &worker(
                "  r := memory/with-arena(Allocate(n), <row>, Identity, n).",
                &prefix,
            ),
            "ArenaRequirement",
        );
    }

    #[test]
    fn private_arena_does_not_discharge_argument_requirements() {
        let prefix = format!(
            "{ROW}{LEAF}Allocate(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  r := 1.\nIdentity(n<u64>) = (r<u64>) :=\n  r := n.\n"
        );
        rejects(
            &worker(
                "  r := memory/with-arena(1, <row>, Identity, Allocate(n)).",
                &prefix,
            ),
            "ArenaRequirement",
        );
    }

    #[test]
    fn workers_cannot_borrow_the_coordinator_arena() {
        let source = format!(
            "{ROW}{LEAF}Chunk(start<u64>, end<u64>, n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  r := 1.\nCoordinator(n<u64>) = (r<u64>) :=\n  r := parallel/reduce-ranges(0, 4, 1, Chunk, n, \"sum\").\nr := memory/with-arena(4, <row>, Coordinator, 0)\nio/print(\"{{}}\\n\", r)\n"
        );
        rejects(&source, "native worker callback Chunk");
    }

    #[test]
    fn unused_transitive_print_is_rejected_in_workers() {
        rejects(
            &worker(
                "  r := Loud(n).",
                "Loud(n<u64>) = (r<u64>) :=\n  ignored := io/print(\"{}\\n\", n)\n  r := n.\n",
            ),
            "Output",
        );
    }

    #[test]
    fn printing_in_an_arena_is_still_observable() {
        rejects(
            &worker(
                "  r := memory/with-arena(1, <row>, Loud, n).",
                &format!(
                    "{ROW}Loud(n<u64>) = (r<u64>) :=\n  ignored := io/print(\"{{}}\\n\", n)\n  r := n.\n"
                ),
            ),
            "Output",
        );
    }

    #[test]
    fn loop_predicate_effect_is_transitive() {
        let prefix = "More(i<u64>, end<u64>) = (r<bool>) :=\n  ignored := io/print(\"{}\\n\", i)\n  r := i < end.\nStep(i<u64>, end<u64>) = (next<u64>, stop<u64>) :=\n  next := i + 1\n  stop := end.\n";
        rejects(
            &worker(
                "  (i, stop) := control/while((start, end), More, Step)\n  r := i.",
                prefix,
            ),
            "Output",
        );
    }

    #[test]
    fn nested_scheduling_is_rejected_transitively() {
        let prefix = "Inner(start<u64>, end<u64>, n<u64>) = (r<u64>) :=\n  r := end - start.\n";
        rejects(
            &worker(
                "  r := parallel/reduce-ranges(start, end, 1, Inner, n, \"sum\").",
                prefix,
            ),
            "Scheduling",
        );
    }

    #[test]
    fn null_is_a_pure_value_without_a_region_requirement() {
        let p = program(&worker(
            "  r := (memory/null() == memory/null() ? | true => 1 | false => 0).",
            "",
        ));
        assert_eq!(summary(&p, "Chunk"), &ComputationSummary::default());
    }

    #[test]
    fn conservative_branch_union_preserves_possible_requirements() {
        rejects(
            &worker(
                "  r := (false ? | true => Allocate(n) | false => 1).",
                &format!("{ROW}{LEAF}Allocate(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  r := 1.\n"),
            ),
            "ArenaRequirement",
        );
    }

    #[test]
    fn comprehension_body_requirements_are_not_lost() {
        rejects(
            &worker(
                "  r := stats/sum([Allocate(i) | i <- start..end]).",
                &format!("{ROW}{LEAF}Allocate(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  r := 1.\n"),
            ),
            "ArenaRequirement",
        );
    }

    #[test]
    fn scans_continue_to_reject_even_locally_supplied_storage() {
        let source = format!(
            "{ROW}Identity(n<u64>) = (r<u64>) :=\n  r := n.\nStep(n<u64>) = (next<u64>, byte<u64>) :=\n  next := memory/with-arena(1, <row>, Identity, n)\n  byte := 65.\nr := io/write-scan(1, 0, Step, 60, \"\\n\", 1024)\n"
        );
        rejects(&source, "scans require no managed storage");
    }

    #[test]
    fn private_arena_and_pure_workers_have_expected_summaries() {
        let prefix = format!(
            "{ROW}{LEAF}Test(n<u64>) = (r<u64>) :=\n  h := Leaf(n)\n  row := memory/read(h, <row>)\n  r := row.value.\n"
        );
        let p = program(&worker(
            "  r := memory/with-arena(1, <row>, Test, n).",
            &prefix,
        ));
        assert!(summary(&p, "Chunk").independent_worker());
        assert!(p.computation.entry.coeffects.arenas.is_empty());
        assert_eq!(
            p.computation.entry.effects,
            BTreeSet::from([Effect::Scheduling, Effect::Output])
        );
    }

    #[test]
    fn missing_callees_fail_closed() {
        let output = Expr {
            kind: Kind::Integer,
            op: Op::Call("Missing".into(), vec![]),
        };
        assert!(
            infer(&BTreeMap::new(), &[], &output)
                .unwrap_err()
                .contains("unknown function Missing")
        );
    }

    #[test]
    fn undeclared_io_and_managed_actions_fail_closed() {
        let output = Expr {
            kind: Kind::Integer,
            op: Op::Primitive("io/unknown".into(), vec![]),
        };
        assert!(infer(&BTreeMap::new(), &[], &output).is_err());
        let output = Expr {
            kind: Kind::Reference,
            op: Op::Heap {
                id: 0,
                action: 99,
                arguments: vec![],
                arity: 0,
                layout: None,
            },
        };
        assert!(infer(&BTreeMap::new(), &[], &output).is_err());
    }
}
